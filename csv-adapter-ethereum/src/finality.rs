//! Ethereum finality checker
//!
//! Checks if a transaction has achieved finality via:
//! 1. Post-merge finalized checkpoint (deterministic finality)
//! 2. Confirmation depth fallback (probabilistic finality)

use crate::rpc::EthereumRpc;

/// Finality configuration
#[derive(Clone, Debug)]
pub struct FinalityConfig {
    /// Required confirmation depth for probabilistic finality
    pub confirmation_depth: u64,
    /// Whether to prefer checkpoint finality over confirmations
    pub prefer_checkpoint_finality: bool,
}

impl Default for FinalityConfig {
    fn default() -> Self {
        Self {
            confirmation_depth: 15,
            prefer_checkpoint_finality: true,
        }
    }
}

/// Finality checker
pub struct FinalityChecker {
    config: FinalityConfig,
}

impl FinalityChecker {
    pub fn new(config: FinalityConfig) -> Self {
        Self { config }
    }

    /// Check if a block at the given number has achieved finality
    pub fn is_finalized(
        &self,
        block_number: u64,
        rpc: &dyn EthereumRpc,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Try checkpoint finality first if preferred
        if self.config.prefer_checkpoint_finality {
            if let Some(finalized) = rpc.get_finalized_block_number()? {
                if block_number <= finalized {
                    return Ok(true);
                }
                // Checkpoint didn't cover this block, fall through to confirmations
            }
        }

        // Fallback to confirmation depth
        let current = rpc.block_number()?;
        let confirmations = current.saturating_sub(block_number);
        Ok(confirmations >= self.config.confirmation_depth)
    }

    /// Get the number of confirmations for a block
    pub fn get_confirmations(
        &self,
        block_number: u64,
        rpc: &dyn EthereumRpc,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let current = rpc.block_number()?;
        Ok(current.saturating_sub(block_number))
    }

    /// Get detailed finality info
    pub fn get_finality_info(
        &self,
        block_number: u64,
        rpc: &dyn EthereumRpc,
    ) -> Result<FinalityInfo, Box<dyn std::error::Error + Send + Sync>> {
        let current = rpc.block_number()?;
        let confirmations = current.saturating_sub(block_number);
        let checkpoint_finalized = rpc.get_finalized_block_number()?.map(|f| block_number <= f);
        let is_final = checkpoint_finalized.unwrap_or(false)
            || confirmations >= self.config.confirmation_depth;

        Ok(FinalityInfo {
            current_block: current,
            block_number,
            confirmations,
            required_depth: self.config.confirmation_depth,
            checkpoint_finalized: checkpoint_finalized.unwrap_or(false),
            is_final,
        })
    }
}

/// Finality information
#[derive(Clone, Debug)]
pub struct FinalityInfo {
    pub current_block: u64,
    pub block_number: u64,
    pub confirmations: u64,
    pub required_depth: u64,
    pub checkpoint_finalized: bool,
    pub is_final: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::MockEthereumRpc;

    fn test_checker() -> FinalityChecker {
        FinalityChecker::new(FinalityConfig::default())
    }

    #[test]
    fn test_confirmations_below_depth() {
        let rpc = MockEthereumRpc::new(100);
        let checker = test_checker();
        // Block 95 has 5 confirmations, need 15
        assert!(!checker.is_finalized(95, &rpc).unwrap());
    }

    #[test]
    fn test_confirmations_at_depth() {
        let rpc = MockEthereumRpc::new(100);
        let mut config = FinalityConfig::default();
        config.confirmation_depth = 5;
        let checker = FinalityChecker::new(config);
        // Block 95 has 5 confirmations, need 5
        assert!(checker.is_finalized(95, &rpc).unwrap());
    }

    #[test]
    fn test_confirmations_above_depth() {
        let rpc = MockEthereumRpc::new(100);
        let checker = test_checker();
        // Block 80 has 20 confirmations, need 15
        assert!(checker.is_finalized(80, &rpc).unwrap());
    }

    #[test]
    fn test_checkpoint_finality() {
        let rpc = MockEthereumRpc::new(1000);
        let checker = test_checker();
        // Block 900 is at or before finalized block (936) → checkpoint finalized
        assert!(checker.is_finalized(900, &rpc).unwrap());
        // Block 990 is after checkpoint (936) and only 10 confirmations (< 15) → not finalized
        assert!(!checker.is_finalized(990, &rpc).unwrap());
    }

    #[test]
    fn test_get_confirmations() {
        let rpc = MockEthereumRpc::new(100);
        let checker = test_checker();
        assert_eq!(checker.get_confirmations(90, &rpc).unwrap(), 10);
    }

    #[test]
    fn test_finality_info() {
        let rpc = MockEthereumRpc::new(100);
        let checker = test_checker();
        let info = checker.get_finality_info(90, &rpc).unwrap();
        assert_eq!(info.current_block, 100);
        assert_eq!(info.block_number, 90);
        assert_eq!(info.confirmations, 10);
        assert_eq!(info.required_depth, 15);
        assert!(!info.is_final); // 10 < 15 and not checkpoint finalized
    }
}
