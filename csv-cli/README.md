# CSV Adapter CLI

**Command-line tool for cross-chain Rights, proofs, wallets, and end-to-end testing.**

```
csv — CSV Adapter CLI v0.1.0
Cross-Chain Rights, Proofs, and Transfers
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
# 1. Generate wallets for all chains
csv wallet generate bitcoin test
csv wallet generate ethereum
csv wallet generate sui test
csv wallet generate aptos test

# 2. Fund wallets from testnet faucets
csv wallet fund bitcoin
csv wallet fund ethereum
csv wallet fund sui
csv wallet fund aptos

# 3. Check balances
csv wallet balance bitcoin
csv wallet balance ethereum
csv wallet balance sui
csv wallet balance aptos

# 4. Deploy contracts (not needed for Bitcoin — UTXO-native)
csv contract deploy --chain sui
csv contract deploy --chain ethereum

# 5. Create a Right on Bitcoin
csv right create --chain bitcoin --value 100000

# 6. Transfer it cross-chain to Sui
csv cross-chain transfer --from bitcoin --to sui --right-id 0x...

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

**Check chain status and connectivity:**
```bash
csv chain status bitcoin
csv chain status ethereum
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
```

**Check balance:**
```bash
csv wallet balance bitcoin
csv wallet balance --address tb1p7xr... bitcoin
csv wallet balance ethereum
csv wallet balance sui
csv wallet balance aptos
```

**Fund from faucet:**
```bash
csv wallet fund bitcoin    # Uses Signet faucet
csv wallet fund ethereum   # Uses Sepolia faucet
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

### Right Operations

**Create a new Right:**
```bash
# Bitcoin Right with 100k sats value
csv right create --chain bitcoin --value 100000

# Ethereum Right (nullifier-based)
csv right create --chain ethereum

# Sui Right (object-based)
csv right create --chain sui

# Aptos Right (resource-based)
csv right create --chain aptos
```

**Show Right details:**
```bash
csv right show 0xabababababababababababababababababababababababababababababababab
```

**List tracked Rights:**
```bash
csv right list
csv right list --chain bitcoin    # Filter by chain
```

**Consume a Right:**
```bash
csv right consume 0xabab...
```

---

### Proof Operations

**Generate inclusion proof:**
```bash
# Bitcoin Merkle proof
csv proof generate --chain bitcoin --right-id 0x... --output btc_proof.json

# Ethereum MPT proof
csv proof generate --chain ethereum --right-id 0x... --output eth_proof.json

# Sui checkpoint proof
csv proof generate --chain sui --right-id 0x... --output sui_proof.json

# Aptos ledger proof
csv proof generate --chain aptos --right-id 0x... --output apt_proof.json
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
csv cross-chain transfer --from bitcoin --to sui --right-id 0x...

# Sui → Ethereum
csv cross-chain transfer --from sui --to ethereum --right-id 0x...

# Bitcoin → Ethereum
csv cross-chain transfer --from bitcoin --to ethereum --right-id 0x...

# Ethereum → Sui
csv cross-chain transfer --from ethereum --to sui --right-id 0x...
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
  "rights": [],
  "transfers": [],
  "contracts": {},
  "addresses": {
    "bitcoin": "tb1p7xr...",
    "ethereum": "0x1234...",
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

Here's the complete flow for transferring a Right from Bitcoin to Ethereum:

```bash
# Step 1: Generate wallets
csv wallet generate bitcoin test
csv wallet generate ethereum

# Step 2: Fund wallets
csv wallet fund bitcoin
csv wallet fund ethereum

# Step 3: Deploy Ethereum contracts
csv contract deploy --chain ethereum

# Step 4: Create Right on Bitcoin
csv right create --chain bitcoin --value 100000

# Step 5: Check balance to confirm funding
csv wallet balance bitcoin

# Step 6: Execute cross-chain transfer
csv cross-chain transfer --from bitcoin --to ethereum --right-id 0x...

# Step 7: Verify the transfer
csv cross-chain list
csv cross-chain status <transfer-id>

# Step 8: Validate the proof
csv proof generate --chain bitcoin --right-id 0x... --output proof.json
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

**"Right not found"** — List tracked Rights:
```bash
csv right list
```

**"Transfer not found"** — List all transfers:
```bash
csv cross-chain list
```

---

## License

MIT or Apache-2.0
