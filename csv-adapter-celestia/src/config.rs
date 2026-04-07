//! Celestia adapter configuration

#[derive(Clone, Debug)]
pub struct CelestiaConfig {
    pub network: Network,
    pub finality_depth: u64,
    pub rpc_url: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Network { Mainnet, Testnet, Local }

impl CelestiaConfig {
    /// Validate configuration values
    pub fn validate(&self) -> Result<String, String> {
        if self.rpc_url.is_empty() {
            return Err("rpc_url cannot be empty".to_string());
        }
        if self.finality_depth == 0 {
            return Err("finality_depth must be greater than 0".to_string());
        }
        if self.finality_depth > 1000 {
            return Err("finality_depth must be <= 1000".to_string());
        }
        Ok("Configuration is valid".to_string())
    }
}

impl Default for CelestiaConfig {
    fn default() -> Self {
        Self {
            network: Network::Testnet,
            finality_depth: 6,
            rpc_url: "http://127.0.0.1:26658".to_string(),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default_config() {
        let config = CelestiaConfig::default();
        assert_eq!(config.finality_depth, 6);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_rpc_url() {
        let mut config = CelestiaConfig::default();
        config.rpc_url = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_finality_depth() {
        let mut config = CelestiaConfig::default();
        config.finality_depth = 0;
        assert!(config.validate().is_err());
    }

}
