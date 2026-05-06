//! Gas/fee estimator for blockchain operations.
//!
//! Provides fee estimation for transactions across different chains.

use crate::services::blockchain::types::BlockchainError;
use csv_core::Chain;

/// Fee estimator for transactions.
pub struct FeeEstimator {
    client: reqwest::Client,
}

impl FeeEstimator {
    /// Create a new fee estimator.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Estimate fee for a transaction on the specified chain.
    pub async fn estimate_fee(
        &self,
        chain: Chain,
        _tx_size: usize,
        priority: FeePriority,
    ) -> Result<u64, BlockchainError> {
        web_sys::console::log_1(&format!("Estimating fee for {:?}", chain).into());

        match chain {
            Chain::Bitcoin => self.estimate_bitcoin_fee(priority).await,
            Chain::Ethereum => self.estimate_ethereum_fee(priority).await,
            Chain::Sui => self.estimate_sui_fee(priority).await,
            Chain::Aptos => self.estimate_aptos_fee(priority).await,
            Chain::Solana => self.estimate_solana_fee(priority).await,
            _ => Err(BlockchainError {
                message: format!("Fee estimation not supported for {:?}", chain),
                chain: Some(chain),
                code: None,
            }),
        }
    }

    /// Estimate Bitcoin fee (satoshis per byte).
    async fn estimate_bitcoin_fee(&self, priority: FeePriority) -> Result<u64, BlockchainError> {
        // mempool.space API provides fee estimates
        let sats_per_byte = match priority {
            FeePriority::Low => 1,
            FeePriority::Medium => 5,
            FeePriority::High => 20,
        };
        Ok(sats_per_byte)
    }

    /// Estimate Ethereum gas price (wei).
    async fn estimate_ethereum_fee(&self, priority: FeePriority) -> Result<u64, BlockchainError> {
        let gwei = match priority {
            FeePriority::Low => 10,
            FeePriority::Medium => 20,
            FeePriority::High => 50,
        };
        // Convert gwei to wei
        Ok(gwei * 1_000_000_000)
    }

    /// Estimate Sui gas price (MIST).
    async fn estimate_sui_fee(&self, priority: FeePriority) -> Result<u64, BlockchainError> {
        let gas_units = match priority {
            FeePriority::Low => 1_000,
            FeePriority::Medium => 2_000,
            FeePriority::High => 5_000,
        };
        // Reference gas price is typically 1000 MIST per unit
        Ok(gas_units * 1_000)
    }

    /// Estimate Aptos gas price (octas).
    async fn estimate_aptos_fee(&self, priority: FeePriority) -> Result<u64, BlockchainError> {
        let gas_units = match priority {
            FeePriority::Low => 500,
            FeePriority::Medium => 1_000,
            FeePriority::High => 2_000,
        };
        // Reference gas price
        Ok(gas_units * 100)
    }

    /// Estimate Solana fee (lamports).
    async fn estimate_solana_fee(&self, _priority: FeePriority) -> Result<u64, BlockchainError> {
        // Solana has fixed fees per signature
        Ok(5_000) // 0.000005 SOL per signature
    }
}

impl Default for FeeEstimator {
    fn default() -> Self {
        Self::new()
    }
}

/// Fee priority level.
#[derive(Debug, Clone, Copy)]
pub enum FeePriority {
    /// Low priority - slower confirmation.
    Low,
    /// Medium priority - standard confirmation.
    Medium,
    /// High priority - fast confirmation.
    High,
}

impl Default for FeePriority {
    fn default() -> Self {
        FeePriority::Medium
    }
}
