# CSV Explorer Testing Guide

This guide covers testing the CSV Explorer with pre-seeded data and deployed contracts.

## Quick Start

### Option 1: Quick Test (Recommended for Development)

```bash
# Run the quick test
./quick-test.sh
```

This will:

- Create and seed the SQLite database with test data
- Build the project (if needed)
- Start the API server
- Run basic API tests
- Keep the server running for manual testing

### Option 2: Full Test Suite

```bash
# Run comprehensive tests
./test.sh
```

This runs all quick tests plus:

- Indexer startup test
- Database integrity checks
- Detailed relationship validation

### Option 3: Docker

```bash
# Start all services via Docker
docker compose up -d

# Check logs
docker compose logs -f api
docker compose logs -f indexer
docker compose logs -f ui
```

## Test Data

The seed data (`storage/src/seed.sql`) includes:

### Rights (8 total)

- **Bitcoin**: 2 rights (1 active with 2 transfers, 1 active with 0 transfers)
- **Ethereum**: 2 rights (1 spent with 3 transfers, 1 active with 1 transfer)
- **Sui**: 2 rights (1 active, 1 pending)
- **Aptos**: 1 right (active with 1 transfer)
- **Solana**: 1 right (active)

### Transfers (7 total)

- **Completed**: 4 transfers (BTC→ETH, ETH→APT, APT→SOL, APT→ETH)
- **In Progress**: 1 transfer (ETH→SUI)
- **Pending**: 1 transfer (SOL→BTC)
- **Failed**: 1 transfer (ETH→SUI)

### Seals (11 total)

- **Available**: 7 seals across all chains
- **Consumed**: 4 seals (linked to completed transfers)
- Types: UTXO, Tapret, Account, Object, Resource, Nullifier

### Contracts (11 total)

- Deployed across all 5 chains
- Types: Nullifier Registry, Right Registry, Bridge
- Status: Active (8), Deprecated (1), Error (1)

## Manual Testing

### API Endpoints

After starting the API server, test these endpoints:

```bash
# Health check
curl http://localhost:8080/health

# Get statistics
curl http://localhost:8080/api/v1/stats

# List rights with pagination
curl "http://localhost:8080/api/v1/rights?limit=10&offset=0"

# Filter rights by chain
curl "http://localhost:8080/api/v1/rights?chain=bitcoin"

# Filter rights by status
curl "http://localhost:8080/api/v1/rights?status=active"

# Get single right
curl http://localhost:8080/api/v1/rights/right_btc_001

# List transfers
curl "http://localhost:8080/api/v1/transfers?limit=10"

# Filter transfers by status
curl "http://localhost:8080/api/v1/transfers?status=completed"

# Get single transfer
curl http://localhost:8080/api/v1/transfers/xfer_001

# List seals
curl "http://localhost:8080/api/v1/seals?limit=10"

# Filter seals by chain
curl "http://localhost:8080/api/v1/seals?chain=ethereum"

# List contracts
curl "http://localhost:8080/api/v1/contracts?limit=10"

# Get chain status
curl http://localhost:8080/api/v1/chains
```

### GraphQL Queries

```bash
# Get aggregate statistics
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ stats { totalRights totalTransfers totalSeals totalContracts } }"}'

# Query rights with pagination
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ rights(filter: {limit: 5}) { edges { node { id chain owner status } } pageInfo { hasNextPage } } }"}'

# Query transfers by status
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ transfers(filter: {status: COMPLETED}) { edges { node { id fromChain toChain status } } } }"}'
```

### Database Queries

Direct SQLite access for debugging:

```bash
# Open database
sqlite3 data/explorer.db

# Useful queries
SELECT chain, status, COUNT(*) FROM rights GROUP BY chain, status;
SELECT from_chain, to_chain, status, COUNT(*) FROM transfers GROUP BY from_chain, to_chain, status;
SELECT chain, seal_type, status, COUNT(*) FROM seals GROUP BY chain, seal_type, status;
SELECT * FROM sync_progress;
```

## Testing the UI

### Web Mode

```bash
# Start UI server
cargo run -p csv-explorer-ui -- serve

# Open browser
open http://localhost:3000
```

### Desktop Mode

```bash
# Launch desktop app
cargo run -p csv-explorer-ui -- desktop
```

### UI Pages to Test

1. **Home** (`/`) - Dashboard with stats and chain status
2. **Rights** (`/rights`) - List of all rights with filtering
3. **Right Detail** (`/rights/:id`) - Detailed view of a right
4. **Transfers** (`/transfers`) - Cross-chain transfer list
5. **Transfer Detail** (`/transfers/:id`) - Transfer progress timeline
6. **Seals** (`/seals`) - Seal inventory
7. **Seal Detail** (`/seals/:id`) - Seal information
8. **Contracts** (`/contracts`) - Deployed CSV contracts
9. **Chains** (`/chains`) - Chain indexer status
10. **Stats** (`/stats`) - Aggregate statistics
11. **Wallet** (`/wallet`) - Wallet connection

## Test Scenarios

### Scenario 1: Multi-Chain Right Lifecycle

1. View `right_btc_001` - Bitcoin origin right
2. Check its transfer history (should show 2 transfers)
3. View transfer `xfer_001` - BTC→ETH completed transfer
4. View transfer `xfer_002` - ETH→SUI in-progress transfer
5. Check associated seals for each chain

### Scenario 2: Cross-Chain Bridge Testing

1. Filter transfers by status: `completed`
2. Verify 4 completed transfers exist
3. Check transfer durations (should be 3.5-4.75 hours)
4. View transfer timelines for completion status

### Scenario 3: Chain Status Monitoring

1. Visit `/chains` to see all chain indexer statuses
2. Verify sync lag is 0 for all chains (seeded data)
3. Check latest block numbers match seed data

### Scenario 4: Contract Deployment Verification

1. Visit `/contracts` to see all deployed contracts
2. Verify 11 contracts across 5 chains
3. Check contract types and versions
4. Note 1 deprecated and 1 error status contract

## Troubleshooting

### API Won't Start

```bash
# Check if port is in use
lsof -i :8080

# Kill existing process
kill $(lsof -t -i:8080)

# Check database permissions
ls -la data/explorer.db
```

### Database Errors

```bash
# Recreate database
rm data/explorer.db
sqlite3 data/explorer.db < storage/src/schema.sql
sqlite3 data/explorer.db < storage/src/seed.sql
```

### Build Failures

```bash
# Clean and rebuild
cargo clean
cargo build --workspace --release
```

### UI Not Loading

```bash
# Check API is running
curl http://localhost:8080/health

# Check UI logs
cargo run -p csv-explorer-ui -- serve 2>&1 | tail -n 50
```

## Performance Testing

### Load Testing

```bash
# Parallel requests
for i in {1..100}; do
  curl -s http://localhost:8080/api/v1/rights > /dev/null &
done
wait

# Monitor response times
time curl http://localhost:8080/api/v1/rights?limit=1000
```

### Database Performance

```sql
-- Check indexes
SELECT name, tbl_name FROM sqlite_master WHERE type='index';

-- Query performance
EXPLAIN QUERY PLAN SELECT * FROM rights WHERE chain='bitcoin' AND status='active';
```

## Continuous Testing

### Run Tests Periodically

```bash
# Every 5 minutes
*/5 * * * * /path/to/csv-explorer/quick-test.sh >> /var/log/csv-explorer-test.log 2>&1
```

### Docker Auto-Test

```bash
# Restart and test
docker compose down
docker compose up -d --build
docker compose logs -f api | grep -A 5 "Test"
```

## Next Steps

After testing:

1. Replace seeded data with live indexer data
2. Configure real RPC endpoints in `config.toml`
3. Enable wallet integration
4. Set up monitoring and alerting
5. Deploy to production environment
