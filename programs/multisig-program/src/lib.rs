use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

declare_id!("9tX4QfdBjXLUXiV1htgqcLedzygnm87zh58DWENv59f1");

#[program]
pub mod multisig_program {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        owners: Vec<Pubkey>,
        threshold: u64,
        timelock_period: i64,
    ) -> Result<()> {
        // Validate inputs
        require!(!owners.is_empty(), ErrorCode::NoOwnersProvided);
        require!(threshold > 0 && threshold <= owners.len() as u64, ErrorCode::InvalidThreshold);
        require!(timelock_period >= 0, ErrorCode::InvalidTimelockPeriod);
        require!(
            owners.len() <= constants::MAX_OWNERS as usize,
            ErrorCode::InvalidOwnerCount
        );
    
        // Check for duplicate owners
        let mut unique_owners = owners.clone();
        unique_owners.sort();
        unique_owners.dedup();
        require!(unique_owners.len() == owners.len(), ErrorCode::DuplicateOwners);
    
        let multisig = &mut ctx.accounts.multisig;
        multisig.owners = owners;
        multisig.threshold = threshold;
        multisig.transaction_count = 0;
        multisig.timelock_period = timelock_period;
        multisig.is_paused = false;
    
        emit!(MultisigCreated {
            multisig: multisig.key(),
            threshold,
            owner_count: u8::try_from(multisig.owners.len())
                .map_err(|_| error!(ErrorCode::InvalidOwnerCount))?,
        });
    
        Ok(())
    }

    pub fn create_transaction(
        ctx: Context<CreateTransaction>,
        program_id: Pubkey,
        instruction_data: Vec<u8>,
        description: String,
    ) -> Result<()> {
        let is_paused = ctx.accounts.multisig.is_paused;
        require!(!is_paused, ErrorCode::MultisigPaused);
        require!(!instruction_data.is_empty(), ErrorCode::EmptyInstruction);
        require!(description.len() <= 200, ErrorCode::DescriptionTooLong);

        // Combine program_id and instruction data
        let mut full_instruction = Vec::with_capacity(32 + instruction_data.len());
        full_instruction.extend_from_slice(&program_id.to_bytes());
        full_instruction.extend_from_slice(&instruction_data);
        
        let tx = &mut ctx.accounts.transaction;
        tx.creator = ctx.accounts.creator.key();
        tx.instruction = full_instruction;
        tx.description = description;
        tx.approved_by = Vec::new();
        tx.cancelled = false;
        tx.executed = false;
        tx.created_at = Clock::get()?.unix_timestamp;
        tx.expires_at = tx.created_at + ctx.accounts.multisig.timelock_period;
        
        let multisig = &mut ctx.accounts.multisig;
        multisig.transaction_count += 1;
        
        emit!(TransactionCreated {
            tx_id: multisig.transaction_count,
            creator: ctx.accounts.creator.key(),
            created_at: tx.created_at,
        });

        Ok(())
    }

    pub fn approve_transaction(ctx: Context<ApproveTransaction>) -> Result<()> {
        let is_paused = ctx.accounts.multisig.is_paused;
        let signer = ctx.accounts.signer.key();
        
        require!(!is_paused, ErrorCode::MultisigPaused);
        require!(!ctx.accounts.transaction.executed, ErrorCode::TransactionAlreadyExecuted);
        require!(!ctx.accounts.transaction.cancelled, ErrorCode::TransactionCancelled);
        require!(
            ctx.accounts.multisig.owners.contains(&signer),
            ErrorCode::NotAuthorized
        );
        require!(
            Clock::get()?.unix_timestamp <= ctx.accounts.transaction.expires_at,
            ErrorCode::TransactionExpired
        );

        let tx = &mut ctx.accounts.transaction;
        if !tx.approved_by.contains(&signer) {
            tx.approved_by.push(signer);
            
            emit!(TransactionApproved {
                tx_id: ctx.accounts.multisig.transaction_count,
                approver: signer,
            });
        }

        Ok(())
    }

    pub fn revoke_approval(ctx: Context<ApproveTransaction>) -> Result<()> {
        let is_paused = ctx.accounts.multisig.is_paused;
        let signer = ctx.accounts.signer.key();
        
        require!(!is_paused, ErrorCode::MultisigPaused);
        require!(!ctx.accounts.transaction.executed, ErrorCode::TransactionAlreadyExecuted);
        require!(!ctx.accounts.transaction.cancelled, ErrorCode::TransactionCancelled);
        require!(
            ctx.accounts.multisig.owners.contains(&signer),
            ErrorCode::NotAuthorized
        );

        let tx = &mut ctx.accounts.transaction;
        if let Some(pos) = tx.approved_by.iter().position(|x| x == &signer) {
            tx.approved_by.remove(pos);
            
            emit!(ApprovalRevoked {
                tx_id: ctx.accounts.multisig.transaction_count,
                revoker: signer,
            });
        }

        Ok(())
    }

    pub fn execute_transaction(ctx: Context<ExecuteTransaction>) -> Result<()> {
        let multisig = &ctx.accounts.multisig;
        let tx = &mut ctx.accounts.transaction;

        require!(!multisig.is_paused, ErrorCode::MultisigPaused);
        require!(!tx.executed, ErrorCode::TransactionAlreadyExecuted);
        require!(!tx.cancelled, ErrorCode::TransactionCancelled);
        require!(
            tx.approved_by.len() as u64 >= multisig.threshold,
            ErrorCode::NotEnoughApprovals
        );
        require!(
            Clock::get()?.unix_timestamp > tx.created_at + multisig.timelock_period,
            ErrorCode::TimelockNotExpired
        );
        require!(
            Clock::get()?.unix_timestamp <= tx.expires_at,
            ErrorCode::TransactionExpired
        );

        let program_id = Pubkey::new_from_array(tx.instruction[..32].try_into().unwrap());
        let instruction_data = &tx.instruction[32..];
        
        // Convert AccountInfo to AccountMeta
        let account_metas: Vec<AccountMeta> = ctx.remaining_accounts
            .iter()
            .map(|acc| {
                if acc.is_writable {
                    AccountMeta::new(*acc.key, acc.is_signer)
                } else {
                    AccountMeta::new_readonly(*acc.key, acc.is_signer)
                }
            })
            .collect();

        let ix = Instruction::new_with_bytes(
            program_id,
            instruction_data,
            account_metas,
        );

        invoke(
            &ix,
            ctx.remaining_accounts,
        )?;

        tx.executed = true;
        
        emit!(TransactionExecuted {
            tx_id: ctx.accounts.multisig.transaction_count,
            executor: ctx.accounts.signer.key(),
        });

        Ok(())
    }

    pub fn cancel_transaction(ctx: Context<CancelTransaction>) -> Result<()> {
        let is_paused = ctx.accounts.multisig.is_paused;
        let signer = ctx.accounts.signer.key();
        
        require!(!is_paused, ErrorCode::MultisigPaused);
        require!(!ctx.accounts.transaction.executed, ErrorCode::TransactionAlreadyExecuted);
        require!(!ctx.accounts.transaction.cancelled, ErrorCode::TransactionCancelled);
        require!(
            signer == ctx.accounts.transaction.creator || 
            ctx.accounts.multisig.owners.contains(&signer),
            ErrorCode::NotAuthorized
        );

        let tx = &mut ctx.accounts.transaction;
        tx.cancelled = true;
        
        emit!(TransactionCancelled {
            tx_id: ctx.accounts.multisig.transaction_count,
            cancelled_by: signer,
        });

        Ok(())
    }


    pub fn update_owners(
            ctx: Context<UpdateConfig>,
            new_owners: Vec<Pubkey>,
        ) -> Result<()> {
            let is_paused = ctx.accounts.multisig.is_paused;
            
            require!(!is_paused, ErrorCode::MultisigPaused);
            require!(!new_owners.is_empty(), ErrorCode::NoOwnersProvided);
            require!(
                new_owners.len() <= constants::MAX_OWNERS as usize,
                ErrorCode::InvalidOwnerCount
            );
            require!(
                new_owners.len() as u64 >= ctx.accounts.multisig.threshold,
                ErrorCode::InvalidOwnerCount
            );
        
            // Check for duplicate owners
            let mut unique_owners = new_owners.clone();
            unique_owners.sort();
            unique_owners.dedup();
            require!(unique_owners.len() == new_owners.len(), ErrorCode::DuplicateOwners);
        
            let multisig = &mut ctx.accounts.multisig;
            multisig.owners = new_owners;
            
            emit!(OwnersUpdated {
                multisig: multisig.key(),
                new_owner_count: u8::try_from(multisig.owners.len())
                    .map_err(|_| error!(ErrorCode::InvalidOwnerCount))?,
            });
        
            Ok(())
        }

    pub fn update_threshold(
        ctx: Context<UpdateConfig>,
        new_threshold: u64,
    ) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        
        require!(!multisig.is_paused, ErrorCode::MultisigPaused);
        require!(
            new_threshold > 0 && new_threshold <= multisig.owners.len() as u64,
            ErrorCode::InvalidThreshold
        );

        multisig.threshold = new_threshold;
        
        emit!(ThresholdUpdated {
            multisig: multisig.key(),
            new_threshold,
        });

        Ok(())
    }

    pub fn toggle_pause(ctx: Context<UpdateConfig>) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        multisig.is_paused = !multisig.is_paused;
        
        emit!(PauseToggled {
            multisig: multisig.key(),
            is_paused: multisig.is_paused,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = creator,
        space = Multisig::space()
    )]
    pub multisig: Account<'info, Multisig>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateTransaction<'info> {
    #[account(mut)]
    pub multisig: Account<'info, Multisig>,
    #[account(
        init,
        payer = creator,
        space = Transaction::space()
    )]
    pub transaction: Account<'info, Transaction>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ApproveTransaction<'info> {
    pub multisig: Account<'info, Multisig>,
    #[account(mut)]
    pub transaction: Account<'info, Transaction>,
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct ExecuteTransaction<'info> {
    pub multisig: Account<'info, Multisig>,
    #[account(mut)]
    pub transaction: Account<'info, Transaction>,
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct CancelTransaction<'info> {
    pub multisig: Account<'info, Multisig>,
    #[account(mut)]
    pub transaction: Account<'info, Transaction>,
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut)]
    pub multisig: Account<'info, Multisig>,
    pub signer: Signer<'info>,
}

#[account]
#[derive(Default)]
pub struct Multisig {
    pub owners: Vec<Pubkey>,
    pub threshold: u64,
    pub transaction_count: u64,
    pub timelock_period: i64,
    pub is_paused: bool,
}

impl Multisig {
    pub fn space() -> usize {
        8 +  // discriminator
        4 + (32 * 20) +  // owners vector with max 20 owners
        8 +  // threshold
        8 +  // transaction_count
        8 +  // timelock_period
        1    // is_paused
    }
}

#[account]
#[derive(Default)]
pub struct Transaction {
    pub creator: Pubkey,
    pub instruction: Vec<u8>,
    pub description: String,
    pub approved_by: Vec<Pubkey>,
    pub cancelled: bool,
    pub executed: bool,
    pub created_at: i64,
    pub expires_at: i64,
}

impl Transaction {
    pub fn space() -> usize {
        8 +  // discriminator
        32 +  // creator
        4 + 1024 +  // instruction
        4 + 200 +  // description
        4 + (32 * 20) +  // approved_by
        1 +  // cancelled
        1 +  // executed
        8 +  // created_at
        8    // expires_at
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Not enough approvals to execute the transaction.")]
    NotEnoughApprovals,
    #[msg("This transaction has already been executed.")]
    TransactionAlreadyExecuted,
    #[msg("Invalid threshold value.")]
    InvalidThreshold,
    #[msg("No owners provided.")]
    NoOwnersProvided,
    #[msg("Empty instruction.")]
    EmptyInstruction,
    #[msg("Signer is not authorized.")]
    NotAuthorized,
    #[msg("Transaction has expired.")]
    TransactionExpired,
    #[msg("Transaction has been cancelled.")]
    TransactionCancelled,
    #[msg("Invalid timelock period.")]
    InvalidTimelockPeriod,
    #[msg("Timelock period has not expired.")]
    TimelockNotExpired,
    #[msg("Description is too long.")]
    DescriptionTooLong,
    #[msg("Invalid instruction data.")]
    InvalidInstruction,
    #[msg("Duplicate owners not allowed.")]
    DuplicateOwners,
    #[msg("Invalid owner count.")]
    InvalidOwnerCount,
    #[msg("Multisig is paused.")]
    MultisigPaused,
}

#[event]
pub struct MultisigCreated {
    #[index]
    pub multisig: Pubkey,
    pub threshold: u64,
    pub owner_count: u8,  // Changed from usize to u8
}

#[event]
pub struct TransactionCreated {
    #[index]
    pub tx_id: u64,
    pub creator: Pubkey,
    pub created_at: i64,
}

#[event]
pub struct TransactionApproved {
    #[index]
    pub tx_id: u64,
    pub approver: Pubkey,
}

#[event]
pub struct ApprovalRevoked {
    #[index]
    pub tx_id: u64,
    pub revoker: Pubkey,
}

#[event]
pub struct TransactionExecuted {
    #[index]
    pub tx_id: u64,
    pub executor: Pubkey,
}

#[event]
pub struct TransactionCancelled {
    #[index]
    pub tx_id: u64,
    pub cancelled_by: Pubkey,
}

#[event]
pub struct OwnersUpdated {
    #[index]
    pub multisig: Pubkey,
    pub new_owner_count: u8,
}

#[event]
pub struct ThresholdUpdated {
    #[index]
    pub multisig: Pubkey,
    pub new_threshold: u64,
}

#[event]
pub struct PauseToggled {
    #[index]
    pub multisig: Pubkey,
    pub is_paused: bool,
}

// Update the constants section:
pub mod constants {
    pub const MAX_OWNERS: u8 = 20;  // Changed from usize to u8
    pub const MAX_INSTRUCTION_SIZE: usize = 1024;
    pub const MAX_DESCRIPTION_LENGTH: usize = 200;
    pub const MIN_TIMELOCK_PERIOD: i64 = 0;
    pub const MAX_TIMELOCK_PERIOD: i64 = 2_592_000; // 30 days in seconds
}

// Utility functions
pub mod utils {
    use super::*;

    pub fn validate_owners(owners: &[Pubkey], threshold: u64) -> Result<()> {
        require!(!owners.is_empty(), ErrorCode::NoOwnersProvided);
        require!(
            owners.len() <= constants::MAX_OWNERS as usize,
            ErrorCode::InvalidOwnerCount
        );
        require!(
            threshold > 0 && threshold <= owners.len() as u64,
            ErrorCode::InvalidThreshold
        );

        // Check for duplicate owners
        let mut unique_owners = owners.to_vec();
        unique_owners.sort();
        unique_owners.dedup();
        require!(unique_owners.len() == owners.len(), ErrorCode::DuplicateOwners);

        Ok(())
    }


    pub fn validate_timelock(timelock_period: i64) -> Result<()> {
        require!(
            timelock_period >= constants::MIN_TIMELOCK_PERIOD && 
            timelock_period <= constants::MAX_TIMELOCK_PERIOD,
            ErrorCode::InvalidTimelockPeriod
        );
        Ok(())
    }

    pub fn validate_description(description: &str) -> Result<()> {
        require!(
            description.len() <= constants::MAX_DESCRIPTION_LENGTH,
            ErrorCode::DescriptionTooLong
        );
        Ok(())
    }

    pub fn validate_instruction_data(data: &[u8]) -> Result<()> {
        require!(!data.is_empty(), ErrorCode::EmptyInstruction);
        require!(
            data.len() <= constants::MAX_INSTRUCTION_SIZE,
            ErrorCode::InvalidInstruction
        );
        Ok(())
    }
}

// Seeds for PDAs
pub mod seeds {
    pub const MULTISIG_SEED: &[u8] = b"multisig";
    pub const TRANSACTION_SEED: &[u8] = b"transaction";
}

// Add derivation paths for PDAs
impl Multisig {
    pub fn find_multisig_pda(creator: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                seeds::MULTISIG_SEED,
                creator.as_ref(),
            ],
            &crate::ID,
        )
    }
}

impl Transaction {
    pub fn find_transaction_pda(multisig: &Pubkey, tx_id: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                seeds::TRANSACTION_SEED,
                multisig.as_ref(),
                tx_id.to_le_bytes().as_ref(),
            ],
            &crate::ID,
        )
    }
}

// Add constraints for the accounts
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeParams {
    pub owners: Vec<Pubkey>,
    pub threshold: u64,
    pub timelock_period: i64,
}

// Add helper methods for the Multisig account
impl Multisig {
    pub fn validate_owner(&self, owner: &Pubkey) -> Result<()> {
        require!(
            self.owners.contains(owner),
            ErrorCode::NotAuthorized
        );
        Ok(())
    }

    pub fn validate_threshold(&self, threshold: u64) -> Result<()> {
        require!(
            threshold > 0 && threshold <= self.owners.len() as u64,
            ErrorCode::InvalidThreshold
        );
        Ok(())
    }

    pub fn is_valid_transaction(&self, tx: &Transaction) -> Result<()> {
        require!(!self.is_paused, ErrorCode::MultisigPaused);
        require!(!tx.executed, ErrorCode::TransactionAlreadyExecuted);
        require!(!tx.cancelled, ErrorCode::TransactionCancelled);
        
        let current_time = Clock::get()?.unix_timestamp;
        require!(current_time <= tx.expires_at, ErrorCode::TransactionExpired);

        Ok(())
    }
}

// Add helper methods for the Transaction account
impl Transaction {
    pub fn can_execute(&self, threshold: u64) -> Result<()> {
        require!(
            self.approved_by.len() as u64 >= threshold,
            ErrorCode::NotEnoughApprovals
        );

        let current_time = Clock::get()?.unix_timestamp;
        require!(
            current_time <= self.expires_at,
            ErrorCode::TransactionExpired
        );

        Ok(())
    }

    pub fn parse_instruction(&self) -> Result<(Pubkey, &[u8])> {
        require!(self.instruction.len() > 32, ErrorCode::InvalidInstruction);
        
        let program_id = Pubkey::new_from_array(
            self.instruction[..32].try_into().map_err(|_| ErrorCode::InvalidInstruction)?
        );
        
        Ok((program_id, &self.instruction[32..]))
    }
}

// Add a custom result type for better error handling
pub type MultisigResult<T = ()> = std::result::Result<T, Error>;