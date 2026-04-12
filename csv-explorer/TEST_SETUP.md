# CSV Explorer - Test Environment Setup

## What Was Created

### 1. Database Seed Data (`storage/src/seed.sql`)

A comprehensive dataset simulating a production CSV environment:

**Rights (8 total)**

- Multi-chain distribution: Bitcoin (2), Ethereum (2), Sui (2), Aptos (1), Solana (1)
- Various statuses: active (6), spent (1), pending (1)
- Different transfer counts: 0-3 transfers per right
- Realistic metadata with types and names

**Transfers (7 total)**

- Cross-chain routes: BTC→ETH→SUI, ETH→APT→SOL→BTC, APT→ETH, ETH→SUI
- Status distribution: completed (4), in_progress (1), pending (1), failed (1)
- Realistic durations: 3.5-4.75 hours for completed transfers
- Full lifecycle tracking with lock_tx, mint_tx, and proof references

**Seals (11 total)**

- All seal types represented: UTXO, Tapret, Account, Object, Resource, Nullifier
- Status split: available (7), consumed (4)
- Linked to specific rights and blocks
- Realistic block heights for each chain

**Contracts (11 total)**

- Deployed across all 5 chains
- Contract types: Nullifier Registry, Right Registry, Bridge
- Version tracking (1.0.0 - 1.2.0)
- Status monitoring: active (8), deprecated (1), error (1)

### 2. Test Scripts

**validate.sh** - Quick validation script

- Database creation and seeding
- Data validation
- API server startup
- Endpoint testing
- Automatic cleanup

**quick-test.sh** - Development testing

- Full build process
- Database setup
- Comprehensive API testing
- Server kept running for manual testing

**test.sh** - Full test suite

- All validation steps
- Indexer testing
- Database integrity checks
- Relationship validation
- Detailed reporting

### 3. Documentation

**TESTING.md** - Comprehensive testing guide

- Quick start instructions
- Test data description
- Manual testing procedures
- API endpoint examples
- GraphQL query examples
- Database query examples
- UI testing scenarios
- Troubleshooting guide
- Performance testing guidelines

## How to Use

### Quick Validation (2 minutes)

```bash
./validate.sh
```

### Development Testing (5 minutes)

```bash
./quick-test.sh
```

### Full Test Suite (10 minutes)

```bash
./test.sh
```

### Docker Deployment

```bash
docker compose up -d
docker compose logs -f
```

## Test Data Details

### Wallet Addresses Used

**Bitcoin Wallets:**

- `bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh`
- `bc1q9h5yjq3gk2v7h8f9d0s1a2z3x4c5v6b7n8m9`

**Ethereum Wallets:**

- `0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb`
- `0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed`

**Sui Wallets:**

- `0x1234567890abcdef1234567890abcdef`
- `0xabcdef1234567890abcdef1234567890`

**Aptos Wallets:**

- `0x1234567890abcdef`

**Solana Wallets:**

- `CsvPDA1234567890abcdef1234567890`

### Contract Addresses

All contracts have realistic deployment addresses for their respective chains, including:

- Bitcoin: Bech32 addresses
- Ethereum: 0x-prefixed addresses
- Sui: Module paths (0x123::CSVRegistry::Registry)
- Aptos: Resource paths (0x1::CSVRegistry)
- Solana: Base58 encoded addresses

### Multi-Chain Transfer Patterns

**Transfer 1: BTC → ETH**

- Lock: Bitcoin UTXO
- Mint: Ethereum account
- Duration: 4.25 hours
- Status: Completed

**Transfer 2: ETH → SUI**

- Lock: Ethereum account
- Status: In Progress (proof pending)

**Transfer 3: ETH → APT**

- Lock: Ethereum
- Mint: Aptos resource
- Duration: 4.75 hours
- Status: Completed

**Transfer 4: APT → SOL**

- Lock: Aptos resource
- Mint: Solana PDA
- Duration: 3.5 hours
- Status: Completed

**Transfer 5: SOL → BTC**

- Lock: Solana account
- Status: Pending (awaiting Bitcoin confirmation)

**Transfer 6: ETH → SUI (Failed)**

- Lock: Ethereum
- Status: Failed (error after 3.75 hours)

**Transfer 7: APT → ETH**

- Lock: Aptos
- Mint: Ethereum
- Duration: 3.5 hours
- Status: Completed

## API Endpoints Available

All endpoints work with the seeded data:

- `GET /health` - Health check
- `GET /api/v1/stats` - Aggregate statistics
- `GET /api/v1/rights` - List rights (with filtering)
- `GET /api/v1/rights/:id` - Get specific right
- `GET /api/v1/transfers` - List transfers (with filtering)
- `GET /api/v1/transfers/:id` - Get specific transfer
- `GET /api/v1/seals` - List seals (with filtering)
- `GET /api/v1/seals/:id` - Get specific seal
- `GET /api/v1/contracts` - List contracts (with filtering)
- `GET /api/v1/chains` - Chain indexer status
- `POST /graphql` - GraphQL queries

## Next Steps

After validating with test data:

1. **Configure Real RPC Endpoints**
   - Edit `config.toml` with production RPC URLs
   - Adjust start blocks for each chain

2. **Run Live Indexer**
   - Start indexer to populate with real data
   - Monitor sync progress

3. **Enable Wallet Integration**
   - Connect real wallets
   - Test live transfers

4. **Deploy to Production**
   - Use Docker Compose
   - Configure monitoring
   - Set up alerts

## Troubleshooting

If tests fail:

1. **Database Issues**

   ```bash
   rm -rf data/
   ./validate.sh
   ```

2. **Build Issues**

   ```bash
   cargo clean
   cargo build --workspace --release
   ```

3. **Port Conflicts**

   ```bash
   lsof -i :8080
   kill $(lsof -t -i:8080)
   ```

4. **Permission Issues**

   ```bash
   chmod +x *.sh
   ```
