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
use crate::runtime_mode::RuntimeMode;

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

    /// Current runtime mode
    pub mode: RuntimeMode,
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
            allow_rpc_fallback: false,
            max_retries: 3,
            retry_delay: Duration::from_secs(5),
            enforce_strict_finality: false,
            mode: RuntimeMode::Normal,
        }
    }

    /// Create a new runtime policy with a specific mode
    pub fn with_mode(mode: RuntimeMode) -> Self {
        let mut policy = Self::new();
        policy.set_mode(mode);
        policy
    }

    /// Set the runtime mode and update policy settings accordingly
    pub fn set_mode(&mut self, mode: RuntimeMode) {
        self.mode = mode;
        self.allow_rpc_fallback = mode.allows_rpc_fallback();
        self.enforce_strict_finality = mode.enforces_strict_finality();
        self.max_retries = mode.max_retries();
        self.retry_delay = mode.retry_delay();
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
        Self::with_mode(RuntimeMode::Normal)
    }

    /// Create a development policy (allows fallbacks for testing)
    pub fn development() -> Self {
        Self::with_mode(RuntimeMode::Degraded)
    }

    /// Create an unsafe policy (emergency mode, minimal checks)
    pub fn unsafe_mode() -> Self {
        Self::with_mode(RuntimeMode::Unsafe)
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
        assert_eq!(policy.mode, RuntimeMode::Normal);
        assert!(!policy.allow_rpc_fallback);
        assert!(policy.enforce_strict_finality);
    }

    #[test]
    fn test_development_policy() {
        let policy = RuntimePolicy::development();
        assert_eq!(policy.mode, RuntimeMode::Degraded);
        assert!(policy.allow_rpc_fallback);
        assert!(!policy.enforce_strict_finality);
    }

    #[test]
    fn test_unsafe_policy() {
        let policy = RuntimePolicy::unsafe_mode();
        assert_eq!(policy.mode, RuntimeMode::Unsafe);
        assert!(policy.allow_rpc_fallback);
        assert!(!policy.enforce_strict_finality);
        assert!(policy.mode.requires_operator_confirmation());
    }

    #[test]
    fn test_set_mode() {
        let mut policy = RuntimePolicy::new();
        assert_eq!(policy.mode, RuntimeMode::Normal);

        policy.set_mode(RuntimeMode::Degraded);
        assert_eq!(policy.mode, RuntimeMode::Degraded);
        assert!(policy.allow_rpc_fallback);
        assert_eq!(policy.max_retries, 5);

        policy.set_mode(RuntimeMode::Unsafe);
        assert_eq!(policy.mode, RuntimeMode::Unsafe);
        assert_eq!(policy.max_retries, 1);
    }
}
