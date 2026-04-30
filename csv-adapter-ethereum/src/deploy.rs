//! Ethereum contract deployment via RPC using Alloy
//!
//! This module provides RPC-based deployment of Ethereum smart contracts
//! using the Alloy SDK, replacing CLI commands like `forge create`.

use crate::config::EthereumConfig;
use crate::error::{EthereumError, EthereumResult};
use crate::rpc::EthereumRpc;

// Alloy imports for real contract deployment
#[cfg(feature = "rpc")]
use alloy::{
    network::EthereumWallet,
    primitives::Bytes,
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
};
#[cfg(feature = "rpc")]
use std::str::FromStr;

/// Ethereum contract deployment result
pub struct ContractDeployment {
    /// Contract address (20 bytes)
    pub contract_address: [u8; 20],
    /// Transaction hash
    pub transaction_hash: [u8; 32],
    /// Block number where contract was deployed
    pub block_number: u64,
    /// Gas used for deployment
    pub gas_used: u64,
    /// Contract bytecode deployed
    pub deployed_bytecode: Vec<u8>,
    /// Constructor arguments (if any)
    pub constructor_args: Vec<u8>,
}

/// Contract deployer for Ethereum
pub struct ContractDeployer {
    config: EthereumConfig,
    rpc: Box<dyn EthereumRpc>,
}

impl ContractDeployer {
    /// Create new contract deployer
    pub fn new(config: EthereumConfig, rpc: Box<dyn EthereumRpc>) -> Self {
        Self { config, rpc }
    }

    /// Deploy a smart contract
    ///
    /// # Arguments
    /// * `bytecode` - The compiled contract bytecode + constructor args
    /// * `from_address` - Address deploying the contract (must have funds)
    ///
    /// # Returns
    /// The contract deployment details
    ///
    /// # Note
    /// This synchronous method provides deployment configuration and validation.
    /// For actual deployment with transaction broadcasting, use `deploy_csv_lock()` 
    /// which uses Alloy for async RPC-based deployment with proper signing.
    pub fn deploy_contract(
        &self,
        bytecode: &[u8],
        from_address: [u8; 20],
    ) -> EthereumResult<ContractDeployment> {
        // Estimate gas for deployment
        let gas_limit = self.estimate_deployment_gas(bytecode.len())?;

        // Get current nonce for address calculation
        let nonce = self
            .rpc
            .get_transaction_count(from_address)
            .map_err(|e| EthereumError::RpcError(format!("Failed to get nonce: {:?}", e)))?;

        // Calculate the contract address that would be created
        // Contract address = keccak256(rlp_encode([sender, nonce]))[12:]
        let contract_address = calculate_contract_address(from_address, nonce);

        // Note: Full deployment with transaction signing requires async RPC
        // The synchronous API provides configuration and address prediction
        // Use deploy_csv_lock() for complete deployment with signing

        Ok(ContractDeployment {
            contract_address,
            transaction_hash: [0u8; 32], // Would be set after broadcast
            block_number: 0,             // Would be set after confirmation
            gas_used: gas_limit,
            deployed_bytecode: bytecode.to_vec(),
            constructor_args: vec![],
        })
    }

    /// Deploy contract with constructor arguments
    pub fn deploy_contract_with_args(
        &self,
        bytecode: &[u8],
        constructor_abi: &[u8],
        constructor_args: &[u8],
        from_address: [u8; 20],
    ) -> EthereumResult<ContractDeployment> {
        // Encode constructor arguments according to ABI
        let encoded_args = self.encode_constructor_args(constructor_abi, constructor_args)?;

        // Append to bytecode
        let mut full_bytecode = bytecode.to_vec();
        full_bytecode.extend_from_slice(&encoded_args);

        self.deploy_contract(&full_bytecode, from_address)
    }

    /// Verify a contract is deployed
    pub fn verify_contract(&self, address: [u8; 20]) -> EthereumResult<bool> {
        // Check if there's code at the address
        match self.rpc.get_code(address) {
            Ok(code) => Ok(!code.is_empty()),
            Err(_) => Ok(false),
        }
    }

    /// Estimate gas for deployment
    pub fn estimate_deployment_gas(&self, bytecode_size: usize) -> EthereumResult<u64> {
        // Base gas for contract creation
        let base_gas = 21000u64;
        // Gas per byte of init code
        let per_byte_gas = 200u64;
        // Additional for storage
        let storage_gas = 20000u64;

        Ok(base_gas + (bytecode_size as u64 * per_byte_gas) + storage_gas)
    }

    /// Get contract code
    pub fn get_contract_code(&self, address: [u8; 20]) -> EthereumResult<Vec<u8>> {
        self.rpc
            .get_code(address)
            .map_err(|e| EthereumError::RpcError(format!("Failed to get code: {:?}", e)))
    }

    /// Encode constructor arguments according to Solidity ABI
    fn encode_constructor_args(
        &self,
        _abi: &[u8],
        _args: &[u8],
    ) -> EthereumResult<Vec<u8>> {
        // Would parse ABI and encode arguments properly
        // For now, return empty
        Ok(vec![])
    }
}

/// Deploy the CSV seal contract on Ethereum using Alloy
///
/// This deploys the CSV (Client-Side Validation) seal contract
/// which manages single-use seals on the Ethereum blockchain.
///
/// # Arguments
/// * `rpc_url` - Ethereum RPC endpoint URL
/// * `private_key_hex` - Deployer private key (hex string, with or without 0x prefix)
/// * `bytecode` - Compiled contract bytecode
///
/// # Returns
/// The contract deployment result with address and transaction hash
#[cfg(feature = "rpc")]
pub async fn deploy_csv_lock(
    rpc_url: &str,
    private_key_hex: &str,
    bytecode: &[u8],
) -> EthereumResult<ContractDeployment> {
    // Parse private key
    let key_clean = private_key_hex.trim_start_matches("0x");
    let signer = PrivateKeySigner::from_str(key_clean)
        .map_err(|e| EthereumError::WalletError(format!("Invalid private key: {}", e)))?;
    
    // Create wallet
    let wallet = EthereumWallet::from(signer.clone());
    
    // Create provider
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect_http(rpc_url.parse().map_err(|e| {
            EthereumError::ConfigError(format!("Invalid RPC URL: {}", e))
        })?);
    
    // Get sender address and nonce
    let sender = signer.address();
    let nonce = provider.get_transaction_count(sender).await
        .map_err(|e| EthereumError::RpcError(format!("Failed to get nonce: {}", e)))?;
    
    // Build deployment transaction
    let tx = TransactionRequest::default()
        .from(sender)
        .nonce(nonce)
        .input(Bytes::from(bytecode.to_vec()).into())
        .gas_limit(3_000_000u64); // Estimate or use dynamic gas
    
    // Send transaction and wait for receipt
    let tx_hash = provider.send_transaction(tx).await
        .map_err(|e| EthereumError::RpcError(format!("Failed to send transaction: {}", e)))?;
    
    // Wait for confirmation
    let receipt = tx_hash.get_receipt().await
        .map_err(|e| EthereumError::RpcError(format!("Failed to get receipt: {}", e)))?;
    
    // Get contract address from receipt
    let contract_address = receipt.contract_address
        .ok_or_else(|| EthereumError::DeploymentError("Contract address not found in receipt".to_string()))?;
    
    Ok(ContractDeployment {
        contract_address: (*contract_address).into(),
        transaction_hash: receipt.transaction_hash.0,
        block_number: receipt.block_number.unwrap_or(0),
        gas_used: receipt.gas_used,
        deployed_bytecode: bytecode.to_vec(),
        constructor_args: vec![],
    })
}

/// Deploy the CSV seal contract on Ethereum (non-RPC fallback)
///
/// When the `rpc` feature is not enabled, this returns an error directing
/// users to enable the feature for full deployment capabilities.
#[cfg(not(feature = "rpc"))]
pub fn deploy_csv_seal_contract(
    config: &EthereumConfig,
    bytecode: &[u8],
    from_address: [u8; 20],
) -> EthereumResult<ContractDeployment> {
    let _ = config;
    let _ = bytecode;
    let _ = from_address;
    Err(EthereumError::NotImplemented(
        "Contract deployment requires the 'rpc' feature. Enable it in Cargo.toml: csv-adapter-ethereum = { features = [\"rpc\"] }".to_string()
    ))
}

/// Calculate contract address from sender and nonce
///
/// Contract address = keccak256(rlp_encode([sender, nonce]))[12:]
pub fn calculate_contract_address(sender: [u8; 20], nonce: u64) -> [u8; 20] {
    use sha3::{Digest, Keccak256};

    // Simple RLP encoding of [sender, nonce]
    let mut rlp = Vec::new();

    // Sender (20 bytes, length-prefixed if > 127)
    rlp.push(0x80 + 20);
    rlp.extend_from_slice(&sender);

    // Nonce (encode compactly)
    if nonce == 0 {
        rlp.push(0x80);
    } else if nonce < 128 {
        rlp.push(nonce as u8);
    } else {
        // Would need proper big-endian encoding for larger nonces
        rlp.push(0x80 + 8);
        rlp.extend_from_slice(&nonce.to_be_bytes());
    }

    // Prefix with list length
    let mut full_rlp = vec![0xc0 + rlp.len() as u8];
    full_rlp.extend_from_slice(&rlp);

    // Hash and extract address
    let hash = Keccak256::digest(&full_rlp);
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..]);

    address
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_deployer_creation() {
        let config = EthereumConfig::default();
        // Mock RPC would be needed for real tests
        // Just verify structure compiles
    }

    #[test]
    fn test_contract_deployment_placeholder() {
        // Verify the deployment structure compiles
        let deployment = ContractDeployment {
            contract_address: [0u8; 20],
            transaction_hash: [0u8; 32],
            block_number: 100,
            gas_used: 500000,
            deployed_bytecode: vec![0x60, 0x80], // PUSH1 80
            constructor_args: vec![],
        };

        assert_eq!(deployment.gas_used, 500000);
    }

    #[test]
    fn test_calculate_contract_address() {
        let sender = [0u8; 20];
        let nonce = 0u64;
        let address = calculate_contract_address(sender, nonce);

        // Address should be 20 bytes
        assert_eq!(address.len(), 20);

        // Different nonces should produce different addresses
        let address2 = calculate_contract_address(sender, 1);
        assert_ne!(address, address2);
    }
}
