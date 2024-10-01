# Solana Multisig Program

This program implements a multisig wallet on the Solana blockchain, allowing multiple parties to collectively manage and approve transactions.

## Program ID

The program is deployed on Solana devnet with the following address:

```
9tX4QfdBjXLUXiV1htgqcLedzygnm87zh58DWENv59f1
```

You can view the program details on the Solana Explorer:
[https://explorer.solana.com/address/9tX4QfdBjXLUXiV1htgqcLedzygnm87zh58DWENv59f1?cluster=devnet](https://explorer.solana.com/address/9tX4QfdBjXLUXiV1htgqcLedzygnm87zh58DWENv59f1?cluster=devnet)

## Features

- Create multisig wallets with customizable owners and approval thresholds
- Propose transactions
- Approve transactions
- Execute transactions once the approval threshold is met

## Instructions

The program includes the following main instructions:

1. `create_multisig`: Initialize a new multisig wallet
2. `create_transaction`: Propose a new transaction
3. `approve_transaction`: Approve a pending transaction
4. `execute_transaction`: Execute a transaction that has met the approval threshold

## Account Structures

### Multisig

```rust
pub struct Multisig {
    pub owners: Vec<Pubkey>,
    pub threshold: u64,
    pub transaction_count: u64,
}
```

### Transaction

```rust
pub struct Transaction {
    pub creator: Pubkey,
    pub instruction: Vec<u8>,
    pub approved_by: Vec<Pubkey>,
    pub executed: bool,
}
```

## Development and Deployment

This program is developed using the Anchor framework. To build and deploy:

1. Build the program:
   ```bash
   anchor build
   ```

2. Deploy to devnet:
   ```bash
   anchor deploy
   ```

3. Initialize the IDL:
   ```bash
   anchor idl init 9tX4QfdBjXLUXiV1htgqcLedzygnm87zh58DWENv59f1 -f target/idl/multisig_program.json
   ```

## Interacting with the Program

You can interact with the program using the Anchor client libraries or by sending transactions directly through the Solana web3.js library.

## Security Considerations

- Ensure that only authorized parties can create multisigs and transactions
- Verify the threshold logic works correctly
- Test edge cases, such as executing transactions without enough approvals

## Deployment Proof

The program has been successfully deployed to Solana devnet. You can verify the deployment using the Solana CLI:

```bash
solana program show 9tX4QfdBjXLUXiV1htgqcLedzygnm87zh58DWENv59f1 --url devnet
```

Output:
```
Program Id: 9tX4QfdBjXLUXiV1htgqcLedzygnm87zh58DWENv59f1
Owner: BPFLoaderUpgradeab1e11111111111111111111111
ProgramData Address: 3eo2MGS2D3qQNxfboHAt6KspV2w7BqLK3TnJBRCj2cqM
Authority: 57wMKYdCPiA8tn28t2ucZkxEz9Lvd9eMLDLXf5kJzR1h
Last Deployed In Slot: 329923560
Data Length: 235520 (0x39800) bytes
Balance: 1.64042328 SOL
```

The program's IDL has also been initialized:

```bash
anchor idl show 9tX4QfdBjXLUXiV1htgqcLedzygnm87zh58DWENv59f1
```

IDL Account: `EScpbwUxwTv5wkRQjHP4RvE3XRaXGiqpHkJ4LpA7pYJU`

## License



## Contributing

