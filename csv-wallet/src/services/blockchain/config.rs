//! Blockchain service configuration.

/// Configuration for blockchain RPC endpoints.
#[derive(Clone, Debug)]
pub struct BlockchainConfig {
    pub ethereum_rpc: String,
    pub bitcoin_rpc: String,
    pub sui_rpc: String,
    pub aptos_rpc: String,
    pub solana_rpc: String,
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            ethereum_rpc: "https://ethereum-sepolia-rpc.publicnode.com".to_string(),
            bitcoin_rpc: "https://mempool.space/testnet/api".to_string(),
            sui_rpc: "https://fullnode.testnet.sui.io:443".to_string(),
            aptos_rpc: "https://fullnode.testnet.aptoslabs.com/v1".to_string(),
            solana_rpc: "https://api.devnet.solana.com".to_string(),
        }
    }
}
