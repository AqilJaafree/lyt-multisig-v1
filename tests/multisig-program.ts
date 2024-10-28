import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { MultisigProgram } from "../target/types/multisig_program";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { expect } from 'chai';

describe("multisig_program", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.MultisigProgram as Program<MultisigProgram>;
  
  // Test accounts
  const owner1 = Keypair.generate();
  const owner2 = Keypair.generate();
  const owner3 = Keypair.generate();
  let multisigPDA: PublicKey;
  
  before(async () => {
    // Airdrop SOL to test accounts
    await provider.connection.requestAirdrop(owner1.publicKey, 10 * anchor.web3.LAMPORTS_PER_SOL);
    await provider.connection.requestAirdrop(owner2.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL);
    await provider.connection.requestAirdrop(owner3.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL);
  });

  it("Initializes the multisig", async () => {
    const owners = [owner1.publicKey, owner2.publicKey, owner3.publicKey];
    const threshold = new anchor.BN(2);
    const timelockPeriod = new anchor.BN(3600); // 1 hour

    const multisig = Keypair.generate();

    try {
      await program.methods
        .initialize(owners, threshold, timelockPeriod)
        .accounts({
          multisig: multisig.publicKey,
          creator: owner1.publicKey,
        })
        .signers([multisig, owner1])
        .rpc();

      const multisigAccount = await program.account.multisig.fetch(multisig.publicKey);
      
      expect(multisigAccount.owners).to.deep.equal(owners);
      expect(multisigAccount.threshold.toString()).to.equal(threshold.toString());
      expect(multisigAccount.timelockPeriod.toString()).to.equal(timelockPeriod.toString());
      expect(multisigAccount.transactionCount.toString()).to.equal("0");
      expect(multisigAccount.isPaused).to.equal(false);

      multisigPDA = multisig.publicKey;
    } catch (error) {
      console.error("Error:", error);
      throw error;
    }
  });

  it("Creates a transaction", async () => {
    const transaction = Keypair.generate();
    const programId = SystemProgram.programId;
    const instructionData = Buffer.from("test instruction");
    const description = "Test transaction";

    try {
      await program.methods
        .createTransaction(programId, instructionData, description)
        .accounts({
          multisig: multisigPDA,
          transaction: transaction.publicKey,
          creator: owner1.publicKey,
        })
        .signers([transaction, owner1])
        .rpc();

      const txAccount = await program.account.transaction.fetch(transaction.publicKey);
      expect(txAccount.creator).to.deep.equal(owner1.publicKey);
      expect(txAccount.description).to.equal(description);
      expect(txAccount.cancelled).to.equal(false);
      expect(txAccount.executed).to.equal(false);
    } catch (error) {
      console.error("Error:", error);
      throw error;
    }
  });

  it("Approves a transaction", async () => {
    const transaction = Keypair.generate();
    const programId = SystemProgram.programId;
    const instructionData = Buffer.from("test instruction");
    const description = "Test transaction";

    // First create a transaction
    await program.methods
      .createTransaction(programId, instructionData, description)
      .accounts({
          multisig: multisigPDA,
          transaction: transaction.publicKey,
          creator: owner1.publicKey,
      })
      .signers([transaction, owner1])
      .rpc();

    // Now approve it
    try {
      await program.methods
        .approveTransaction()
        .accounts({
          multisig: multisigPDA,
          transaction: transaction.publicKey,
          signer: owner2.publicKey,
        })
        .signers([owner2])
        .rpc();

      const txAccount = await program.account.transaction.fetch(transaction.publicKey);
      expect(txAccount.approvedBy).to.include(owner2.publicKey);
    } catch (error) {
      console.error("Error:", error);
      throw error;
    }
  });

  it("Executes a transaction after threshold approvals", async () => {
    const transaction = Keypair.generate();
    const programId = SystemProgram.programId;
    const instructionData = Buffer.from("test instruction");
    const description = "Test transaction";

    // Create transaction
    await program.methods
      .createTransaction(programId, instructionData, description)
      .accounts({
          multisig: multisigPDA,
          transaction: transaction.publicKey,
          creator: owner1.publicKey,
      })
      .signers([transaction, owner1])
      .rpc();

    // Get approvals from owner1 and owner2
    await program.methods
      .approveTransaction()
      .accounts({
        multisig: multisigPDA,
        transaction: transaction.publicKey,
        signer: owner1.publicKey,
      })
      .signers([owner1])
      .rpc();

    await program.methods
      .approveTransaction()
      .accounts({
        multisig: multisigPDA,
        transaction: transaction.publicKey,
        signer: owner2.publicKey,
      })
      .signers([owner2])
      .rpc();

    // Wait for timelock to expire
    await new Promise(resolve => setTimeout(resolve, 3600 * 1000));

    // Execute transaction
    try {
      await program.methods
        .executeTransaction()
        .accounts({
          multisig: multisigPDA,
          transaction: transaction.publicKey,
          signer: owner1.publicKey,
        })
        .remainingAccounts([
          {
            pubkey: SystemProgram.programId,
            isWritable: false,
            isSigner: false,
          },
        ])
        .signers([owner1])
        .rpc();

      const txAccount = await program.account.transaction.fetch(transaction.publicKey);
      expect(txAccount.executed).to.equal(true);
    } catch (error) {
      console.error("Error:", error);
      throw error;
    }
  });

  // Rest of your test cases...
});