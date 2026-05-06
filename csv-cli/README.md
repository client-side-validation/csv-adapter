# CSV Adapter CLI

**Command-line tool for cross-chain Sanads, proofs, wallets, and end-to-end testing.**

```
csv — CSV Adapter CLI v0.1.0
Cross-Chain Sanads, Proofs, and Transfers
```

---

## Installation

```bash
# Build from source
cargo build -p csv-cli --release

# Or install globally
cargo install --path csv-cli
```

The binary is available as `csv`:

```bash
csv --help
```

---

## Quick Start

```bash
# 1. One-command setup: Generate wallets for all chains
csv wallet init                    # Uses default account 0
csv wallet init --account 1        # Use Bitcoin account index 1 (BIP-86)
csv wallet init --network test     # Testnet setup

# Or generate wallets individually:
csv wallet generate bitcoin test
csv wallet generate ethereum
csv wallet generate sui test
csv wallet generate aptos test
csv wallet generate solana  # Solana wallet (devnet)

# 2. Fund wallets from testnet faucets
csv wallet fund bitcoin
csv wallet fund ethereum
csv wallet fund sui
csv wallet fund aptos
csv wallet fund solana

# 3. Check balances
csv wallet balance bitcoin
csv wallet balance ethereum
csv wallet balance sui
csv wallet balance aptos
csv wallet balance solana

# 4. Deploy contracts (not needed for Bitcoin — UTXO-native)
csv contract deploy --chain sui
csv contract deploy --chain ethereum

# 5. Create a Sanad on Bitcoin
csv sanad create --chain bitcoin --value 100000

# 6. Transfer it cross-chain to Sui
csv cross-chain transfer --from bitcoin --to sui --sanad-id 0x...

# 7. Verify the proof
csv proof verify-cross-chain --source bitcoin --dest sui --proof proof.json
```

---

## Command Reference

### Chain Management

**List all supported chains:**

```bash
csv chain list
```

The CLI now uses a **plugin-based scalable architecture** supporting 5+ chains:
- Bitcoin (UTXO-based)
- Ethereum (Account-based with smart contracts)
- Solana (High-performance, account-based)
- Sui (Object-based)
- Aptos (Resource-based)

**Check chain status and connectivity:**

```bash
csv chain status bitcoin
csv chain status ethereum
csv chain status solana
csv chain status sui
csv chain status aptos
```

**View RPC endpoint info:**

```bash
csv chain info bitcoin
csv chain info aptos
```

**Change RPC URL:**

```bash
csv chain set-rpc bitcoin http://localhost:38332
csv chain set-rpc ethereum https://rpc.sepolia.org
```

**Change network:**

```bash
csv chain set-network bitcoin test    # Signet
csv chain set-network bitcoin dev     # Regtest
csv chain set-network bitcoin main    # Mainnet
```

**Set contract address:**

```bash
csv chain set-contract ethereum 0x1234...abcd
csv chain set-contract sui 0x5678...ef01
```

---

### Wallet Operations

**Initialize all wallets (one-command setup):**

```bash
# Generate wallets for all chains with default Bitcoin account 0
csv wallet init

# Use a specific Bitcoin account index (BIP-86 multi-account support)
csv wallet init --account 1
csv wallet init --account 2 --network test

# Full example: 24-word mnemonic, testnet, account 5, with faucet funding
csv wallet init --words 24 --network test --account 5 --fund true
```

**Generate a new wallet:**

```bash
# Bitcoin (BIP-86 Taproot)
csv wallet generate bitcoin test

# Ethereum (secp256k1)
csv wallet generate ethereum

# Sui (Ed25519)
csv wallet generate sui test

# Aptos (Ed25519)
csv wallet generate aptos test

# Solana (Ed25519, base58 addresses)
csv wallet generate solana
```

**Check balance:**

```bash
csv wallet balance bitcoin
csv wallet balance --address tb1p7xr... bitcoin
csv wallet balance ethereum
csv wallet balance solana
csv wallet balance sui
csv wallet balance aptos
```

**Fund from faucet:**

```bash
csv wallet fund bitcoin    # Uses Signet faucet
csv wallet fund ethereum   # Uses Sepolia faucet
csv wallet fund solana     # Uses Solana Devnet faucet
csv wallet fund sui        # Uses Sui Testnet faucet
csv wallet fund aptos      # Uses Aptos Testnet faucet
```

**Export wallet:**

```bash
csv wallet export bitcoin --format address
csv wallet export ethereum --format json
```

**Import wallet:**

```bash
csv wallet import bitcoin 0xabcdef...
csv wallet import ethereum "word1 word2 word3 ..."
```

**List all wallets:**

```bash
csv wallet list
```

---

### Sanad Operations

**Create a new Sanad:**

```bash
# Bitcoin Sanad with 100k sats value
csv sanad create --chain bitcoin --value 100000

# Ethereum Sanad (nullifier-based)
csv sanad create --chain ethereum

# Sui Sanad (object-based)
csv sanad create --chain sui

# Aptos Sanad (resource-based)
csv sanad create --chain aptos
```

**Show Sanad details:**

```bash
csv sanad show 0xabababababababababababababababababababababababababababababababab
```

**List tracked Sanads:**

```bash
csv sanad list
csv sanad list --chain bitcoin    # Filter by chain
```

**Consume a Sanad:**

```bash
csv sanad consume 0xabab...
```

---

### Proof Operations

**Generate inclusion proof:**

```bash
# Bitcoin Merkle proof
csv proof generate --chain bitcoin --sanad-id 0x... --output btc_proof.json

# Ethereum MPT proof
csv proof generate --chain ethereum --sanad-id 0x... --output eth_proof.json

# Sui checkpoint proof
csv proof generate --chain sui --sanad-id 0x... --output sui_proof.json

# Aptos ledger proof
csv proof generate --chain aptos --sanad-id 0x... --output apt_proof.json
```

**Verify proof on a chain:**

```bash
csv proof verify --chain sui --proof btc_proof.json
```

**Verify cross-chain proof:**

```bash
# Bitcoin proof verified on Sui
csv proof verify-cross-chain --source bitcoin --dest sui btc_proof.json

# Ethereum proof verified on Aptos
csv proof verify-cross-chain --source ethereum --dest aptos eth_proof.json
```

---

### Cross-Chain Transfers

**Execute a cross-chain transfer:**

```bash
# Bitcoin → Sui
csv cross-chain transfer --from bitcoin --to sui --sanad-id 0x...

# Sui → Ethereum
csv cross-chain transfer --from sui --to ethereum --sanad-id 0x...

# Bitcoin → Ethereum
csv cross-chain transfer --from bitcoin --to ethereum --sanad-id 0x...

# Ethereum → Sui
csv cross-chain transfer --from ethereum --to sui --sanad-id 0x...
```

**Check transfer status:**

```bash
csv cross-chain status 0xf378a3ffdaa7383a...
```

**List all transfers:**

```bash
csv cross-chain list
csv cross-chain list --from bitcoin
csv cross-chain list --to ethereum
```

---

### Contract Deployment

**Deploy contracts:**

```bash
# Sui Move package
csv contract deploy --chain sui

# Aptos Move package
csv contract deploy --chain aptos

# Ethereum Solidity contracts
csv contract deploy --chain ethereum
```

**Check deployment status:**

```bash
csv contract status bitcoin    # N/A — UTXO-native
csv contract status sui
csv contract status aptos
csv contract status ethereum
```

**Verify deployment:**

```bash
csv contract verify sui
csv contract verify ethereum
```

**List deployed contracts:**

```bash
csv contract list
```

---

### Seal Operations

**Create a seal:**

```bash
csv seal create --chain bitcoin --value 100000
csv seal create --chain ethereum
```

**Consume a seal:**

```bash
csv seal consume bitcoin 0x01...
```

**Verify seal status:**

```bash
csv seal verify bitcoin 0x01...
```

**List consumed seals:**

```bash
csv seal list
csv seal list --chain bitcoin
```

---

### End-to-End Testing

**Run cross-chain tests:**

```bash
# Default test: Bitcoin → Sui
csv test run

# Specific chain pair
csv test run --chain-pair bitcoin:sui

# All chain pairs
csv test run --all
```

**Run specific test scenarios:**

```bash
csv test scenario double_spend
csv test scenario invalid_proof
csv test scenario ownership_transfer
```

**View test results:**

```bash
csv test results
```

---

### Validation

**Validate a consignment:**

```bash
csv validate consignment consignment.json
```

**Validate a proof:**

```bash
csv validate proof proof.json --chain sui
```

**Validate seal consumption:**

```bash
csv validate seal 0x01...
```

**Validate commitment chain:**

```bash
csv validate commitment-chain commitments.json
```

---

## Configuration

The CLI uses `~/.csv/config.toml` by default. Create or edit it:

```toml
[chains.bitcoin]
rpc_url = "https://mempool.space/signet/api/"
network = "test"
finality_depth = 6
default_fee = 10

[chains.ethereum]
rpc_url = "https://rpc.sepolia.org"
network = "test"
chain_id = 11155111
finality_depth = 15
contract_address = null
default_fee = 20000000000

[chains.sui]
rpc_url = "https://fullnode.testnet.sui.io:443"
network = "test"
finality_depth = 1
default_fee = 1000

[chains.aptos]
rpc_url = "https://fullnode.testnet.aptoslabs.com/v1"
network = "test"
finality_depth = 1
default_fee = 100

[wallets]
# Populated automatically by `csv wallet generate`

[faucets]
# Default faucet URLs are pre-configured

[general]
data_dir = "~/.csv/data"
```

Use a custom config file:

```bash
csv --config /path/to/custom.toml chain list
```

---

## Data Storage

The CLI persists state to `~/.csv/data/state.json`:

```json
{
  "sanads": [],
  "transfers": [],
  "contracts": {},
  "addresses": {
    "bitcoin": "tb1p7xr...",
    "ethereum": "0x1234...",
    "solana": "HN7cAB...",
    "sui": "0x5678...",
    "aptos": "0x9abc..."
  },
  "consumed_seals": []
}
```

---

## Global Options

```
-v, --verbose          Enable debug logging
-c, --config <PATH>    Use custom config file
-h, --help             Print help
-V, --version          Print version
```

---

## Cross-Chain Transfer Flow

Here's the complete flow for transferring a Sanad from Bitcoin to Ethereum:

```bash
# Step 1: Generate wallets
csv wallet generate bitcoin test
csv wallet generate ethereum

# Step 2: Fund wallets
csv wallet fund bitcoin
csv wallet fund ethereum

# Step 3: Deploy Ethereum contracts
csv contract deploy --chain ethereum

# Step 4: Create Sanad on Bitcoin
csv sanad create --chain bitcoin --value 100000

# Step 5: Check balance to confirm funding
csv wallet balance bitcoin

# Step 6: Execute cross-chain transfer
csv cross-chain transfer --from bitcoin --to ethereum --sanad-id 0x...

# Step 7: Verify the transfer
csv cross-chain list
csv cross-chain status <transfer-id>

# Step 8: Validate the proof
csv proof generate --chain bitcoin --sanad-id 0x... --output proof.json
csv proof verify-cross-chain --source bitcoin --dest ethereum proof.json
```

---

## Troubleshooting

**"No address for bitcoin"** — Generate a wallet first:

```bash
csv wallet generate bitcoin test
```

**"Chain connection failed"** — Check RPC URL:

```bash
csv chain status bitcoin
csv chain set-rpc bitcoin <new-url>
```

**"Sanad not found"** — List tracked Sanads:

```bash
csv sanad list
```

**"Transfer not found"** — List all transfers:

```bash
csv cross-chain list
```

---

## License

MIT or Apache-2.0
