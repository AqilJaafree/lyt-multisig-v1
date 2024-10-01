use anchor_lang::prelude::*;

declare_id!("9tX4QfdBjXLUXiV1htgqcLedzygnm87zh58DWENv59f1");

#[program]
pub mod multisig_program {
    use super::*;

    pub fn create_multisig(
        ctx: Context<CreateMultisig>,
        owners: Vec<Pubkey>,
        threshold: u64,
    ) -> Result<()> {
        require!(threshold > 0 && threshold <= owners.len() as u64, ErrorCode::InvalidThreshold);
        require!(!owners.is_empty(), ErrorCode::NoOwnersProvided);

        let multisig = &mut ctx.accounts.multisig;
        multisig.owners = owners;
        multisig.threshold = threshold;
        multisig.transaction_count = 0;
        Ok(())
    }

    pub fn create_transaction(
        ctx: Context<CreateTransaction>,
        instruction: Vec<u8>,
    ) -> Result<()> {
        require!(!instruction.is_empty(), ErrorCode::EmptyInstruction);

        let multisig = &mut ctx.accounts.multisig;
        let tx = &mut ctx.accounts.transaction;
        tx.creator = *ctx.accounts.creator.key;
        tx.instruction = instruction;
        tx.approved_by = Vec::new();
        tx.executed = false;
        multisig.transaction_count += 1;
        Ok(())
    }

    pub fn approve_transaction(ctx: Context<ApproveTransaction>) -> Result<()> {
        let multisig = &ctx.accounts.multisig;
        let tx = &mut ctx.accounts.transaction;
        let signer = ctx.accounts.signer.key();
    
        require!(!tx.executed, ErrorCode::TransactionAlreadyExecuted);
        require!(multisig.owners.contains(&signer), ErrorCode::NotAuthorized);
    
        if !tx.approved_by.contains(&signer) {
            tx.approved_by.push(signer);
        }
    
        Ok(())
    }

    pub fn execute_transaction(ctx: Context<ExecuteTransaction>) -> Result<()> {
        let tx = &mut ctx.accounts.transaction;
        let multisig = &ctx.accounts.multisig;

        require!(!tx.executed, ErrorCode::TransactionAlreadyExecuted);

        let num_approvals = tx.approved_by.len() as u64;
        require!(num_approvals >= multisig.threshold, ErrorCode::NotEnoughApprovals);

        // Placeholder for executing the instruction
        // Implement actual instruction execution logic here
        // For example, using invoke or invoke_signed for CPI

        tx.executed = true;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateMultisig<'info> {
    #[account(
        init,
        payer = creator,
        space = Multisig::LEN
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
        space = Transaction::LEN
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

#[account]
pub struct Multisig {
    pub owners: Vec<Pubkey>,
    pub threshold: u64,
    pub transaction_count: u64,
}

impl Multisig {
    const LEN: usize = 8 + // Anchor discriminator
        4 + // Vector length prefix for owners
        (32 * 10) + // Assuming a maximum of 10 owners (adjust as needed)
        8 + // u64 threshold
        8; // u64 transaction_count
}

#[account]
pub struct Transaction {
    pub creator: Pubkey,
    pub instruction: Vec<u8>,
    pub approved_by: Vec<Pubkey>,
    pub executed: bool,
}

impl Transaction {
    const LEN: usize = 8 + // Anchor discriminator
        32 + // Pubkey creator
        4 + // Vector length prefix for instruction
        1024 + // Assuming max instruction size (adjust as needed)
        4 + // Vector length prefix for approved_by
        (32 * 10) + // Assuming a maximum of 10 approvals (adjust as needed)
        1; // bool executed
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
}