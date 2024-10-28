# Solana Multisig Program

A secure and feature-rich multisignature wallet program built on Solana using the Anchor framework.

## Overview

This multisig program allows multiple owners to collectively manage and execute transactions with configurable thresholds, timelock periods, and security features.

## Features

### Security Features
- Configurable timelock period for transaction execution
- Emergency pause mechanism
- Owner authorization checks
- Threshold enforcement
- Transaction validation
- Size limits enforcement
- Duplicate approval prevention

### Transaction Management
- Create transactions with descriptions
- Approve/Revoke transaction approvals
- Execute transactions with timelock
- Cancel transactions
- Transaction expiration mechanism

### Owner Management
- Add/Remove owners
- Update threshold
- Maximum 20 owners per multisig
- Duplicate owner prevention

## Account Structure

### Multisig Account
```rust
pub struct Multisig {
    pub owners: Vec<Pubkey>,
    pub threshold: u64,
    pub transaction_count: u64,
    pub timelock_period: i64,
    pub is_paused: bool,
}
```

### Transaction Account
```rust
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
```

## Program Instructions

### Initialize Multisig
```typescript
program.methods
    .initialize(
        owners: PublicKey[], 
        threshold: BN, 
        timelockPeriod: BN
    )
    .accounts({
        multisig: multisigPDA,
        creator: wallet.publicKey,
        systemProgram: SystemProgram.programId,
    })
```

### Create Transaction
```typescript
program.methods
    .createTransaction(
        programId: PublicKey,
        instructionData: Buffer,
        description: string
    )
    .accounts({
        multisig: multisigPDA,
        transaction: transactionPDA,
        creator: wallet.publicKey,
        systemProgram: SystemProgram.programId,
    })
```

### Approve Transaction
```typescript
program.methods
    .approveTransaction()
    .accounts({
        multisig: multisigPDA,
        transaction: transactionPDA,
        signer: owner.publicKey,
    })
```

### Execute Transaction
```typescript
program.methods
    .executeTransaction()
    .accounts({
        multisig: multisigPDA,
        transaction: transactionPDA,
        signer: executor.publicKey,
    })
```

### Update Owners
```typescript
program.methods
    .updateOwners(newOwners: PublicKey[])
    .accounts({
        multisig: multisigPDA,
        signer: wallet.publicKey,
    })
```

### Update Threshold
```typescript
program.methods
    .updateThreshold(newThreshold: BN)
    .accounts({
        multisig: multisigPDA,
        signer: wallet.publicKey,
    })
```

## Constraints

1. **Size Limits**
   - Maximum 20 owners per multisig
   - Maximum instruction size: 1024 bytes
   - Maximum description length: 200 characters

2. **Time Constraints**
   - Minimum timelock period: 0 seconds
   - Maximum timelock period: 30 days

3. **Security Constraints**
   - Only owners can approve transactions
   - Must meet threshold for execution
   - Cannot execute expired transactions
   - Cannot execute during pause

## Error Codes

```rust
pub enum ErrorCode {
    NotEnoughApprovals,
    TransactionAlreadyExecuted,
    InvalidThreshold,
    NoOwnersProvided,
    EmptyInstruction,
    NotAuthorized,
    TransactionExpired,
    TransactionCancelled,
    InvalidTimelockPeriod,
    TimelockNotExpired,
    DescriptionTooLong,
    InvalidInstruction,
    DuplicateOwners,
    InvalidOwnerCount,
    MultisigPaused,
}
```

## Events

1. MultisigCreated
2. TransactionCreated
3. TransactionApproved
4. ApprovalRevoked
5. TransactionExecuted
6. TransactionCancelled
7. OwnersUpdated
8. ThresholdUpdated
9. PauseToggled

## Getting Started

### Prerequisites
- Solana Tool Suite
- Anchor Framework
- Node.js and yarn
- Rust

### Installation
```bash
# Clone the repository
git clone [repository-url]

# Install dependencies
yarn install

# Build the program
anchor build

# Run tests
anchor test

# Deploy
anchor deploy
```

### Test Suite
```bash
# Run all tests
anchor test

# Run specific test
anchor test [test-name]
```

## Security Considerations

1. **Threshold Configuration**
   - Set appropriate threshold values
   - Consider owner count when updating

2. **Timelock Management**
   - Set appropriate timelock periods
   - Monitor transaction expiration

3. **Owner Management**
   - Regularly verify owner list
   - Plan owner updates carefully

4. **Emergency Procedures**
   - Test pause functionality
   - Document recovery procedures

## Contributing

1. Fork the repository
2. Create feature branch
3. Commit changes
4. Push to branch
5. Create Pull Request

## License

ISC


