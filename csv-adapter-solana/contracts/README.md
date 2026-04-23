# CSV Seal - Solana Anchor Program

Cross-Chain Right Transfer implementation on Solana using the Anchor framework.

## Overview

This program implements the same CSV (Client-Side Validation) seal functionality as the Aptos, Sui, and Ethereum contracts:

- **RightAccount**: PDA storing right data (right_id, commitment, owner, nullifier, state_root)
- **LockRegistry**: Tracks lock records for refunds with 24h timeout
- **Events**: Emitted for all cross-chain operations

## Instructions

### `initialize_registry`

Initialize the LockRegistry (called once during deployment).

**Accounts:**

- `registry` (PDA): LockRegistry account
- `authority` (signer): Authority that manages the registry
- `system_program`: System program

### `create_seal`

Create a new Right on Solana.

**Args:**

- `right_id`: [u8; 32] - Unique Right identifier
- `commitment`: [u8; 32] - Commitment hash
- `state_root`: [u8; 32] - Off-chain state commitment

**Accounts:**

- `right_account` (PDA): RightAccount to create
- `owner` (signer): Owner of the right
- `system_program`: System program

### `consume_seal`

Consume a Right (single-use enforcement).

**Accounts:**

- `right_account`: RightAccount to consume
- `consumer` (signer): Must be the owner

### `lock_right`

Lock a Right for cross-chain transfer.

**Args:**

- `destination_chain`: u8 - Target chain ID (0=Bitcoin, 1=Sui, 2=Aptos, 3=Ethereum, 4=Solana)
- `destination_owner`: [u8; 32] - Hashed destination owner address

**Accounts:**

- `right_account`: RightAccount to lock
- `registry`: LockRegistry
- `owner` (signer): Must be the owner
- `recent_blockhashes`: For source tx hash
- `system_program`: System program

### `mint_right`

Mint a new Right from a cross-chain transfer proof.

**Args:**

- `right_id`: [u8; 32] - From source chain
- `commitment`: [u8; 32] - Preserved commitment
- `state_root`: [u8; 32] - Off-chain state
- `source_chain`: u8 - Source chain ID
- `source_seal_ref`: [u8; 32] - Source seal reference

**Accounts:**

- `right_account` (PDA): New RightAccount
- `owner` (signer): New owner
- `system_program`: System program

### `refund_right`

Refund a locked Right after timeout.

**Args:**

- `state_root`: [u8; 32] - New state root

**Accounts:**

- `registry`: LockRegistry
- `original_right`: Original RightAccount (locked)
- `new_right_account` (PDA): New RightAccount to create
- `claimant` (signer): Original owner
- `system_program`: System program

### `transfer_right`

Transfer ownership of a Right.

**Args:**

- `new_owner`: Pubkey - New owner address

**Accounts:**

- `right_account`: RightAccount
- `current_owner` (signer): Current owner

### `register_nullifier`

Register a nullifier for a Right.

**Args:**

- `nullifier`: [u8; 32] - Nullifier hash

**Accounts:**

- `right_account`: RightAccount
- `authority` (signer): Owner or authority

## Events

- `RegistryInitialized`: LockRegistry created
- `RightCreated`: New right created
- `RightConsumed`: Right consumed
- `CrossChainLock`: Right locked for cross-chain transfer
- `CrossChainMint`: Right minted from cross-chain proof
- `CrossChainRefund`: Locked right refunded
- `RightTransferred`: Ownership transferred
- `NullifierRegistered`: Nullifier registered

## Install

```bash
# Anchor
curl --proto '=https' --tlsv1.2 -sSfL https://solana-install.solana.workers.dev | bash      

npm install -g @coral-xyz/anchor-cli

# Cargo build SBF
cargo install cargo-build-sbf
```

## Deployment

```bash
# Build
anchor build

# The outputs goes to another directory: programs/csv-seal/target/sbpf-solana-solana/release/csv_seal.so
# Copy it to: target/deploy/csv_seal.so

# OR
cd programs/csv-seal
cargo build-sbf --no-default-features
cp target/sbpf-solana-solana/release/csv_seal.so ../../target/deploy/

# Deploy to devnet
anchor program deploy --provider.cluster devnet

# Initialize LockRegistry
anchor run initialize --provider.cluster devnet
```

Or use the CLI:

```bash
csv contract deploy solana --network devnet
```

## Chain IDs

- `0`: Bitcoin
- `1`: Sui
- `2`: Aptos
- `3`: Ethereum
- `4`: Solana

## Error Codes

- `6000`: AlreadyConsumed
- `6001`: AlreadyLocked
- `6002`: LockNotFound
- `6003`: RefundTimeoutNotExpired
- `6004`: AlreadyRefunded
- `6005`: NotAuthorized
- `6006`: NullifierAlreadyRegistered
- `6007`: NotConsumed
- `6008`: RegistryFull
- `6009`: InvalidChainId
- `6010`: InvalidCommitment
- `6011`: InvalidProof
- `6012`: RightNotFound
- `6013`: InvalidStateRoot
