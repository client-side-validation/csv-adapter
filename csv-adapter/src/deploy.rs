//! Contract deployment manager for CSV adapters.
//!
//! This module provides a unified interface for deploying CSV contracts
//! across all supported blockchains using their respective SDKs.
//!
//! # Example
//!
//! ```no_run
//! use csv_adapter::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let client = CsvClient::builder()
//!         .with_chain(Chain::Ethereum)
//!         .build()?;
//!
//!     let deployment = client.deploy()
//!         .deploy_csv_lock(&rpc_url, &private_key, &bytecode)
//!         .await?;
//!
//!     println!("Contract deployed at: {:?}", deployment.contract_address);
//!     Ok(())
//! }
//! ```

use std::sync::Arc;

use crate::client::ClientRef;
use crate::errors::CsvError;
use csv_adapter_core::Chain;

/// Result type for deployment operations.
pub type DeploymentResult<T> = Result<T, DeploymentError>;

/// Error type for deployment operations.
#[derive(Debug, thiserror::Error)]
pub enum DeploymentError {
    /// Ethereum deployment error.
    #[cfg(feature = "ethereum")]
    #[error("Ethereum deployment error: {0}")]
    Ethereum(#[from] csv_adapter_ethereum::EthereumError),

    /// Sui deployment error.
    #[cfg(feature = "sui")]
    #[error("Sui deployment error: {0}")]
    Sui(#[from] csv_adapter_sui::SuiError),

    /// Aptos deployment error.
    #[cfg(feature = "aptos")]
    #[error("Aptos deployment error: {0}")]
    Aptos(#[from] csv_adapter_aptos::AptosError),

    /// Solana deployment error.
    #[cfg(feature = "solana")]
    #[error("Solana deployment error: {0}")]
    Solana(#[from] csv_adapter_solana::SolanaError),

    /// Chain not enabled.
    #[error("Chain {0} is not enabled for deployment")]
    ChainNotEnabled(String),

    /// Feature not enabled.
    #[error("Deployment feature not enabled for {0}")]
    FeatureNotEnabled(String),

    /// Generic error.
    #[error("Deployment error: {0}")]
    Generic(String),
}

impl From<DeploymentError> for CsvError {
    fn from(err: DeploymentError) -> Self {
        CsvError::DeploymentError(err.to_string())
    }
}

/// Unified contract deployment result.
#[derive(Debug, Clone)]
pub struct ContractDeployment {
    /// Chain where the contract was deployed.
    pub chain: Chain,
    /// Contract/program/package address.
    pub address: Vec<u8>,
    /// Transaction hash/digest.
    pub transaction_hash: String,
    /// Gas/fee used.
    pub gas_used: u64,
    /// Contract name/type.
    pub contract_type: String,
    /// Block number/slot where deployed.
    pub block_number: Option<u64>,
}

/// Deployment manager for CSV contracts.
///
/// Provides a unified interface for deploying CSV seal contracts
/// across all supported blockchains.
pub struct DeploymentManager {
    client_ref: Arc<ClientRef>,
}

impl DeploymentManager {
    /// Create a new deployment manager.
    pub(crate) fn new(client_ref: Arc<ClientRef>) -> Self {
        Self { client_ref }
    }

    /// Check if a chain is enabled for deployment.
    pub fn is_chain_enabled(&self, chain: Chain) -> bool {
        self.client_ref.is_chain_enabled(chain)
    }

    // =========================================================================
    // Ethereum Deployment
    // =========================================================================

    /// Deploy CSV Lock contract on Ethereum.
    ///
    /// # Arguments
    /// * `rpc_url` - Ethereum RPC endpoint
    /// * `private_key_hex` - Deployer private key (hex, with or without 0x)
    /// * `bytecode` - Compiled contract bytecode
    ///
    /// # Returns
    /// Deployment result with contract address.
    #[cfg(feature = "deploy-ethereum")]
    pub async fn deploy_csv_lock(
        &self,
        rpc_url: &str,
        private_key_hex: &str,
        bytecode: &[u8],
    ) -> DeploymentResult<ContractDeployment> {
        use csv_adapter_ethereum::deploy::deploy_csv_lock as eth_deploy;

        let result = eth_deploy(rpc_url, private_key_hex, bytecode).await?;

        Ok(ContractDeployment {
            chain: Chain::Ethereum,
            address: result.contract_address.to_vec(),
            transaction_hash: result.transaction_hash,
            gas_used: result.gas_used,
            contract_type: "CsvLock".to_string(),
            block_number: Some(result.block_number),
        })
    }

    /// Deploy CSV Seal contract on Ethereum.
    #[cfg(feature = "deploy-ethereum")]
    pub async fn deploy_csv_seal_contract(
        &self,
        rpc_url: &str,
        private_key_hex: &str,
    ) -> DeploymentResult<ContractDeployment> {
        use csv_adapter_ethereum::deploy::deploy_csv_seal_contract as eth_deploy;

        let result = eth_deploy(rpc_url, private_key_hex).await?;

        Ok(ContractDeployment {
            chain: Chain::Ethereum,
            address: result.contract_address.to_vec(),
            transaction_hash: result.transaction_hash,
            gas_used: result.gas_used,
            contract_type: "CsvSeal".to_string(),
            block_number: Some(result.block_number),
        })
    }

    /// Placeholder for when ethereum feature is not enabled.
    #[cfg(not(feature = "deploy-ethereum"))]
    pub async fn deploy_csv_lock(
        &self,
        _rpc_url: &str,
        _private_key_hex: &str,
        _bytecode: &[u8],
    ) -> DeploymentResult<ContractDeployment> {
        Err(DeploymentError::FeatureNotEnabled("Ethereum".to_string()))
    }

    /// Placeholder for when ethereum feature is not enabled.
    #[cfg(not(feature = "deploy-ethereum"))]
    pub async fn deploy_csv_seal_contract(
        &self,
        _rpc_url: &str,
        _private_key_hex: &str,
    ) -> DeploymentResult<ContractDeployment> {
        Err(DeploymentError::FeatureNotEnabled("Ethereum".to_string()))
    }

    // =========================================================================
    // Sui Deployment
    // =========================================================================

    /// Publish CSV package on Sui.
    ///
    /// # Arguments
    /// * `rpc_url` - Sui RPC endpoint
    /// * `compiled_modules` - Pre-compiled Move bytecode modules
    /// * `signer_address` - Address of the signer
    ///
    /// # Returns
    /// Deployment result with package ID.
    #[cfg(feature = "deploy-sui")]
    pub async fn publish_csv_package(
        &self,
        rpc_url: &str,
        compiled_modules: Vec<Vec<u8>>,
        signer_address: &str,
    ) -> DeploymentResult<ContractDeployment> {
        use csv_adapter_sui::deploy::publish_csv_package as sui_publish;

        let result = sui_publish(rpc_url, compiled_modules, signer_address).await?;

        Ok(ContractDeployment {
            chain: Chain::Sui,
            address: result.package_id.to_vec(),
            transaction_hash: result.transaction_digest,
            gas_used: result.gas_used,
            contract_type: "CsvPackage".to_string(),
            block_number: None,
        })
    }

    /// Deploy CSV seal package on Sui.
    #[cfg(feature = "sui")]
    pub async fn deploy_csv_seal_package(
        &self,
        config: &csv_adapter_sui::config::SuiConfig,
        rpc: Box<dyn csv_adapter_sui::rpc::SuiRpc>,
        package_bytes: &[u8],
        gas_budget: u64,
    ) -> DeploymentResult<ContractDeployment> {
        use csv_adapter_sui::deploy::deploy_csv_seal_package as sui_deploy;

        let result = sui_deploy(config, rpc, package_bytes, gas_budget).await?;

        Ok(ContractDeployment {
            chain: Chain::Sui,
            address: result.package_id.to_vec(),
            transaction_hash: result.transaction_digest,
            gas_used: result.gas_used,
            contract_type: "CsvSealPackage".to_string(),
            block_number: None,
        })
    }

    /// Placeholder for when sui-sdk-deploy feature is not enabled.
    #[cfg(not(feature = "deploy-sui"))]
    pub async fn publish_csv_package(
        &self,
        _rpc_url: &str,
        _compiled_modules: Vec<Vec<u8>>,
        _signer_address: &str,
    ) -> DeploymentResult<ContractDeployment> {
        Err(DeploymentError::FeatureNotEnabled("Sui SDK".to_string()))
    }

    // =========================================================================
    // Aptos Deployment
    // =========================================================================

    /// Publish CSV module on Aptos.
    ///
    /// # Arguments
    /// * `rpc_url` - Aptos REST endpoint
    /// * `module_bytes` - Compiled Move module bytecode
    /// * `signer` - LocalAccount with private key
    ///
    /// # Returns
    /// Deployment result with module address.
    #[cfg(feature = "deploy-aptos")]
    pub async fn publish_csv_module(
        &self,
        rpc_url: &str,
        module_bytes: Vec<u8>,
        signer: &aptos_sdk::types::LocalAccount,
    ) -> DeploymentResult<ContractDeployment> {
        use csv_adapter_aptos::deploy::publish_csv_module as aptos_publish;

        let result = aptos_publish(rpc_url, module_bytes, signer).await?;

        Ok(ContractDeployment {
            chain: Chain::Aptos,
            address: result.account_address.to_vec(),
            transaction_hash: result.transaction_hash,
            gas_used: result.gas_used,
            contract_type: "CsvModule".to_string(),
            block_number: Some(result.version),
        })
    }

    /// Deploy CSV seal module on Aptos.
    #[cfg(feature = "aptos")]
    pub async fn deploy_csv_seal_module(
        &self,
        config: &csv_adapter_aptos::config::AptosConfig,
        signing_key: ed25519_dalek::SigningKey,
        rpc: Box<dyn csv_adapter_aptos::rpc::AptosRpc>,
        module_bytes: &[u8],
    ) -> DeploymentResult<ContractDeployment> {
        use csv_adapter_aptos::deploy::deploy_csv_seal_module as aptos_deploy;

        let result = aptos_deploy(config, signing_key, rpc, module_bytes).await?;

        Ok(ContractDeployment {
            chain: Chain::Aptos,
            address: result.account_address.to_vec(),
            transaction_hash: result.transaction_hash,
            gas_used: result.gas_used,
            contract_type: "CsvSealModule".to_string(),
            block_number: Some(result.version),
        })
    }

    /// Placeholder for when aptos-sdk feature is not enabled.
    #[cfg(not(feature = "deploy-aptos"))]
    pub async fn publish_csv_module(
        &self,
        _rpc_url: &str,
        _module_bytes: Vec<u8>,
        _signer: &(), // Placeholder type
    ) -> DeploymentResult<ContractDeployment> {
        Err(DeploymentError::FeatureNotEnabled("Aptos SDK".to_string()))
    }

    // =========================================================================
    // Solana Deployment
    // =========================================================================

    /// Deploy CSV program on Solana.
    ///
    /// # Arguments
    /// * `rpc_url` - Solana RPC endpoint
    /// * `program_keypair` - Keypair for the program account
    /// * `program_data` - Compiled BPF program bytes
    /// * `payer` - Keypair with funds for deployment
    ///
    /// # Returns
    /// Deployment result with program ID.
    #[cfg(feature = "deploy-solana")]
    pub async fn deploy_csv_program(
        &self,
        rpc_url: &str,
        program_keypair: &solana_sdk::signature::Keypair,
        program_data: &[u8],
        payer: &solana_sdk::signature::Keypair,
    ) -> DeploymentResult<ContractDeployment> {
        use csv_adapter_solana::deploy::deploy_csv_program as solana_deploy;
        use solana_sdk::signature::Signature;

        let result = solana_deploy(rpc_url, program_keypair, program_data, payer).await?;

        Ok(ContractDeployment {
            chain: Chain::Solana,
            address: result.program_id.to_bytes().to_vec(),
            transaction_hash: result.signature.to_string(),
            gas_used: 0, // Would get from transaction
            contract_type: "CsvProgram".to_string(),
            block_number: Some(result.slot),
        })
    }

    /// Deploy CSV seal program on Solana.
    #[cfg(feature = "solana")]
    pub async fn deploy_csv_seal_program(
        &self,
        config: &csv_adapter_solana::config::SolanaConfig,
        wallet: csv_adapter_solana::wallet::ProgramWallet,
        rpc: Box<dyn csv_adapter_solana::rpc::SolanaRpc>,
        program_data: &[u8],
    ) -> DeploymentResult<ContractDeployment> {
        use csv_adapter_solana::deploy::deploy_csv_seal_program as solana_deploy;

        let result = solana_deploy(config, wallet, rpc, program_data).await?;

        Ok(ContractDeployment {
            chain: Chain::Solana,
            address: result.program_id.to_bytes().to_vec(),
            transaction_hash: result.signature.to_string(),
            gas_used: 0,
            contract_type: "CsvSealProgram".to_string(),
            block_number: Some(result.slot),
        })
    }

    /// Placeholder for when solana feature is not enabled.
    #[cfg(not(feature = "deploy-solana"))]
    pub async fn deploy_csv_program(
        &self,
        _rpc_url: &str,
        _program_keypair: &(),
        _program_data: &[u8],
        _payer: &(),
    ) -> DeploymentResult<ContractDeployment> {
        Err(DeploymentError::FeatureNotEnabled("Solana".to_string()))
    }

    // =========================================================================
    // Generic Deployment Helpers
    // =========================================================================

    /// Verify a deployment on any chain.
    ///
    /// This checks if the deployed contract/program exists on-chain.
    pub async fn verify_deployment(
        &self,
        _chain: Chain,
        _address: &[u8],
    ) -> DeploymentResult<bool> {
        // TODO: Implement verification using chain adapters
        // This would use the adapter's RPC client to check if the
        // contract/program exists on-chain
        Err(DeploymentError::Generic(
            "Verification not yet implemented".to_string(),
        ))
    }

    /// Get deployment cost estimate.
    ///
    /// Returns an estimated cost for deploying a contract of the given size.
    pub async fn estimate_deployment_cost(
        &self,
        chain: Chain,
        contract_size: usize,
    ) -> DeploymentResult<u64> {
        // TODO: Use adapter-specific estimation when available
        // For now, provide rough estimates
        let base_cost = match chain {
            Chain::Ethereum => contract_size as u64 * 200, // Gas units
            Chain::Sui => 10_000_000,                      // MIST
            Chain::Aptos => contract_size as u64 * 100,    // Gas units
            Chain::Solana => contract_size as u64 * 1000,    // Lamports
            _ => contract_size as u64 * 100,
        };

        Ok(base_cost)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_error_display() {
        let err = DeploymentError::Generic("test error".to_string());
        assert_eq!(err.to_string(), "Deployment error: test error");

        let err = DeploymentError::FeatureNotEnabled("Ethereum".to_string());
        assert_eq!(
            err.to_string(),
            "Deployment feature not enabled for Ethereum"
        );
    }

    #[test]
    fn test_contract_deployment_fields() {
        let deployment = ContractDeployment {
            chain: Chain::Ethereum,
            address: vec![1, 2, 3, 4],
            transaction_hash: "0x1234".to_string(),
            gas_used: 50000,
            contract_type: "CsvLock".to_string(),
            block_number: Some(12345678),
        };

        assert_eq!(deployment.chain, Chain::Ethereum);
        assert_eq!(deployment.address, vec![1, 2, 3, 4]);
        assert_eq!(deployment.transaction_hash, "0x1234");
        assert_eq!(deployment.gas_used, 50000);
        assert_eq!(deployment.contract_type, "CsvLock");
        assert_eq!(deployment.block_number, Some(12345678));
    }
}
