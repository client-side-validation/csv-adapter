# Production Hardening Implementation

## Implementation Plan

### 1. Rate Limiting
- Add bounded queue limits (max 1000 items) to seal registries
- Update all seal registry implementations across adapters

### 2. Mock Mode Guards
- Add `#[cfg(debug_assertions)]` attributes to mock RPC implementations
- Ensure mocks are only compiled in debug builds

### 3. Memory Limits
- Add maximum size constraints to caches (max 1000 entries)
- Add maximum size constraints to registries (max 10000 entries)

### 4. Timeout Configuration
- Add configurable timeouts with reasonable defaults
- RPC calls: 30s default
- Health checks: 5s default

### 5. Circuit Breakers
- Implement failure detection with configurable thresholds
- Max 5 failures before circuit opens
- 60s reset timeout

## Files to Modify

### Core (csv-adapter-core)
- Seal-related types and interfaces

### Bitcoin (csv-adapter-bitcoin)
- seal.rs - SealRegistry
- adapter.rs - Main adapter structure
- rpc.rs - RPC client

### Ethereum (csv-adapter-ethereum)
- seal.rs - Seal handling
- rpc.rs - RPC client
- adapter.rs - Main adapter structure

### Sui (csv-adapter-sui)
- seal.rs - Seal handling
- rpc.rs - RPC client
- adapter.rs - Main adapter structure

### Aptos (csv-adapter-aptos)
- seal.rs - Seal handling
- rpc.rs - RPC client
- adapter.rs - Main adapter structure

### Celestia (csv-adapter-celestia)
- seal.rs - Seal handling
- rpc.rs - RPC client
- adapter.rs - Main adapter structure

## Implementation Approach

### Rate Limiting Implementation
```rust
pub struct SealRegistry {
    used_seals: std::collections::HashSet<Vec<u8>>,
    max_size: usize,  // Add bounded limit
}

impl SealRegistry {
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            used_seals: std::collections::HashSet::new(),
            max_size,
        }
    }
    
    pub fn mark_seal_used(&mut self, seal: &BitcoinSealRef) -> BitcoinResult<()> {
        // Check max size before inserting
        if self.used_seals.len() >= self.max_size {
            return Err(BitcoinError::RegistryFull);
        }
        // ... rest of implementation
    }
}
```

### Circuit Breaker Implementation
```rust
pub struct CircuitBreaker {
    failure_count: usize,
    max_failures: usize,
    last_failure_time: Option<SystemTime>,
    reset_timeout: Duration,
    state: CircuitState,
}

enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}
```

### Timeout Configuration
```rust
pub struct TimeoutConfig {
    pub rpc_call: Duration,
    pub health_check: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            rpc_call: Duration::from_secs(30),
            health_check: Duration::from_secs(5),
        }
    }
}
```

### Memory Limits
```rust
pub struct Cache<T> {
    entries: HashMap<Key, T>,
    max_size: usize,
}
```
