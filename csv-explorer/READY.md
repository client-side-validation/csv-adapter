# CSV Explorer - Test Environment Ready

## ✅ Setup Complete

The CSV Explorer test environment has been successfully configured with:

### Database Seeded with Test Data
Location: `data/explorer.db` (84KB)

**Data Summary:**
- ✅ 8 Rights across 5 chains (Bitcoin, Ethereum, Sui, Aptos, Solana)
- ✅ 7 Cross-chain transfers (4 completed, 1 in-progress, 1 pending, 1 failed)
- ✅ 11 Seals (7 available, 4 consumed)
- ✅ 11 Deployed contracts (8 active, 1 deprecated, 1 error)
- ✅ Sync progress for all 5 chains

### Built Binaries
All release binaries are compiled and ready:
- ✅ `target/release/csv-explorer-indexer`
- ✅ `target/release/csv-explorer-api`
- ✅ `target/release/csv-explorer-ui`

### Test Scripts Created
- ✅ `validate.sh` - Quick validation (2 min)
- ✅ `quick-test.sh` - Development testing (5 min)
- ✅ `test.sh` - Full test suite (10 min)

### Documentation Created
- ✅ `TESTING.md` - Comprehensive testing guide
- ✅ `TEST_SETUP.md` - Test environment details

## How to Test

### Option 1: Quick Validation
```bash
./validate.sh
```

### Option 2: Manual Testing

**Start API Server:**
```bash
./target/release/csv-explorer-api start
```

**Test Endpoints:**
```bash
# Health check
curl http://localhost:8080/health

# Get statistics
curl http://localhost:8080/api/v1/stats

# List rights
curl "http://localhost:8080/api/v1/rights?limit=5"

# List transfers  
curl "http://localhost:8080/api/v1/transfers?limit=5"

# List seals
curl "http://localhost:8080/api/v1/seals?limit=5"

# List contracts
curl "http://localhost:8080/api/v1/contracts?limit=5"

# Chain status
curl http://localhost:8080/api/v1/chains
```

**Start UI Server:**
```bash
./target/release/csv-explorer-ui serve
```
Then open: http://localhost:3000

### Option 3: Docker
```bash
docker compose up -d
```

## Test Data Highlights

### Multi-Chain Rights
```
Bitcoin:  2 rights (1 with 2 transfers, 1 fresh)
Ethereum: 2 rights (1 spent with 3 transfers, 1 with 1 transfer)
Sui:      2 rights (1 active, 1 pending)
Aptos:    1 right (1 transfer)
Solana:   1 right (fresh)
```

### Cross-Chain Transfers
```
BTC → ETH → SUI  (Multi-hop transfer)
ETH → APT → SOL → BTC  (Full circle transfer)
APT → ETH  (Return transfer)
ETH → SUI  (Failed transfer for error testing)
```

### Contract Deployments
```
Bitcoin:  Nullifier Registry + Right Registry (v1.0.0)
Ethereum: Nullifier Registry + Right Registry + Bridge (v1.1.0-1.2.0)
Sui:      Registry + Bridge (v1.0.0, 1 deprecated)
Aptos:    Registry + Nullifier (v1.0.0)
Solana:   Registry + Bridge (v1.0.0, 1 in error state)
```

## Available Wallets for Testing

The seeded data includes these wallet addresses:

**Bitcoin:**
- `bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh` (active, 2 transfers)
- `bc1q9h5yjq3gk2v7h8f9d0s1a2z3x4c5v6b7n8m9` (fresh)

**Ethereum:**
- `0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb` (active, 3 transfers, 1 right spent)
- `0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed` (active, 1 transfer)

**Sui:**
- `0x1234567890abcdef1234567890abcdef` (active)
- `0xabcdef1234567890abcdef1234567890` (pending)

**Aptos:**
- `0x1234567890abcdef` (active, 1 transfer)

**Solana:**
- `CsvPDA1234567890abcdef1234567890` (active)

## Testing Scenarios

### Scenario 1: View Right Lifecycle
1. Query right `right_btc_001`
2. View its 2 transfers (`xfer_001`, `xfer_002`)
3. Check associated seals (`seal_btc_001`)
4. Trace multi-chain journey: BTC → ETH → SUI

### Scenario 2: Monitor Transfer Status
1. Filter transfers by `completed` status (should show 4)
2. Filter by `in_progress` (should show 1)
3. Filter by `pending` (should show 1)
4. Filter by `failed` (should show 1)
5. Check transfer durations (3.5-4.75 hours average)

### Scenario 3: Chain Sync Verification
1. Check all chains show `synced` status
2. Verify sync lag is 0 for all chains
3. Confirm latest block numbers are realistic

### Scenario 4: Contract Health Check
1. List all 11 contracts
2. Identify 8 active, 1 deprecated, 1 error
3. Verify contract versions and deployment dates

## GraphQL Queries

```graphql
# Get statistics
query {
  stats {
    totalRights
    totalTransfers
    totalSeals
    totalContracts
  }
}

# List rights with filter
query {
  rights(filter: { chain: "bitcoin", limit: 5 }) {
    edges {
      node {
        id
        chain
        owner
        status
        transferCount
      }
    }
  }
}

# Get transfers by status
query {
  transfers(filter: { status: "completed" }) {
    edges {
      node {
        id
        fromChain
        toChain
        durationMs
      }
    }
  }
}
```

## Next Steps

1. **Test with UI:**
   ```bash
   ./target/release/csv-explorer-ui serve
   # Open http://localhost:3000
   ```

2. **Run Indexer with Live Data:**
   ```bash
   # Configure real RPC endpoints in config.toml
   ./target/release/csv-explorer-indexer start
   ```

3. **Deploy to Production:**
   ```bash
   docker compose up -d
   ```

## Troubleshooting

**API won't start:**
```bash
# Check port availability
lsof -i :8080

# Kill existing process
kill $(lsof -t -i:8080)

# Restart
./target/release/csv-explorer-api start
```

**Database issues:**
```bash
# Recreate from seed
rm data/explorer.db
sqlite3 data/explorer.db < storage/src/schema.sql
sqlite3 data/explorer.db < storage/src/seed.sql
```

**Build issues:**
```bash
cargo clean
cargo build --workspace --release
```

## Files Reference

```
csv-explorer/
├── data/
│   └── explorer.db              # Seeded SQLite database (84KB)
├── storage/src/
│   ├── schema.sql               # Database schema
│   └── seed.sql                 # Test data
├── target/release/
│   ├── csv-explorer-indexer     # Indexer binary
│   ├── csv-explorer-api         # API server binary
│   └── csv-explorer-ui          # UI server binary
├── validate.sh                  # Quick validation script
├── quick-test.sh                # Development test script
├── test.sh                      # Full test suite
├── TESTING.md                   # Testing guide
├── TEST_SETUP.md                # Test environment details
└── README.md                    # Main documentation
```
