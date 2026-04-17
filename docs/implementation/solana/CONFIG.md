# Solana Configuration

This guide covers configuration options for the Solana CSV adapter.

## Configuration Options

### Network Settings

| Setting | Description | Default | Example |
|----------|-------------|---------|---------|
| `network` | Solana network to connect to | `devnet` | `mainnet`, `devnet`, `testnet`, `local` |
| `rpc_url` | Custom RPC endpoint | Network default | `https://api.mainnet-beta.solana.com` |
| `csv_program_id` | CSV program ID for operations | `CsvProgram11111111111111111111111111111111` | Custom program address |

### Connection Settings

| Setting | Description | Default | Example |
|----------|-------------|---------|---------|
| `timeout_seconds` | RPC timeout duration | `30` | `60` |
| `max_retries` | Maximum retry attempts | `3` | `5` |
| `commitment` | Transaction commitment level | `confirmed` | `finalized` |

## Configuration Files

### Environment Variables

```bash
# Network
export SOLANA_NETWORK=devnet
export SOLANA_RPC_URL=https://api.devnet.solana.com

# Program
export CSV_PROGRAM_ID=YourProgramIdHere

# Connection
export SOLANA_TIMEOUT=60
export SOLANA_MAX_RETRIES=5
```

### Configuration File

Create `~/.config/csv/solana.toml`:

```toml
[network]
type = "devnet"  # devnet, testnet, mainnet, local
rpc_url = "https://api.devnet.solana.com"

[program]
csv_program_id = "YourProgramIdHere"

[connection]
timeout_seconds = 30
max_retries = 3
commitment = "confirmed"
```

## Network Endpoints

### Mainnet
- **RPC**: `https://api.mainnet-beta.solana.com`
- **WebSocket**: `wss://api.mainnet-beta.solana.com`

### Devnet  
- **RPC**: `https://api.devnet.solana.com`
- **WebSocket**: `wss://api.devnet.solana.com`

### Testnet
- **RPC**: `https://api.testnet.solana.com`
- **WebSocket**: `wss://api.testnet.solana.com`

### Local
- **RPC**: `http://localhost:8899`
- **WebSocket**: `ws://localhost:8900`

## Program Deployment

### Development

```bash
# Build program
cargo build-sbf

# Deploy to devnet
solana program deploy --program-id dist/program.so --url https://api.devnet.solana.com

# Set program ID in config
solana config set --keypair ~/.config/solana/id.json --program-id YOUR_PROGRAM_ID
```

### Production

```bash
# Deploy to mainnet
solana program deploy --program-id dist/program.so --url https://api.mainnet-beta.solana.com

# Verify deployment
solana program show --program-id YOUR_PROGRAM_ID --url https://api.mainnet-beta.solana.com
```

## Security Considerations

### Key Management

- **Never commit private keys** to version control
- **Use hardware wallets** for production
- **Encrypt local key storage** with strong passwords
- **Rotate keys regularly** and invalidate old ones

### Program Security

- **Validate all inputs** before processing
- **Use proper access controls** for admin functions
- **Implement rate limiting** for public endpoints
- **Audit program code** regularly

### Network Security

- **Use HTTPS endpoints** in production
- **Verify RPC responses** to prevent MITM attacks
- **Implement proper error handling** for network failures
- **Use commitment levels** appropriate for security requirements

## Troubleshooting

### Common Issues

#### RPC Connection Failed

**Symptoms**: Unable to connect to Solana RPC

**Solutions**:
```bash
# Check network connectivity
curl -X POST https://api.devnet.solana.com -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getSlot"}'

# Verify WebSocket connection
wscat -c wss://api.devnet.solana.com

# Check with custom RPC
solana cluster-version --url https://backup-rpc.example.com
```

#### Transaction Not Found

**Symptoms**: `getTransaction` returns null or error

**Solutions**:
```bash
# Check transaction signature
solana confirm <SIGNATURE> --url https://api.devnet.solana.com

# Verify with different commitment
solana transaction confirm <SIGNATURE> --commitment confirmed

# Check if transaction was dropped
solana transaction show <SIGNATURE> --output json
```

#### Account Not Found

**Symptoms**: `getAccountInfo` returns null for known account

**Solutions**:
```bash
# Check account address format
solana address <ADDRESS>

# Verify on correct network
solana account <ADDRESS> --url https://api.devnet.solana.com

# Check if account was closed
solana account <ADDRESS> --output json --output json
```

## Performance Optimization

### Connection Pooling

```rust
use csv_adapter_solana::SolanaRpc;
use std::sync::Arc;

// Create connection pool
let rpc_pool = Arc::new(SolanaRpc::with_pool_size(10));
```

### Batch Operations

```rust
// Group multiple operations
let mut batch = client.transaction_batch();

// Add operations
batch.create_right(right1)?;
batch.create_right(right2)?;
batch.create_right(right3)?;

// Execute all at once
let results = batch.execute().await?;
```

### Caching

```rust
use std::time::Duration;

// Cache account states
let cached_client = SolanaRpc::with_cache(
    Duration::from_secs(300), // 5 minutes
    1000 // max cache size
);
```

## Monitoring

### Metrics to Track

- **RPC Response Time**: Average time per call
- **Transaction Success Rate**: Percentage of successful transactions  
- **Account State Changes**: Number of seal operations
- **Error Rates**: By error type and network
- **Connection Pool Usage**: Active vs idle connections

### Health Checks

```bash
# RPC endpoint health
curl -s https://api.devnet.solana.com | jq '.result'

# Cluster status
solana cluster-version

# Current slot
solana slot
```

---

*Last updated: April 2026*
