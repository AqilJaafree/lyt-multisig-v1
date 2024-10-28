// tests/multisig.rs
use anchor_lang::{prelude::*, solana_program::system_instruction};
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use multisig_program::state::*;
use multisig_program::error::*;

pub struct TestContext {
    pub program_id: Pubkey,
    pub payer: Keypair,
    pub last_blockhash: Hash,
    pub banks_client: BanksClient,
}

async fn setup() -> (TestContext, Vec<Keypair>) {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "multisig_program",
        program_id,
        processor!(multisig_program::entry::entry),
    );

    let mut context = program_test.start_with_context().await;
    let payer = Keypair::new();

    // Create test owners
    let owner1 = Keypair::new();
    let owner2 = Keypair::new();
    let owner3 = Keypair::new();
    let owners = vec![owner1, owner2, owner3];

    // Airdrop SOL to payer
    context.banks_client
        .process_transaction(Transaction::new_signed_with_payer(
            &[system_instruction::transfer(
                &context.payer.pubkey(),
                &payer.pubkey(),
                1_000_000_000,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        ))
        .await
        .unwrap();

    let test_context = TestContext {
        program_id,
        payer: payer.clone(),
        last_blockhash: context.last_blockhash,
        banks_client: context.banks_client,
    };

    (test_context, owners)
}

#[tokio::test]
async fn test_initialize_multisig() {
    let (context, owners) = setup().await;
    
    // Find PDA for multisig
    let (multisig_pda, _bump) = Pubkey::find_program_address(
        &[b"multisig", context.payer.pubkey().as_ref()],
        &context.program_id,
    );
    
    let owner_pubkeys: Vec<Pubkey> = owners.iter().map(|kp| kp.pubkey()).collect();
    let threshold = 2u64;
    let timelock = 3600i64; // 1 hour

    let accounts = multisig_program::accounts::Initialize {
        multisig: multisig_pda,
        creator: context.payer.pubkey(),
        system_program: system_program::ID,
    };

    let ix = multisig_program::instruction::initialize(
        context.program_id,
        accounts,
        owner_pubkeys.clone(),
        threshold,
        timelock,
    );

    let mut transaction = Transaction::new_with_payer(
        &[ix],
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer], context.last_blockhash);

    context.banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Verify the state
    let multisig_account = context.banks_client
        .get_account(multisig_pda)
        .await
        .unwrap()
        .unwrap();

    let multisig = Multisig::try_deserialize(&mut multisig_account.data.as_ref()).unwrap();
    assert_eq!(multisig.owners, owner_pubkeys);
    assert_eq!(multisig.threshold, threshold);
    assert_eq!(multisig.transaction_count, 0);
    assert_eq!(multisig.timelock_period, timelock);
    assert!(!multisig.is_paused);
}

#[tokio::test]
async fn test_create_transaction() {
    let (context, owners) = setup().await;
    
    // First initialize the multisig
    let (multisig_pda, _) = Pubkey::find_program_address(
        &[b"multisig", context.payer.pubkey().as_ref()],
        &context.program_id,
    );

    // Initialize multisig first
    let owner_pubkeys: Vec<Pubkey> = owners.iter().map(|kp| kp.pubkey()).collect();
    initialize_multisig(&context, &multisig_pda, &owner_pubkeys).await;

    // Create transaction PDA
    let (transaction_pda, _) = Pubkey::find_program_address(
        &[b"transaction", multisig_pda.as_ref(), &[0u64.to_le_bytes()].concat()],
        &context.program_id,
    );

    // Create a test transaction
    let program_id_to_call = Pubkey::new_unique();
    let instruction_data = vec![1, 2, 3, 4];
    let description = "Test transaction".to_string();

    let accounts = multisig_program::accounts::CreateTransaction {
        multisig: multisig_pda,
        transaction: transaction_pda,
        creator: context.payer.pubkey(),
        system_program: system_program::ID,
    };

    let ix = multisig_program::instruction::create_transaction(
        context.program_id,
        accounts,
        program_id_to_call,
        instruction_data.clone(),
        description.clone(),
    );

    let mut transaction = Transaction::new_with_payer(
        &[ix],
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer], context.last_blockhash);

    context.banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Verify the transaction state
    let tx_account = context.banks_client
        .get_account(transaction_pda)
        .await
        .unwrap()
        .unwrap();

    let tx = multisig_program::state::Transaction::try_deserialize(&mut tx_account.data.as_ref()).unwrap();
    assert_eq!(tx.creator, context.payer.pubkey());
    assert_eq!(tx.description, description);
    assert!(!tx.cancelled);
    assert!(!tx.executed);
    assert_eq!(tx.approved_by.len(), 0);
}

#[tokio::test]
async fn test_approve_transaction() {
    let (context, owners) = setup().await;
    
    // Setup multisig and transaction first
    let (multisig_pda, _) = Pubkey::find_program_address(
        &[b"multisig", context.payer.pubkey().as_ref()],
        &context.program_id,
    );
    let (transaction_pda, _) = Pubkey::find_program_address(
        &[b"transaction", multisig_pda.as_ref(), &[0u64.to_le_bytes()].concat()],
        &context.program_id,
    );

    let owner_pubkeys: Vec<Pubkey> = owners.iter().map(|kp| kp.pubkey()).collect();
    
    // Initialize multisig and create transaction
    initialize_multisig(&context, &multisig_pda, &owner_pubkeys).await;
    create_test_transaction(&context, &multisig_pda, &transaction_pda).await;

    // Approve transaction
    let accounts = multisig_program::accounts::ApproveTransaction {
        multisig: multisig_pda,
        transaction: transaction_pda,
        signer: owners[0].pubkey(),
    };

    let ix = multisig_program::instruction::approve_transaction(
        context.program_id,
        accounts,
    );

    let mut transaction = Transaction::new_with_payer(
        &[ix],
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer, &owners[0]], context.last_blockhash);

    context.banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Verify approval state
    let tx_account = context.banks_client
        .get_account(transaction_pda)
        .await
        .unwrap()
        .unwrap();

    let tx = multisig_program::state::Transaction::try_deserialize(&mut tx_account.data.as_ref()).unwrap();
    assert_eq!(tx.approved_by.len(), 1);
    assert!(tx.approved_by.contains(&owners[0].pubkey()));
}

// Helper functions
async fn initialize_multisig(
    context: &TestContext,
    multisig_pda: &Pubkey,
    owner_pubkeys: &[Pubkey],
) {
    let accounts = multisig_program::accounts::Initialize {
        multisig: *multisig_pda,
        creator: context.payer.pubkey(),
        system_program: system_program::ID,
    };

    let ix = multisig_program::instruction::initialize(
        context.program_id,
        accounts,
        owner_pubkeys.to_vec(),
        2u64,
        3600i64,
    );

    let mut transaction = Transaction::new_with_payer(
        &[ix],
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer], context.last_blockhash);

    context.banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

async fn create_test_transaction(
    context: &TestContext,
    multisig_pda: &Pubkey,
    transaction_pda: &Pubkey,
) {
    let accounts = multisig_program::accounts::CreateTransaction {
        multisig: *multisig_pda,
        transaction: *transaction_pda,
        creator: context.payer.pubkey(),
        system_program: system_program::ID,
    };

    let ix = multisig_program::instruction::create_transaction(
        context.program_id,
        accounts,
        Pubkey::new_unique(),
        vec![1, 2, 3, 4],
        "Test transaction".to_string(),
    );

    let mut transaction = Transaction::new_with_payer(
        &[ix],
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer], context.last_blockhash);

    context.banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}