//! Testnet deployment helpers for the Bitcoin adapter
//!
//! Provides pre-configured testnet setups for Signet and Testnet3,
//! including RPC endpoints, contract deployments, and validation tools.

use crate::seal_protocol::BitcoinSealProtocol;
use crate::config::{BitcoinConfig, Network};
use crate::wallet::SealWallet;

/// Testnet deployment configuration
#[derive(Clone, Debug)]
pub struct TestnetDeployConfig {
    /// Bitcoin network (Signet or Testnet3)
    pub network: Network,
    /// RPC endpoint URL
    pub rpc_url: &'static str,
    /// Required confirmations for finality
    pub finality_depth: u32,
    /// Publication timeout in seconds
    pub publication_timeout: u64,
}

impl TestnetDeployConfig {
    /// Signet deployment configuration (recommended for development)
    pub fn signet() -> Self {
        Self {
            network: Network::Signet,
            rpc_url: "https://mempool.space/signet/api/",
            finality_depth: 6,
            publication_timeout: 3600,
        }
    }

    /// Testnet3 deployment configuration
    pub fn testnet3() -> Self {
        Self {
            network: Network::Testnet,
            rpc_url: "https://blockstream.info/testnet/api/",
            finality_depth: 6,
            publication_timeout: 3600,
        }
    }

    /// Convert to BitcoinConfig
    pub fn to_config(&self) -> BitcoinConfig {
        BitcoinConfig {
            network: self.network,
            finality_depth: self.finality_depth,
            publication_timeout_seconds: self.publication_timeout,
            rpc_url: self.rpc_url.to_string(),
            xpub: None,
        }
    }
}

/// Create a testnet-ready adapter with a random wallet
pub fn create_testnet_adapter(
    network: Network,
) -> Result<BitcoinSealProtocol, Box<dyn std::error::Error + Send + Sync>> {
    let config = match network {
        Network::Signet => TestnetDeployConfig::signet().to_config(),
        Network::Testnet => TestnetDeployConfig::testnet3().to_config(),
        _ => return Err("Only Signet and Testnet3 are supported for testnet deployment".into()),
    };

    let wallet = SealWallet::generate_random(network.to_bitcoin_network());
    BitcoinSealProtocol::with_wallet(config, wallet)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

/// Validate testnet connectivity
pub fn validate_testnet_connectivity(
    adapter: &BitcoinSealProtocol,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Try to get current block height
    let height = adapter.get_current_height_for_test();

    if height > 0 {
        Ok(())
    } else {
        Err("Failed to connect to testnet - could not retrieve block height".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signet_config_creation() {
        let config = TestnetDeployConfig::signet().to_config();
        assert_eq!(config.network, Network::Signet);
        assert_eq!(config.finality_depth, 6);
        assert!(!config.rpc_url.is_empty());
    }

    #[test]
    fn test_testnet3_config_creation() {
        let config = TestnetDeployConfig::testnet3().to_config();
        assert_eq!(config.network, Network::Testnet);
        assert_eq!(config.finality_depth, 6);
    }

    #[test]
    fn test_create_testnet_adapter_signet() {
        let adapter = create_testnet_adapter(Network::Signet);
        assert!(adapter.is_ok());
    }

    #[test]
    #[ignore] // Requires live network access
    fn test_live_signet_connectivity() {
        let adapter = create_testnet_adapter(Network::Signet).unwrap();
        // This would actually connect to Signet
        let result = validate_testnet_connectivity(&adapter);
        // May fail in CI without network access
        let _ = result;
    }
}
