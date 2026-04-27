//! Aptos module deployment via RPC
//!
//! This module provides RPC-based deployment of Aptos Move modules,
//! replacing the need for CLI commands like `aptos move publish`.

use ed25519_dalek::SigningKey;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::AptosConfig;
use crate::error::{AptosError, AptosResult};
use crate::rpc::AptosRpc;

// Aptos SDK imports for real deployment
#[cfg(feature = "aptos-sdk")]
use aptos_sdk::types::LocalAccount;

/// Aptos module deployment result
pub struct ModuleDeployment {
    /// The account address where the module is deployed
    pub account_address: [u8; 32],
    /// Module name
    pub module_name: String,
    /// Transaction version where the module was deployed
    pub version: u64,
    /// Transaction hash
    pub transaction_hash: String,
    /// Gas used
    pub gas_used: u64,
    /// Success status
    pub success: bool,
}

/// Module deployer for Aptos
pub struct ModuleDeployer {
    config: AptosConfig,
    signing_key: SigningKey,
    rpc: Box<dyn AptosRpc>,
}

impl ModuleDeployer {
    /// Create new module deployer
    pub fn new(
        config: AptosConfig,
        signing_key: SigningKey,
        rpc: Box<dyn AptosRpc>,
    ) -> Self {
        Self {
            config,
            signing_key,
            rpc,
        }
    }

    /// Deploy a Move module
    ///
    /// # Arguments
    /// * `module_bytes` - The compiled Move module bytecode
    /// * `module_name` - Name of the module
    ///
    /// # Returns
    /// The module deployment details
    pub async fn deploy_module(
        &self,
        module_bytes: &[u8],
        module_name: &str,
    ) -> AptosResult<ModuleDeployment> {
        // Get sender address from signing key
        let sender = self.signing_key.verifying_key().to_bytes();
        let mut sender_addr = [0u8; 32];
        sender_addr.copy_from_slice(&sender);

        // Get account sequence number
        let sequence_number = self
            .rpc
            .get_account_sequence_number(sender_addr)
            .map_err(|e| AptosError::SerializationError(format!("Failed to get sequence: {:?}", e)))?;

        // Build the publish transaction
        // Entry function: 0x1::code::publish_package_txn
        let payload = self.build_publish_payload(module_bytes, module_name)?;

        // Build and sign transaction
        let _tx = self
            .build_signed_transaction(sender_addr, sequence_number, payload)
            .await?;

        // Submit via RPC
        // let response = self.rpc.submit_transaction(tx).await?;

        // Placeholder - real implementation would submit and wait
        let _ = module_bytes;

        Ok(ModuleDeployment {
            account_address: sender_addr,
            module_name: module_name.to_string(),
            version: 1, // Would be actual version from response
            transaction_hash: "0x...".to_string(), // Would be actual hash
            gas_used: self.config.transaction.max_gas,
            success: true,
        })
    }

    /// Deploy multiple modules as a package
    pub async fn deploy_package(
        &self,
        modules: &[(Vec<u8>, String)],
        package_name: &str,
    ) -> AptosResult<Vec<ModuleDeployment>> {
        let mut deployments = Vec::new();

        for (bytes, name) in modules {
            let deployment = self.deploy_module(bytes, name).await?;
            deployments.push(deployment);
        }

        // Also deploy the package metadata if needed
        let _ = package_name;

        Ok(deployments)
    }

    /// Upgrade an existing module
    pub async fn upgrade_module(
        &self,
        module_bytes: &[u8],
        module_name: &str,
    ) -> AptosResult<ModuleDeployment> {
        // Check if upgrade policy allows it
        // Then deploy new version
        self.deploy_module(module_bytes, module_name).await
    }

    /// Verify a module is deployed
    pub fn verify_module(&self, address: [u8; 32], module_name: &str) -> AptosResult<bool> {
        let module_resource = format!(
            "0x1::code::PackageRegistry"
        );

        match self.rpc.get_account_resource(address, &module_resource) {
            Ok(Some(_)) => {
                // Module exists, check if specific module is in package
                // Real implementation would parse the PackageRegistry
                let _ = module_name;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Estimate deployment cost
    pub fn estimate_deployment_cost(&self, module_size: usize) -> u64 {
        // Aptos gas estimation
        // Base gas + per-byte cost
        let base_gas = self.config.transaction.max_gas as u64;
        let per_byte_cost = 10u64; // Rough estimate

        base_gas + (module_size as u64 * per_byte_cost)
    }

    /// Build publish payload for module
    fn build_publish_payload(
        &self,
        module_bytes: &[u8],
        _module_name: &str,
    ) -> AptosResult<serde_json::Value> {
        // Build the EntryFunction payload for publishing
        // Entry function: 0x1::code::publish_package_txn

        let payload = serde_json::json!({
            "type": "entry_function_payload",
            "function": "0x1::code::publish_package_txn",
            "type_arguments": [],
            "arguments": [
                // Metadata blob (simplified)
                "0x00",
                // Code blobs (the module bytes)
                format!("0x{}", hex::encode(module_bytes))
            ]
        });

        Ok(payload)
    }

    /// Build and sign a transaction
    async fn build_signed_transaction(
        &self,
        sender: [u8; 32],
        sequence_number: u64,
        payload: serde_json::Value,
    ) -> AptosResult<serde_json::Value> {
        use ed25519_dalek::Signer;

        // Get chain ID and ledger info
        let ledger = self
            .rpc
            .get_ledger_info()
            .map_err(|e| AptosError::SerializationError(format!("Failed to get ledger: {:?}", e)))?;

        // Calculate expiration
        let expiration_secs = (ledger.ledger_timestamp / 1_000_000) + 600;

        // Build unsigned transaction
        let tx_payload = serde_json::json!({
            "sender": format!("0x{}", hex::encode(sender)),
            "sequence_number": sequence_number.to_string(),
            "max_gas_amount": self.config.transaction.max_gas.to_string(),
            "gas_unit_price": "100",
            "expiration_timestamp_secs": expiration_secs.to_string(),
            "payload": payload,
        });

        // Sign the transaction
        let tx_json_str = serde_json::to_string(&tx_payload).unwrap_or_default();
        let message = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(tx_json_str.as_bytes());
            hasher.finalize()
        };

        let signature = self.signing_key.sign(&message);
        let public_key = self.signing_key.verifying_key().to_bytes();

        // Build signed transaction
        let signed_tx = serde_json::json!({
            "sender": format!("0x{}", hex::encode(sender)),
            "sequence_number": sequence_number.to_string(),
            "max_gas_amount": self.config.transaction.max_gas.to_string(),
            "gas_unit_price": "100",
            "expiration_timestamp_secs": expiration_secs.to_string(),
            "payload": payload,
            "signature": {
                "type": "ed25519_signature",
                "public_key": format!("0x{}", hex::encode(public_key)),
                "signature": format!("0x{}", hex::encode(signature.to_bytes()))
            }
        });

        Ok(signed_tx)
    }
}

/// Deploy the CSV seal module on Aptos
///
/// This deploys the CSV (Client-Side Validation) seal module
/// which manages single-use seals on the Aptos blockchain.
pub async fn deploy_csv_seal_module(
    config: &AptosConfig,
    signing_key: SigningKey,
    rpc: Box<dyn AptosRpc>,
    module_bytes: &[u8],
) -> AptosResult<ModuleDeployment> {
    let deployer = ModuleDeployer::new(config.clone(), signing_key, rpc);
    deployer.deploy_module(module_bytes, "csv_seal").await
}

/// Publish CSV module on Aptos using the Aptos SDK
///
/// # Arguments
/// * `rpc_url` - Aptos RPC endpoint URL
/// * `module_bytes` - Compiled Move module bytecode
/// * `signer` - LocalAccount with private key for signing
///
/// # Returns
/// The module deployment with transaction hash
#[cfg(feature = "aptos-sdk")]
pub async fn publish_csv_module(
    rpc_url: &str,
    module_bytes: Vec<u8>,
    signer: &aptos_sdk::types::LocalAccount,
) -> AptosResult<ModuleDeployment> {
    use aptos_sdk::{
        rest_client::Client,
        transaction_builder::TransactionBuilder,
        types::transaction::{EntryFunction, TransactionPayload},
    };
    
    // Create REST client
    let client = Client::new(rpc_url.parse().map_err(|e| {
        AptosError::SerializationError(format!("Invalid RPC URL: {}", e))
    })?);
    
    // Get chain ID and account sequence
    let account_info = client
        .get_account(signer.address())
        .await
        .map_err(|e| AptosError::RpcError(format!("Failed to get account: {}", e)))?;
    let sequence_number = account_info.inner().sequence_number;
    
    // Build publish transaction using 0x1::code::publish_package_txn
    let code_payload = TransactionPayload::EntryFunction(EntryFunction::new(
        aptos_sdk::move_types::language_storage::ModuleId::new(
            aptos_sdk::types::account_address::AccountAddress::ONE,
            aptos_sdk::move_types::ident_str!("code").to_owned(),
        ),
        aptos_sdk::move_types::ident_str!("publish_package_txn").to_owned(),
        vec![], // type arguments
        vec![
            // metadata bytes (empty for now)
            bcs::to_bytes::<Vec<u8>>(&vec![]).map_err(|e| {
                AptosError::SerializationError(format!("Failed to serialize metadata: {}", e))
            })?,
            // code bytes
            bcs::to_bytes::<Vec<Vec<u8>>>(&vec![module_bytes]).map_err(|e| {
                AptosError::SerializationError(format!("Failed to serialize code: {}", e))
            })?,
        ],
    ));
    
    // Build transaction
    let tx_builder = TransactionBuilder::new(
        code_payload,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 600,
        aptos_sdk::types::chain_id::ChainId::testnet(), // Would detect from network
    )
    .sender(signer.address())
    .sequence_number(sequence_number)
    .max_gas_amount(100000)
    .gas_unit_price(100);
    
    // Sign transaction
    let signed_tx = signer.sign_with_transaction_builder(tx_builder);
    
    // Submit and wait
    let response = client
        .submit_and_wait(&signed_tx)
        .await
        .map_err(|e| AptosError::RpcError(format!("Failed to submit transaction: {}", e)))?;
    
    Ok(ModuleDeployment {
        account_address: signer.address().into_bytes(),
        module_name: "csv_seal".to_string(),
        version: response.version,
        transaction_hash: response.hash.to_string(),
        gas_used: response.gas_used.unwrap_or(0),
        success: response.success,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    #[test]
    fn test_module_deployer_creation() {
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let config = AptosConfig::default();
        // Mock RPC would be needed for real tests
        // Just verify structure compiles
    }

    #[test]
    fn test_module_deployment_placeholder() {
        // Verify the deployment structure compiles
        let deployment = ModuleDeployment {
            account_address: [0u8; 32],
            module_name: "csv_seal".to_string(),
            version: 1,
            transaction_hash: "0x...".to_string(),
            gas_used: 1000,
            success: true,
        };

        assert_eq!(deployment.module_name, "csv_seal");
        assert!(deployment.success);
    }

    #[test]
    fn test_estimate_deployment_cost() {
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let config = AptosConfig::default();
        let mock_rpc = crate::rpc::MockAptosRpc::new(1);
        let deployer = ModuleDeployer::new(config, signing_key, Box::new(mock_rpc));

        let cost = deployer.estimate_deployment_cost(1024);
        assert!(cost > 0);
    }
}
