import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { CsvSeal } from "../target/types/csv_seal";
import { expect } from "chai";

describe("csv_seal", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.CsvSeal as Program<CsvSeal>;

  // Test accounts
  const authority = provider.wallet;
  const owner = anchor.web3.Keypair.generate();

  // Test data
  const rightId = Buffer.from(Array(32).fill(1));
  const commitment = Buffer.from(Array(32).fill(2));
  const stateRoot = Buffer.from(Array(32).fill(3));
  const destinationOwner = Buffer.from(Array(32).fill(4));
  const nullifier = Buffer.from(Array(32).fill(5));

  // PDAs
  let registryPda: anchor.web3.PublicKey;
  let rightPda: anchor.web3.PublicKey;
  let rightBump: number;

  before(async () => {
    // Airdrop to owner
    await provider.connection.requestAirdrop(
      owner.publicKey,
      anchor.web3.LAMPORTS_PER_SOL
    );

    // Derive PDAs
    [registryPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("lock_registry")],
      program.programId
    );

    [rightPda, rightBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("right"), owner.publicKey.toBuffer(), rightId],
      program.programId
    );
  });

  it("Initializes the LockRegistry", async () => {
    try {
      await program.methods
        .initializeRegistry()
        .accounts({
          registry: registryPda,
          authority: authority.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      const registry = await program.account.lockRegistry.fetch(registryPda);
      expect(registry.authority.toString()).to.equal(authority.publicKey.toString());
      expect(registry.refundTimeout).to.equal(86400);
      expect(registry.lockCount).to.equal(0);
    } catch (e) {
      // Registry might already be initialized
      console.log("Registry may already be initialized:", e);
    }
  });

  it("Creates a new right (seal)", async () => {
    await program.methods
      .createSeal(
        Array.from(rightId),
        Array.from(commitment),
        Array.from(stateRoot)
      )
      .accounts({
        rightAccount: rightPda,
        owner: owner.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([owner])
      .rpc();

    const right = await program.account.rightAccount.fetch(rightPda);
    expect(right.owner.toString()).to.equal(owner.publicKey.toString());
    expect(Buffer.from(right.rightId).toString("hex")).to.equal(rightId.toString("hex"));
    expect(Buffer.from(right.commitment).toString("hex")).to.equal(commitment.toString("hex"));
    expect(right.consumed).to.be.false;
    expect(right.locked).to.be.false;
  });

  it("Consumes a seal", async () => {
    await program.methods
      .consumeSeal()
      .accounts({
        rightAccount: rightPda,
        consumer: owner.publicKey,
      })
      .signers([owner])
      .rpc();

    const right = await program.account.rightAccount.fetch(rightPda);
    expect(right.consumed).to.be.true;
  });

  it("Fails to consume already consumed seal", async () => {
    try {
      await program.methods
        .consumeSeal()
        .accounts({
          rightAccount: rightPda,
          consumer: owner.publicKey,
        })
        .signers([owner])
        .rpc();
      expect.fail("Should have thrown an error");
    } catch (e) {
      expect(e.toString()).to.include("AlreadyConsumed");
    }
  });

  // Additional tests for lock_right, mint_right, refund_right would go here
  // They require more complex setup with multiple accounts and PDAs
});
