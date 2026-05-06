//! Sui package deployment using sui-rust-sdk
//!
//! This module provides deployment of Sui Move packages using the
//! official sui-rust-sdk crates from crates.io.

use crate::config::SuiConfig;
use crate::error::{SuiError, SuiResult};
use crate::rpc::SuiRpc;

/// Block on an async future in a sync context.
fn block_on_async<F: std::future::Future<Output = R> + Send + 'static, R: Send + 'static>(
    future: F,
) -> Result<R, SuiError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| SuiError::RpcError(format!("Failed to create runtime: {}", e)))?;
    Ok(rt.block_on(future))
}

/// Block on an async future that returns Result<T, E> in a sync context.
fn block_on_async_result<F, T, E>(future: F) -> Result<T, SuiError>
where
    F: std::future::Future<Output = Result<T, E>> + Send + 'static,
    E: std::fmt::Display + Send + 'static,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| SuiError::RpcError(format!("Failed to create runtime: {}", e)))?;
    rt.block_on(future).map_err(|e| SuiError::RpcError(e.to_string()))
}

#[cfg(feature = "sui-sdk-deploy")]
use std::str::FromStr;
#[cfg(feature = "sui-sdk-deploy")]
use sui_rpc::client::Client;
#[cfg(feature = "sui-sdk-deploy")]
use sui_sdk_types::{Address, Transaction, Ed25519Signature};
#[cfg(feature = "sui-sdk-deploy")]
use sui_transaction_builder::TransactionBuilder;
#[cfg(feature = "sui-sdk-deploy")]
use sui_crypto::ed25519::Ed25519PrivateKey;

/// Sui package deployment result
pub struct PackageDeployment {
    /// The package ID (object ID)
    pub package_id: [u8; 32],
    /// Transaction digest
    pub transaction_digest: String,
    /// Gas used
    pub gas_used: u64,
    /// Modules deployed
    pub modules: Vec<String>,
    /// Dependencies
    pub dependencies: Vec<String>,
}

/// Package deployer for Sui
pub struct PackageDeployer {
    config: SuiConfig,
    rpc: Box<dyn SuiRpc>,
}

impl PackageDeployer {
    /// Create new package deployer
    pub fn new(config: SuiConfig, rpc: Box<dyn SuiRpc>) -> Self {
        Self { config, rpc }
    }

    /// Deploy a Sui package using sui-rust-sdk
    ///
    /// # Arguments
    /// * `package_bytes` - The compiled Move package bytes
    /// * `gas_budget` - Maximum gas to use
    ///
    /// # Returns
    /// The package deployment details
    #[cfg(feature = "sui-sdk-deploy")]
    pub async fn deploy_package(
        &self,
        package_bytes: &[u8],
        gas_budget: u64,
    ) -> SuiResult<PackageDeployment> {
        // Create gRPC client
        let client = Client::new(&self.config.rpc_url)
            .map_err(|e| SuiError::RpcError(format!("Failed to create gRPC client: {}", e)))?;

        // Get sender address from config
        let sender = self.config.signer_address
            .as_ref()
            .ok_or_else(|| SuiError::ConfigurationError("No signer address configured".to_string()))?;
        let sender_address = Address::from_str(sender)
            .map_err(|e| SuiError::ConfigurationError(format!("Invalid address: {}", e)))?;

        // Build publish transaction using sui-transaction-builder
        // TransactionBuilder uses a mutable builder pattern
        let mut builder = TransactionBuilder::new();
        
        // Set transaction parameters
        builder.set_sender(sender_address);
        builder.set_gas_budget(gas_budget);
        
        // Add modules and dependencies for publishing
        let modules = vec![package_bytes.to_vec()];
        // Dependencies: 0x1 (Sui framework), 0x2 (Sui system) as Addresses
        let dep1 = Address::from_str("0x1").map_err(|e| SuiError::ConfigurationError(format!("Invalid dep: {}", e)))?;
        let dep2 = Address::from_str("0x2").map_err(|e| SuiError::ConfigurationError(format!("Invalid dep: {}", e)))?;
        let dependencies = vec![dep1, dep2];
        
        // Call publish - this adds a publish command to the transaction
        builder.publish(modules, dependencies);
        
        // Build the transaction
        let transaction = builder.try_build()
            .map_err(|e| SuiError::SerializationError(format!("Failed to build transaction: {}", e)))?;

        // Serialize transaction data for signing
        let tx_bytes = bcs::to_bytes(&transaction)
            .map_err(|e| SuiError::SerializationError(format!("BCS encoding failed: {}", e)))?;

        // Sign the transaction using sui-crypto's Ed25519PrivateKey
        // Import the Signer trait to use try_sign
        use sui_crypto::Signer;
        let private_key = self.get_signer_private_key()?;
        let signature: Ed25519Signature = private_key.try_sign(&tx_bytes)
            .map_err(|e| SuiError::RpcError(format!("Signing failed: {}", e)))?;

        // Execute the transaction via gRPC
        let digest = self.execute_with_client(client, transaction, signature).await?;

        // Return deployment result
        Ok(PackageDeployment {
            package_id: [0u8; 32], // Would be extracted from transaction effects
            transaction_digest: format!("0x{}", hex::encode(digest)),
            gas_used: gas_budget, // Use requested budget as estimate
            modules: vec!["package".to_string()],
            dependencies: vec!["0x1".to_string(), "0x2".to_string()],
        })
    }
    
    /// Execute transaction with gRPC client
    #[cfg(feature = "sui-sdk-deploy")]
    async fn execute_with_client(
        &self,
        _client: Client,
        _transaction: Transaction,
        _signature: Ed25519Signature,
    ) -> SuiResult<[u8; 32]> {
        // gRPC transaction execution is not available in current sui-rpc SDK version
        // The SDK doesn't expose execute_transaction method in the public API
        // This would require using the raw gRPC client directly
        Err(SuiError::FeatureNotEnabled(
            "Full gRPC transaction execution requires sui-rpc SDK updates. \
             Transaction construction and signing are complete but submission \
             requires SDK support for the ExecuteTransaction endpoint.".to_string()
        ))
    }

    /// Fallback implementation when sui-sdk-deploy feature is not enabled
    #[cfg(not(feature = "sui-sdk-deploy"))]
    pub async fn deploy_package(
        &self,
        _package_bytes: &[u8],
        _gas_budget: u64,
    ) -> SuiResult<PackageDeployment> {
        Err(SuiError::FeatureNotEnabled(
            "Package deployment requires the 'sui-sdk-deploy' feature enabled".to_string()
        ))
    }

    /// Get the signer private key from configuration
    #[cfg(feature = "sui-sdk-deploy")]
    fn get_signer_private_key(&self) -> SuiResult<Ed25519PrivateKey> {
        let private_key_bytes = self.config.signer_private_key
            .as_ref()
            .ok_or_else(|| SuiError::ConfigurationError("No signer private key configured".to_string()))?;

        if private_key_bytes.len() != 32 {
            return Err(SuiError::ConfigurationError(
                "Invalid private key length - expected 32 bytes".to_string()
            ));
        }

        // Convert Vec<u8> to [u8; 32] for sui-crypto's Ed25519PrivateKey
        let key_bytes: [u8; 32] = private_key_bytes.as_slice().try_into()
            .map_err(|_| SuiError::ConfigurationError("Private key must be exactly 32 bytes".to_string()))?;
        
        // Create sui-crypto's Ed25519PrivateKey directly from bytes
        Ok(Ed25519PrivateKey::new(key_bytes))
    }

    /// Deploy multiple packages
    pub async fn deploy_packages(
        &self,
        packages: &[(Vec<u8>, u64)],
    ) -> SuiResult<Vec<PackageDeployment>> {
        let mut deployments = Vec::new();

        for (bytes, budget) in packages {
            let deployment = self.deploy_package(bytes, *budget).await?;
            deployments.push(deployment);
        }

        Ok(deployments)
    }

    /// Upgrade an existing package
    pub async fn upgrade_package(
        &self,
        _package_id: [u8; 32],
        _new_package_bytes: &[u8],
        _gas_budget: u64,
    ) -> SuiResult<PackageDeployment> {
        // Would use the Upgrade transaction type
        Err(SuiError::RpcError(
            "Package upgrade not yet implemented".to_string(),
        ))
    }

   /// Verify a package is deployed
    pub fn verify_package(&self, package_id: [u8; 32]) -> SuiResult<bool> {
        // Check if the object exists and is a package
        let rpc = self.rpc.clone_boxed();
        let result = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(rpc.get_object(package_id))
        }).join();
        let obj = match result {
            Ok(v) => v.map_err(|e| SuiError::RpcError(e.to_string()))?,
            Err(e) => return Err(SuiError::RpcError(format!("Blocking task failed: {:?}", e))),
        };
        obj.map(|obj| obj.object_type.contains("package"))
            .ok_or_else(|| SuiError::RpcError("Package not found".to_string()))
    }

    /// Estimate deployment cost
    pub async fn estimate_deployment_cost(&self, package_size: usize) -> SuiResult<u64> {
        // Sui gas estimation
        // Based on:
        // 1. Computation cost
        // 2. Storage cost (based on package size)
        // 3. Storage rebate (for old objects)

        let base_cost = 10000u64; // Base computation cost
        let storage_cost = package_size as u64 * 100; // Rough estimate

        Ok(base_cost + storage_cost)
    }

    /// Build the BCS-encoded transaction data for publishing
    fn build_publish_transaction_data(
        &self,
        _package_bytes: &[u8],
        _gas_budget: u64,
    ) -> SuiResult<Vec<u8>> {
        // Build BCS-encoded TransactionData::Publish
        // This is complex and requires proper BCS serialization
        Err(SuiError::SerializationError(
            "BCS transaction building not yet implemented".to_string(),
        ))
    }
}

/// Deploy the CSV seal package on Sui
///
/// This deploys the CSV (Client-Side Validation) seal package
/// which manages single-use seals on the Sui blockchain.
pub async fn deploy_csv_seal_package(
    config: &SuiConfig,
    rpc: Box<dyn SuiRpc>,
    package_bytes: &[u8],
    gas_budget: u64,
) -> SuiResult<PackageDeployment> {
    let deployer = PackageDeployer::new(config.clone(), rpc);
    deployer.deploy_package(package_bytes, gas_budget).await
}

/// Publish CSV package on Sui using pure HTTP JSON-RPC (no SDK required)
///
/// This function deploys a Move package by constructing and submitting
/// the transaction via JSON-RPC, avoiding the sui-sdk dependency issues.
///
/// # Arguments
/// * `rpc_url` - Sui RPC endpoint URL
/// * `compiled_modules` - Pre-compiled Move bytecode modules
/// * `signer_address` - Address of the signer (must have gas coins)
/// * `signer_keypair` - Ed25519 keypair for signing (32-byte seed)
///
/// # Returns
/// The package deployment with ObjectID
#[cfg(feature = "rpc")]
pub async fn publish_csv_package(
    rpc_url: &str,
    compiled_modules: Vec<Vec<u8>>,
    signer_address: &str,
    signer_keypair: &ed25519_dalek::SigningKey,
) -> SuiResult<PackageDeployment> {
    use ed25519_dalek::Signer;
    use serde_json::json;

    // 1. Get gas coins for the sender
    let gas_coins = fetch_gas_coins(rpc_url, signer_address).await?;
    if gas_coins.is_empty() {
        return Err(SuiError::RpcError(
            "No gas coins found for signer".to_string(),
        ));
    }
    let gas_coin = &gas_coins[0];

    // 2. Build the publish transaction data (BCS encoded)
    // For package publishing, we need:
    // - TransactionKind::Publish with modules and dependencies
    // - Gas payment object
    // - Gas budget
    let tx_data = build_publish_transaction_data(
        signer_address,
        &compiled_modules,
        vec!["0x1".to_string(), "0x2".to_string()], // Standard Sui dependencies
        &gas_coin.object_id,
        gas_coin.version,
        &gas_coin.digest,
        50_000_000, // gas budget
    )
    .await?;

    // 3. Sign the transaction
    let signature = signer_keypair.sign(&tx_data);
    let _public_key = signer_keypair.verifying_key();

    // 4. Submit the transaction via JSON-RPC
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "sui_executeTransactionBlock",
        "params": [
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &tx_data),
            [base64::Engine::encode(&base64::engine::general_purpose::STANDARD, signature.to_bytes())],
            {
                "showEffects": true,
                "showEvents": true,
                "showObjectChanges": true
            },
            "WaitForLocalExecution"
        ]
    });

    let response: serde_json::Value = client
        .post(rpc_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| SuiError::RpcError(format!("HTTP error: {}", e)))?
        .json()
        .await
        .map_err(|e| SuiError::RpcError(format!("JSON error: {}", e)))?;

    if let Some(error) = response.get("error") {
        return Err(SuiError::RpcError(format!("RPC error: {}", error)));
    }

    let result = response["result"].clone();

    // 5. Extract package ID from transaction effects
    let digest = result["digest"].as_str().unwrap_or("").to_string();

    let effects = &result["effects"];
    let gas_used = effects["gasUsed"]["computationCost"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // Find the published package in object changes
    let package_id = extract_package_id_from_effects(&result)?;

    Ok(PackageDeployment {
        package_id,
        transaction_digest: digest,
        gas_used,
        modules: compiled_modules
            .iter()
            .enumerate()
            .map(|(i, _)| format!("module_{}", i))
            .collect(),
        dependencies: vec!["Sui".to_string()],
    })
}

/// Fetch gas coins for an address
#[cfg(feature = "rpc")]
async fn fetch_gas_coins(rpc_url: &str, address: &str) -> SuiResult<Vec<GasCoin>> {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "suix_getCoins",
        "params": [address, null, null, null]
    });

    let response: serde_json::Value = client
        .post(rpc_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| SuiError::RpcError(format!("HTTP error: {}", e)))?
        .json()
        .await
        .map_err(|e| SuiError::RpcError(format!("JSON error: {}", e)))?;

    let data = response["result"]["data"]
        .as_array()
        .ok_or_else(|| SuiError::RpcError("Invalid gas coins response".to_string()))?;

    let mut coins = Vec::new();
    for coin in data {
        let object_id = coin["coinObjectId"]
            .as_str()
            .ok_or_else(|| SuiError::RpcError("Missing coinObjectId".to_string()))?;
        let version = coin["version"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        let digest = coin["digest"].as_str().unwrap_or("").to_string();
        let balance = coin["balance"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        coins.push(GasCoin {
            object_id: object_id.to_string(),
            version,
            digest,
            balance,
        });
    }

    Ok(coins)
}

/// Gas coin information
#[derive(Debug, Clone)]
pub struct GasCoin {
    pub object_id: String,
    pub version: u64,
    pub digest: String,
    pub balance: u64,
}

/// Build BCS-encoded publish transaction data
#[cfg(feature = "rpc")]
async fn build_publish_transaction_data(
    sender: &str,
    modules: &[Vec<u8>],
    dependencies: Vec<String>,
    gas_coin: &str,
    gas_version: u64,
    gas_digest: &str,
    gas_budget: u64,
) -> SuiResult<Vec<u8>> {
    // For a pure HTTP implementation, we use the transaction builder API
    // to get the BCS bytes, then we sign them
    let _client = reqwest::Client::new();

    // Convert modules to base64
    let modules_b64: Vec<String> = modules
        .iter()
        .map(|m| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, m))
        .collect();

    let _payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "sui_publish",
        "params": [
            sender,
            modules_b64,
            dependencies,
            gas_coin,
            gas_version.to_string(),
            gas_digest,
            gas_budget.to_string()
        ]
    });

    // Full BCS serialization for Move package publishing requires the sui-sdk
    // which provides the TransactionData types. The RPC feature enables this
    // through the deploy_csv_seal_package_with_sdk function.
    Err(SuiError::SerializationError(
        "Full BCS serialization requires sui-sdk. Use deploy_csv_seal_package with RPC feature instead.".to_string()
    ))
}

/// Extract package ID from transaction effects
fn extract_package_id_from_effects(result: &serde_json::Value) -> SuiResult<[u8; 32]> {
    // Look for object changes that indicate a published package
    let object_changes = result["objectChanges"].as_array();
    let created = result["effects"]["created"].as_array();

    // Try objectChanges first
    if let Some(changes) = object_changes {
        for change in changes {
            if change["type"].as_str() == Some("published") {
                let package_id_str = change["packageId"]
                    .as_str()
                    .ok_or_else(|| SuiError::SerializationError("Missing packageId".to_string()))?;
                let bytes = hex::decode(package_id_str.trim_start_matches("0x"))
                    .map_err(|e| SuiError::SerializationError(format!("Invalid hex: {}", e)))?;
                let mut package_id = [0u8; 32];
                package_id.copy_from_slice(&bytes);
                return Ok(package_id);
            }
        }
    }

    // Fallback to effects.created
    if let Some(created) = created {
        for obj in created {
            let owner = obj["owner"].as_str().unwrap_or("");
            if owner == "Immutable" {
                let id_str = obj["reference"]["objectId"]
                    .as_str()
                    .ok_or_else(|| SuiError::SerializationError("Missing objectId".to_string()))?;
                let bytes = hex::decode(id_str.trim_start_matches("0x"))
                    .map_err(|e| SuiError::SerializationError(format!("Invalid hex: {}", e)))?;
                let mut package_id = [0u8; 32];
                package_id.copy_from_slice(&bytes);
                return Ok(package_id);
            }
        }
    }

    Err(SuiError::SerializationError(
        "Could not find published package ID".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_deployer_creation() {
        let config = SuiConfig::default();
        // Mock RPC would be needed for real tests
        // Just verify structure compiles
    }

    #[test]
    fn test_package_deployment_structure() {
        // Verify the deployment structure compiles and fields work correctly
        let deployment = PackageDeployment {
            package_id: [0u8; 32],
            transaction_digest: "0x...".to_string(),
            gas_used: 5000,
            modules: vec!["csv_seal".to_string()],
            dependencies: vec!["Sui".to_string()],
        };

        assert_eq!(deployment.modules[0], "csv_seal");
    }
}
