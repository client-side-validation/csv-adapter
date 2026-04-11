# Developer Guide

> How to build on CSV — from first Right to production deployment

---

## 1. Getting Started

### Prerequisites

```bash
# Rust toolchain (1.80+)
rustup install stable

# Chain CLIs (for deployment and testing)
curl -L https://foundry.paradigm.xyz | bash && foundryup          # Ethereum
cargo install --locked --git https://github.com/MystenLabs/sui.git --bin sui  # Sui
cargo install --git https://github.com/aptos-labs/aptos-core.git aptos        # Aptos
```

### Project Structure

```
csv-adapter/
├── csv-adapter-core/         # Core types: Right, Commitment, SealRef, ProofBundle
├── csv-adapter-bitcoin/      # Bitcoin Signet adapter (UTXO + Tapret)
├── csv-adapter-ethereum/     # Ethereum Sepolia adapter (Solidity contracts)
├── csv-adapter-sui/          # Sui Testnet adapter (Move contracts)
├── csv-adapter-aptos/        # Aptos Testnet adapter (Move contracts)
├── csv-adapter-store/        # Persistent seal storage
├── csv-cli/                  # Command-line interface
│   └── src/
│       ├── commands/
│       │   ├── wallet.rs     # Wallet generation, funding, balance
│       │   ├── contract.rs   # Contract deployment
│       │   ├── tests.rs      # E2E test runner
│       │   └── ...
│       └── main.rs
└── docs/
    ├── ARCHITECTURE.md       # System architecture
    ├── BLUEPRINT.md          # Development roadmap
    └── E2E_TESTNET_MANUAL.md # End-to-end testing guide
```

### Quick Build

```bash
cargo build --release -p csv-cli
./target/release/csv --help
```

---

## 2. Core Concepts

### The Right

A **Right** is a verifiable, single-use digital asset. It has:

- **`right_id`**: Unique identifier, computed as `H(commitment || salt)`
- **`commitment`**: Hash of the Right's state and rules
- **`owner`**: Proof of ownership (chain-specific format)
- **`nullifier`**: Optional — set when the Right is consumed

```rust
use csv_adapter_core::{Right, Hash, OwnershipProof};

// Create a Right
let commitment = Hash::new([0xAB; 32]);
let owner = OwnershipProof {
    proof: vec![/* signature bytes */],
    owner: vec![/* owner address */],
    scheme: Some(SignatureScheme::Secp256k1),
};
let right = Right::new(commitment, owner, b"unique-salt");

// Transfer to new owner
let new_right = right.transfer(new_owner, b"transfer-salt");

// Consume (single-use enforcement)
let mut right = right;
right.consume(Some(b"my-secret"), Some(&chain_context));
```

### The Commitment

A **Commitment** binds a state transition to an anchor on the blockchain:

```rust
use csv_adapter_core::Commitment;

// Simple commitment (single protocol)
let commitment = Commitment::simple(
    contract_id,       // What Right this is for
    previous_commitment, // Hash of previous commitment (or zero)
    payload_hash,       // What changed
    &seal_ref,          // What seal was consumed
    domain_separator,   // Chain-specific isolation
);

// Hash the commitment
let hash = commitment.hash();
```

### The Seal

A **seal** is the single-use mechanism. Each chain has its own seal type:

| Chain | Seal | How It Works |
|-------|------|-------------|
| Bitcoin | `txid:vout` | Spending the UTXO destroys the seal |
| Sui | `object_id:version` | Deleting the object destroys the seal |
| Aptos | `resource_address:account` | Destroying the resource destroys the seal |
| Ethereum | `nullifier_hash` | Registering the nullifier consumes the seal |

### The Proof Bundle

A **ProofBundle** contains everything needed to verify a Right was locked on the source chain:

```rust
use csv_adapter_core::{ProofBundle, InclusionProof, FinalityProof};

struct ProofBundle {
    inclusion_proof: InclusionProof,  // "This tx is in this block"
    finality_proof: FinalityProof,    // "This block is finalized"
    seal_ref: SealRef,                // "This seal was consumed"
    anchor_ref: AnchorRef,            // "This anchor was published"
}
```

---

## 3. Building a Chain Adapter

Every chain adapter implements the `AnchorLayer` trait. Here's the minimal implementation:

```rust
use csv_adapter_core::{AnchorLayer, Hash, DAGSegment, ProofBundle, SignatureScheme};

pub struct MyChainAdapter {
    config: MyConfig,
    // ... chain-specific state
}

impl AnchorLayer for MyChainAdapter {
    type SealRef = MySealRef;
    type AnchorRef = MyAnchorRef;
    type InclusionProof = MyInclusionProof;
    type FinalityProof = MyFinalityProof;

    fn publish(&self, commitment: Hash, seal: Self::SealRef) -> Result<Self::AnchorRef> {
        // 1. Build a transaction that includes the commitment
        // 2. Sign and broadcast it
        // 3. Return the anchor reference (tx hash, block height)
        todo!()
    }

    fn verify_inclusion(&self, anchor: Self::AnchorRef) -> Result<Self::InclusionProof> {
        // 1. Fetch the block containing the anchor transaction
        // 2. Extract the Merkle/state proof
        // 3. Return the inclusion proof
        todo!()
    }

    fn verify_finality(&self, anchor: Self::AnchorRef) -> Result<Self::FinalityProof> {
        // 1. Check how many confirmations the anchor has
        // 2. Compare against required finality depth
        // 3. Return the finality proof
        todo!()
    }

    fn enforce_seal(&self, seal: Self::SealRef) -> Result<()> {
        // 1. Check if seal was already consumed
        // 2. Mark it as consumed
        // 3. Return error if already consumed (replay prevention)
        todo!()
    }

    fn create_seal(&self, value: Option<u64>) -> Result<Self::SealRef> {
        // 1. Generate a new seal reference
        //    (e.g., derive a new UTXO, create a new object)
        // 2. Return the seal reference
        todo!()
    }

    fn hash_commitment(
        &self,
        contract_id: Hash,
        previous_commitment: Hash,
        transition_payload_hash: Hash,
        seal_ref: &Self::SealRef,
    ) -> Hash {
        // Compute the commitment hash using the chain's domain separator
        todo!()
    }

    fn build_proof_bundle(
        &self,
        anchor: Self::AnchorRef,
        transition_dag: DAGSegment,
    ) -> Result<ProofBundle> {
        // 1. Get inclusion proof for the anchor
        // 2. Get finality proof for the anchor
        // 3. Build the proof bundle
        todo!()
    }

    fn rollback(&self, anchor: Self::AnchorRef) -> Result<()> {
        // Handle chain reorganizations
        // Clear seals that were anchored in rolled-back blocks
        todo!()
    }

    fn domain_separator(&self) -> [u8; 32] {
        // Return a unique 32-byte domain separator for this chain
        // Prevents cross-chain replay attacks
        todo!()
    }

    fn signature_scheme(&self) -> SignatureScheme {
        // Return the signature scheme used by this chain
        // Secp256k1 for Bitcoin/Ethereum, Ed25519 for Sui/Aptos
        todo!()
    }
}
```

### Adapter File Structure

```
csv-adapter-mychain/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Module exports
│   ├── adapter.rs       # AnchorLayer implementation
│   ├── config.rs        # Network + contract configuration
│   ├── error.rs         # Chain-specific error types
│   ├── proofs.rs        # Inclusion + finality proof types
│   ├── real_rpc.rs      # Real RPC client (feature-gated)
│   ├── seal.rs          # Seal registry
│   ├── signatures.rs    # Signing scheme
│   └── types.rs         # Chain-specific types
└── tests/
    └── testnet_e2e.rs   # Network-dependent integration tests
```

### RPC Implementation

For real network interaction, implement an RPC client:

```rust
// Using mempool.space-style REST API
pub struct MyChainRpc {
    client: Client,
    base_url: String,
}

impl MyChainRpc {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        Self { client, base_url }
    }

    // Add retry logic with exponential backoff
    fn get_with_retry<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<T, Error> {
        // Retry up to 3 times with 2s → 4s → 8s backoff
        todo!()
    }

    // Chain-specific RPC methods
    pub fn get_block_count(&self) -> Result<u64, Error> { todo!() }
    pub fn send_raw_transaction(&self, tx_bytes: Vec<u8>) -> Result<[u8; 32], Error> { todo!() }
}
```

---

## 4. Testing

### Unit Tests

```bash
# Run all unit tests
cargo test

# Run tests for a specific adapter
cargo test -p csv-adapter-bitcoin
cargo test -p csv-adapter-ethereum
cargo test -p csv-adapter-sui
```

### E2E Tests

```bash
# Run all 9 cross-chain transfer tests
./target/release/csv test run --all

# Run a specific chain pair
./target/release/csv test run -p ethereum:sui

# Run Bitcoin Signet real transaction test
cargo test -p csv-adapter-bitcoin --test signet_real_tx --features signet-rest -- --ignored --nocapture
```

### Integration Test Setup

For real network tests, set environment variables:

```bash
# Bitcoin Signet
export CSV_SIGNET_FUNDING_TXID="<txid-of-funding-transaction>"
export CSV_SIGNET_FUNDING_VOUT=0
export CSV_SIGNET_FUNDING_AMOUNT=10000
```

---

## 5. Deployment

### Ethereum (Sepolia)

```bash
cd csv-adapter-ethereum/contracts

# Build contracts
~/.foundry/bin/forge build

# Deploy (requires DEPLOYER_KEY env var)
DEPLOYER_KEY="0x<private-key>" ~/.foundry/bin/forge script script/Deploy.s.sol \
  --rpc-url https://ethereum-sepolia-rpc.publicnode.com \
  --broadcast
```

### Sui (Testnet)

```bash
# Publish package
sui client publish csv-adapter-sui/contracts --gas-budget 500000000 --json

# Note the packageId from output, then create registry:
sui client call \
  --package <package-id> \
  --module csv_seal \
  --function create_registry \
  --gas-budget 10000000
```

### Aptos (Testnet)

```bash
# Compile
aptos move compile --package-dir csv-adapter-aptos/contracts

# Publish
aptos move publish \
  --package-dir csv-adapter-aptos/contracts \
  --profile default \
  --assume-yes

# Initialize registry
aptos move run \
  --function-id "<account>::csv_seal::init_registry" \
  --profile default \
  --assume-yes
```

---

## 6. Using the CLI

```bash
# Generate wallets
./target/release/csv wallet generate bitcoin
./target/release/csv wallet generate ethereum
./target/release/csv wallet wallet generate sui
./target/release/csv wallet generate aptos

# Check balances
./target/release/csv wallet balance bitcoin
./target/release/csv wallet balance ethereum

# Deploy contracts
./target/release/csv contract deploy ethereum
./target/release/csv contract deploy sui

# Run tests
./target/release/csv test run --all

# View results
./target/release/csv test results
./target/release/csv contract list
```

---

## 7. Adding a New Chain

### Step 1: Create the Adapter Crate

```bash
# Copy an existing adapter as a template
cp -r csv-adapter-sui csv-adapter-mychain
cd csv-adapter-mychain

# Update Cargo.toml
sed -i 's/csv-adapter-sui/csv-adapter-mychain/g' Cargo.toml
```

### Step 2: Define Chain Types

```rust
// src/types.rs

/// Seal reference for MyChain
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MyChainSealRef {
    /// Transaction hash that consumed the seal
    pub tx_hash: [u8; 32],
    /// Output/index within the transaction
    pub output_index: u32,
    /// Optional nonce
    pub nonce: Option<u64>,
}

/// Anchor reference for MyChain
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MyChainAnchorRef {
    /// Transaction hash containing the commitment
    pub tx_hash: [u8; 32],
    /// Block height where the transaction was included
    pub block_height: u64,
}
```

### Step 3: Implement AnchorLayer

Follow the template in Section 3 of this guide.

### Step 4: Add to CLI

```rust
// csv-cli/src/config.rs
pub enum Chain {
    Bitcoin,
    Ethereum,
    Sui,
    Aptos,
    MyChain,  // Add your chain
}

// csv-cli/src/commands/tests.rs
let pairs = if all {
    vec![
        // ... existing pairs
        (Chain::MyChain, Chain::Ethereum),
        (Chain::Ethereum, Chain::MyChain),
    ]
}
```

### Step 5: Write Tests

```rust
// tests/testnet_e2e.rs
#[test]
#[ignore]
fn test_mychain_testnet_e2e() {
    // Connect to real network
    // Create seal
    // Publish commitment
    // Verify inclusion
    // Verify finality
}
```

---

## 8. Stability Guarantees

### Stable API (SemVer-compliant)

The following will **not** change without a major version bump:

- All types in `csv-adapter-core/src/{hash,seal,right,commitment,proof}.rs`
- The `AnchorLayer` trait and its associated types
- Error types and their variants
- Serialization formats (canonical bytes)

### Internal API (May Change)

- State machine types (`genesis`, `schema`, `state`, `transition`, `consignment`)
- VM types (`vm`)
- MPC types (`mpc`)
- Monitor and store internals

### Version Numbers

| Component | Current Version | Breaking Change Policy |
|-----------|----------------|----------------------|
| csv-adapter-core | 0.1.0 | Major version bump for any breaking change |
| csv-adapter-bitcoin | 0.1.0 | Major version bump for any breaking change |
| csv-adapter-ethereum | 0.1.0 | Major version bump for any breaking change |
| csv-adapter-sui | 0.1.0 | Major version bump for any breaking change |
| csv-adapter-aptos | 0.1.0 | Major version bump for any breaking change |
| csv-cli | 0.1.0 | Major version bump for CLI breaking changes |

---

## 9. Contributing

### Code Style

```bash
# Format all code
cargo fmt

# Check for clippy warnings
cargo clippy --all-features --all-targets -- -D warnings

# Run all tests
cargo test
```

### Pull Request Checklist

- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] E2E tests pass (`./target/release/csv test run --all`)
- [ ] New types have doc comments
- [ ] New error types have `thiserror::Error` derive
- [ ] Public API changes are documented in CHANGELOG.md
