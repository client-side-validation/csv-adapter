# CSV Explorer - Network Guide

## Overview

CSV Explorer now supports **Mainnet** and **Testnet** with independent services for each network.

## Quick Start

### Start Mainnet
```bash
NETWORK=mainnet ./start.sh
```
- UI: http://localhost:3000
- API: http://localhost:8080
- Database: `data/mainnet.db`
- Config: `config.mainnet.toml`

### Start Testnet
```bash
NETWORK=testnet ./start.sh
```
- UI: http://localhost:3001
- API: http://localhost:8081
- Database: `data/testnet.db`
- Config: `config.testnet.toml`

### Start Both Networks
```bash
# Terminal 1 - Mainnet
NETWORK=mainnet ./start.sh

# Terminal 2 - Testnet
NETWORK=testnet ./start.sh
```

## UI Features

### Network Selector
- Located in the header (top-right)
- Dropdown to switch between Mainnet/Testnet
- Badge shows current network (blue for mainnet, purple for testnet)
- Clicking opens the selected network in a new tab

### External Explorer Links
All chain IDs link to the appropriate explorer based on network:

| Chain | Mainnet Explorer | Testnet Explorer |
|-------|-----------------|------------------|
| Bitcoin | mempool.space | mempool.space/testnet |
| Ethereum | etherscan.io | sepolia.etherscan.io |
| Sui | suiscan.xyz/mainnet | suiscan.xyz/testnet |
| Aptos | explorer.aptoslabs.com | explorer.aptoslabs.com (testnet) |
| Solana | solscan.io | solscan.io (devnet) |

## Monitoring

### Check Status
```bash
NETWORK=mainnet ./start.sh status
NETWORK=testnet ./start.sh status
```

### View Logs
```bash
# Mainnet
tail -f /tmp/csv-explorer-indexer-mainnet.log
tail -f /tmp/csv-explorer-api-mainnet.log
tail -f /tmp/csv-explorer-ui-mainnet.log

# Testnet
tail -f /tmp/csv-explorer-indexer-testnet.log
tail -f /tmp/csv-explorer-api-testnet.log
tail -f /tmp/csv-explorer-ui-testnet.log
```

### Stop Services
```bash
# Stop all services (both networks)
./stop.sh
```

## Configuration

### Mainnet Config (`config.mainnet.toml`)
- Bitcoin mainnet (mempool.space)
- Ethereum mainnet (llamarpc)
- Sui mainnet
- Aptos mainnet
- Solana mainnet-beta

### Testnet Config (`config.testnet.toml`)
- Bitcoin testnet (mempool.space/testnet)
- Ethereum Sepolia
- Sui testnet
- Aptos testnet
- Solana devnet

## Ports Summary

| Service | Mainnet | Testnet |
|---------|---------|---------|
| UI | 3000 | 3001 |
| API | 8080 | 8081 |
| GraphQL | 8080/graphql | 8081/graphql |
| Playground | 8080/playground | 8081/playground |

## API Endpoints

### Mainnet
- Health: http://localhost:8080/health
- Stats: http://localhost:8080/api/v1/stats
- Rights: http://localhost:8080/api/v1/rights
- Transfers: http://localhost:8080/api/v1/transfers
- Seals: http://localhost:8080/api/v1/seals

### Testnet
- Health: http://localhost:8081/health
- Stats: http://localhost:8081/api/v1/stats
- Rights: http://localhost:8081/api/v1/rights
- Transfers: http://localhost:8081/api/v1/transfers
- Seals: http://localhost:8081/api/v1/seals
