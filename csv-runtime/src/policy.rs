//! Runtime Policy Module
//!
//! This module centralizes all policy decisions that were previously
//! made by individual adapters. The runtime is the single authority for:
//! - Finality depth requirements
//! - RPC fallback behavior
//! - Retry logic
//! - Confirmation interpretation
//!
//! Adapters MUST NOT make policy decisions. They only execute operations
//! according to runtime-provided policies.

use std::time::Duration;

/// Runtime policy configuration
///
/// All policy decisions for cross-chain transfers are centralized here.
/// Adapters receive policy via RuntimeExecutionContext and MUST NOT
/// override or ignore these policies.
#[derive(Debug, Clone)]
pub struct RuntimePolicy {
    /// Finality depth required for each chain
    pub finality_depths: std::collections::HashMap<String, u64>,

    /// Whether to allow RPC fallback to simulated mode
    pub allow_rpc_fallback: bool,

    /// Maximum number of retry attempts for transient failures
    pub max_retries: u32,

    /// Retry delay between attempts
    pub retry_delay: Duration,

    /// Whether to enforce strict finality (no probabilistic finality)
    pub enforce_strict_finality: bool,
}

impl RuntimePolicy {
    /// Create a new runtime policy with default values
    pub fn new() -> Self {
        let mut finality_depths = std::collections::HashMap::new();
        
        // Default finality depths per chain
        finality_depths.insert("bitcoin".to_string(), 6);
        finality_depths.insert("ethereum".to_string(), 15);
        finality_depths.insert("solana".to_string(), 32);
        finality_depths.insert("aptos".to_string(), 5);
        finality_depths.insert("sui".to_string(), 15);
        finality_depths.insert("celestia".to_string(), 100);

        Self {
            finality_depths,
            allow_rpc_fallback: false, // Production: no fallback
            max_retries: 3,
            retry_delay: Duration::from_secs(5),
            enforce_strict_finality: false, // Allow probabilistic finality
        }
    }

    /// Get the required finality depth for a chain
    pub fn finality_depth_for_chain(&self, chain_id: &str) -> Option<u64> {
        self.finality_depths.get(chain_id).copied()
    }

    /// Set the finality depth for a specific chain
    pub fn set_finality_depth(&mut self, chain_id: String, depth: u64) {
        self.finality_depths.insert(chain_id, depth);
    }

    /// Create a production policy (no fallbacks, strict enforcement)
    pub fn production() -> Self {
        let mut policy = Self::new();
        policy.allow_rpc_fallback = false;
        policy.enforce_strict_finality = true;
        policy
    }

    /// Create a development policy (allows fallbacks for testing)
    pub fn development() -> Self {
        let mut policy = Self::new();
        policy.allow_rpc_fallback = true;
        policy.enforce_strict_finality = false;
        policy
    }
}

impl Default for RuntimePolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_finality_depths() {
        let policy = RuntimePolicy::new();
        assert_eq!(policy.finality_depth_for_chain("bitcoin"), Some(6));
        assert_eq!(policy.finality_depth_for_chain("ethereum"), Some(15));
        assert_eq!(policy.finality_depth_for_chain("solana"), Some(32));
    }

    #[test]
    fn test_set_custom_finality_depth() {
        let mut policy = RuntimePolicy::new();
        policy.set_finality_depth("bitcoin".to_string(), 12);
        assert_eq!(policy.finality_depth_for_chain("bitcoin"), Some(12));
    }

    #[test]
    fn test_production_policy() {
        let policy = RuntimePolicy::production();
        assert!(!policy.allow_rpc_fallback);
        assert!(policy.enforce_strict_finality);
    }

    #[test]
    fn test_development_policy() {
        let policy = RuntimePolicy::development();
        assert!(policy.allow_rpc_fallback);
        assert!(!policy.enforce_strict_finality);
    }
}
