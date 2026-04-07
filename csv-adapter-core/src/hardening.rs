//! Production hardening utilities for core module
//!
//! This module provides:
//! - Bounded queues for rate limiting
//! - Circuit breakers for failure detection
//! - Timeout configuration
//! - Memory limits enforcement

use std::collections::VecDeque;
use std::time::Duration;

/// Maximum number of items in bounded queues
pub const MAX_SEAL_REGISTRY_SIZE: usize = 1000;

/// Maximum number of entries in caches
pub const MAX_CACHE_SIZE: usize = 1000;

/// Maximum number of entries in registries
pub const MAX_REGISTRY_SIZE: usize = 10000;

/// Default timeout for RPC calls
pub const DEFAULT_RPC_TIMEOUT: Duration = Duration::from_secs(30);

/// Default timeout for health checks
pub const DEFAULT_HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Default maximum failures before circuit opens
pub const DEFAULT_CIRCUIT_MAX_FAILURES: usize = 5;

/// Default reset timeout for circuit breaker
pub const DEFAULT_CIRCUIT_RESET_TIMEOUT: Duration = Duration::from_secs(60);

/// Bounded queue for seal registry operations
#[derive(Clone, Debug)]
pub struct BoundedQueue<T> {
    queue: VecDeque<T>,
    max_size: usize,
}

impl<T> BoundedQueue<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            max_size,
        }
    }

    pub fn push(&mut self, item: T) -> bool {
        if self.queue.len() >= self.max_size {
            return false;
        }
        self.queue.push_back(item);
        true
    }

    pub fn pop(&mut self) -> Option<T> {
        self.queue.pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.queue.len() >= self.max_size
    }
}

impl<T> Default for BoundedQueue<T> {
    fn default() -> Self {
        Self::new(MAX_SEAL_REGISTRY_SIZE)
    }
}

/// Circuit breaker state
#[derive(Clone, Debug, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker for failure detection
pub struct CircuitBreaker {
    failure_count: usize,
    max_failures: usize,
    last_failure_time: Option<std::time::SystemTime>,
    reset_timeout: Duration,
    state: CircuitState,
}

impl CircuitBreaker {
    pub fn new(max_failures: usize, reset_timeout: Duration) -> Self {
        Self {
            failure_count: 0,
            max_failures,
            last_failure_time: None,
            reset_timeout,
            state: CircuitState::Closed,
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(std::time::SystemTime::now());

        if self.failure_count >= self.max_failures {
            self.state = CircuitState::Open;
        }
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitState::Closed;
    }

    pub fn allow_request(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if last_failure.elapsed().unwrap_or_default() > self.reset_timeout {
                        self.state = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    self.state = CircuitState::HalfOpen;
                    true
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn state(&self) -> &CircuitState {
        &self.state
    }

    pub fn failure_count(&self) -> usize {
        self.failure_count
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(DEFAULT_CIRCUIT_MAX_FAILURES, DEFAULT_CIRCUIT_RESET_TIMEOUT)
    }
}

/// Timeout configuration
#[derive(Clone, Debug)]
pub struct TimeoutConfig {
    pub rpc_call: Duration,
    pub health_check: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            rpc_call: DEFAULT_RPC_TIMEOUT,
            health_check: DEFAULT_HEALTH_CHECK_TIMEOUT,
        }
    }
}

/// Memory limits configuration
#[derive(Clone, Debug)]
pub struct MemoryLimits {
    pub cache_size: usize,
    pub registry_size: usize,
}

impl Default for MemoryLimits {
    fn default() -> Self {
        Self {
            cache_size: MAX_CACHE_SIZE,
            registry_size: MAX_REGISTRY_SIZE,
        }
    }
}
