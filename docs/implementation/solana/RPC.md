# Solana RPC Client

This guide covers the RPC client implementation for Solana CSV adapter.

## Overview

The Solana RPC client provides a robust interface for interacting with Solana blockchain, with support for both standard RPC endpoints and WebSocket connections.

## Architecture

### Components

- **SolanaRpc Trait**: Core interface for blockchain operations
- **RealSolanaRpc**: Production implementation with connection pooling
- **MockSolanaRpc**: Testing implementation with mock data

### Connection Management

- Connection pooling for performance
- Automatic failover between endpoints
- WebSocket support for real-time updates
- Timeout and retry logic

## API Reference

### Core Operations

```rust
use csv_adapter_solana::SolanaRpc;

// Get account information
let account = rpc.get_account(&pubkey).await?;

// Get multiple accounts
let accounts = rpc.get_multiple_accounts(&pubkeys).await?;

// Get transaction details
let tx = rpc.get_transaction(&signature).await?;

// Send transaction
let sig = rpc.send_transaction(&transaction).await?;

// Get latest slot
let slot = rpc.get_latest_slot().await?;
```

### Advanced Operations

```rust
// Get slot with specific commitment
let slot = rpc.get_slot_with_commitment(CommitmentConfig::finalized()).await?;

// Wait for transaction confirmation
let status = rpc.wait_for_confirmation(&signature).await?;

// Get account changes between slots
let changes = rpc.get_account_changes(from_slot, to_slot).await?;
```

## Configuration

### Connection Settings

```rust
use csv_adapter_solana::{SolanaRpc, RealSolanaRpc};

// Create with default settings
let rpc = RealSolanaRpc::new("https://api.devnet.solana.com", 30);

// Create with custom timeout
let rpc = RealSolanaRpc::with_timeout("https://api.mainnet-beta.solana.com", 60);

// Create with commitment
let rpc = RealSolanaRpc::with_commitment(
    "https://api.devnet.solana.com",
    CommitmentConfig::confirmed()
);
```

### Connection Pooling

```rust
use std::sync::Arc;
use csv_adapter_solana::SolanaRpcPool;

// Create connection pool
let pool = SolanaRpcPool::builder()
    .max_connections(10)
    .min_connections(2)
    .build();

// Get connection from pool
let rpc = pool.get_connection().await?;
```

## Error Handling

### Error Types

```rust
use csv_adapter_solana::{SolanaError, SolanaResult};

match result {
    Ok(data) => handle_success(data),
    Err(SolanaError::Rpc(msg)) => handle_rpc_error(msg),
    Err(SolanaError::Transaction(msg)) => handle_tx_error(msg),
    Err(SolanaError::Network(msg)) => handle_network_error(msg),
}
```

### Retry Logic

```rust
// Automatic retry with exponential backoff
let result = rpc
    .with_max_retries(5)
    .with_backoff(Duration::from_millis(1000))
    .get_account(&pubkey)
    .await;
```

## Performance Optimization

### Batching

```rust
// Batch multiple operations
let mut batch = rpc.transaction_batch();

batch.add_operation(create_right_op)?;
batch.add_operation(consume_seal_op)?;
batch.add_operation(transfer_right_op)?;

let results = batch.execute().await?;
```

### Caching

```rust
// Enable response caching
let rpc = RealSolanaRpc::with_cache(
    Duration::from_secs(300), // 5 minutes
    1000 // max cache size
);
```

### Concurrent Operations

```rust
use tokio::task::JoinSet;

// Parallel account queries
let mut tasks = JoinSet::new();

for pubkey in pubkeys {
    let task = tokio::spawn(async move {
        rpc.get_account(&pubkey).await
    });
    tasks.spawn(task);
}

let results = tasks.join_all().await;
```

## WebSocket Support

### Real-time Updates

```rust
use csv_adapter_solana::SolanaWebSocket;

// Subscribe to account changes
let mut ws = SolanaWebSocket::new("wss://api.devnet.solana.com");

ws.subscribe_account_changes(&pubkey, |change| {
    match change {
        AccountChange::Balance { old, new } => {
            println!("Balance changed: {} -> {}", old, new);
        }
        AccountChange::Data { old, new } => {
            println!("Data changed: {} -> {}", old, new);
        }
        AccountChange::Closed => {
            println!("Account closed");
        }
    }
}).await?;
```

### Slot Subscriptions

```rust
// Subscribe to slot updates
ws.subscribe_slots(|slot_info| {
    println!("New slot: {}", slot_info.slot);
    println!("Block hash: {}", slot_info.block_hash);
}).await?;
```

## Testing

### Mock Implementation

```rust
use csv_adapter_solana::MockSolanaRpc;

// Create mock with predefined data
let mut mock = MockSolanaRpc::new();

// Add test account
let test_account = AccountInfo {
    pubkey: "Test11111111111111111111111111111111".parse().unwrap(),
    lamports: 1000000000,
    data: vec![1, 2, 3],
    owner: "TestOwner11111111111111111111111111111111".parse().unwrap(),
};

mock.add_account(test_account);

// Use in tests
let rpc: Box<dyn SolanaRpc> = Box::new(mock);
let result = rpc.get_account(&test_account.pubkey).await?;
assert!(result.is_ok());
```

### Integration Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use csv_adapter_solana::MockSolanaRpc;

    #[tokio::test]
    async fn test_account_retrieval() {
        let mock = MockSolanaRpc::new();
        let test_pubkey = "Test11111111111111111111111111111111".parse().unwrap();
        
        let result = mock.get_account(&test_pubkey).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_transaction_sending() {
        let mock = MockSolanaRpc::new();
        let tx = create_test_transaction();
        
        let result = mock.send_transaction(&tx).await;
        assert!(result.is_ok());
    }
}
```

## Monitoring

### Metrics Collection

```rust
use csv_adapter_solana::SolanaMetrics;

// Create metrics collector
let metrics = SolanaMetrics::new();

// Track RPC calls
metrics.record_rpc_call("get_account", Duration::from_millis(150));

// Track errors
metrics.record_error("rpc_timeout", 1);

// Get statistics
let stats = metrics.get_statistics();
println!("Average RPC time: {}ms", stats.avg_response_time);
println!("Error rate: {:.2}%", stats.error_rate);
```

### Health Checks

```rust
// Health check implementation
async fn health_check(rpc: &dyn SolanaRpc) -> Result<HealthStatus, SolanaError> {
    let slot = rpc.get_latest_slot().await?;
    let health = HealthStatus {
        is_healthy: true,
        current_slot: slot,
        last_check: chrono::Utc::now(),
    };
    
    Ok(health)
}
```

## Security Considerations

### Input Validation

```rust
impl SolanaRpc for RealSolanaRpc {
    fn get_account(&self, pubkey: &Pubkey) -> SolanaResult<AccountInfo> {
        // Validate pubkey format
        if pubkey.to_string().len() != 44 {
            return Err(SolanaError::InvalidInput("Invalid public key format".to_string()));
        }
        
        // Continue with implementation
        self.get_account_internal(pubkey).await
    }
}
```

### Rate Limiting

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct RateLimitedRpc {
    inner: Box<dyn SolanaRpc>,
    requests: HashMap<String, Vec<Instant>>,
    max_requests_per_second: u32,
}

impl RateLimitedRpc {
    fn new(rpc: Box<dyn SolanaRpc>) -> Self {
        Self {
            inner: rpc,
            requests: HashMap::new(),
            max_requests_per_second: 10,
        }
    }
}
```

## Troubleshooting

### Common Issues

#### Connection Timeouts

**Problem**: RPC calls timing out after 30 seconds

**Solutions**:
```bash
# Check network connectivity
ping api.devnet.solana.com

# Increase timeout
export SOLANA_TIMEOUT=60

# Use WebSocket for real-time data
wscat -c wss://api.devnet.solana.com
```

#### Account Not Found

**Problem**: `getAccountInfo` returns null for known account

**Solutions**:
```bash
# Verify network
solana cluster-version

# Check on correct cluster
solana account <ADDRESS> --url https://api.mainnet-beta.solana.com

# Verify account format
solana address <ADDRESS>
```

#### Transaction Failures

**Problem**: Transactions failing with "insufficient funds" error

**Solutions**:
```bash
# Check account balance
solana balance <ADDRESS>

# Check recent transactions
solana history <ADDRESS> --limit 10

# Verify compute budget
solana confirm <SIGNATURE> --output json
```

---

*Last updated: April 2026*
