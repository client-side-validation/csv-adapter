//! Cross-chain utility functions

use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use chrono;

use csv_adapter_core::hash::Hash;

use crate::config::{Chain, Config};
use crate::output;

/// RPC response for block height queries
#[derive(Debug, serde::Deserialize)]
pub struct JsonRpcResponse<T> {
    pub result: Option<T>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, serde::Deserialize)]
pub struct JsonRpcError {
    pub message: String,
}

/// Bitcoin REST API block height response
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub struct BitcoinBlockHeight {
    pub height: u64,
}

pub fn get_chain_height(chain: &Chain, config: &Config) -> u64 {
    // Try to fetch from RPC, fallback to reasonable defaults
    let runtime = tokio::runtime::Runtime::new().ok();

    if let Some(rt) = runtime {
        let result = rt.block_on(async { fetch_chain_height_rpc(chain, config).await });

        if let Ok(height) = result {
            return height;
        }
    }

    // Fallback to reasonable defaults if RPC fails
    tracing::warn!(chain = ?chain, "RPC height fetch failed, using fallback");
    match chain {
        Chain::Bitcoin => 300_000,
        Chain::Ethereum => 7_000_000,
        Chain::Sui => 350_000_000,
        Chain::Aptos => 15_000_000,
        Chain::Solana => 250_000_000, // Solana has very high block numbers
    }
}

/// Fetch chain height via RPC call
async fn fetch_chain_height_rpc(chain: &Chain, config: &Config) -> anyhow::Result<u64> {
    let chain_config = config
        .chains
        .get(chain)
        .ok_or_else(|| anyhow::anyhow!("Chain not configured"))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    match chain {
        Chain::Bitcoin => {
            // Mempool.space API or similar
            let url = if chain_config.rpc_url.contains("mempool.space") {
                format!(
                    "{}/api/blocks/tip/height",
                    chain_config.rpc_url.trim_end_matches('/')
                )
            } else {
                // Fallback to esplora-style endpoint
                format!(
                    "{}/blocks/tip/height",
                    chain_config.rpc_url.trim_end_matches('/')
                )
            };

            let response = client.get(&url).send().await?;
            let height: u64 = response.text().await?.parse()?;
            Ok(height)
        }
        Chain::Ethereum => {
            // JSON-RPC eth_blockNumber
            let body = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_blockNumber",
                "params": [],
                "id": 1
            });

            let response = client
                .post(&chain_config.rpc_url)
                .json(&body)
                .send()
                .await?;

            let rpc_response: JsonRpcResponse<String> = response.json().await?;

            if let Some(error) = rpc_response.error {
                return Err(anyhow::anyhow!("RPC error: {}", error.message));
            }

            let hex_height = rpc_response
                .result
                .ok_or_else(|| anyhow::anyhow!("No result in response"))?;

            // Parse hex string (0x prefix)
            let height = u64::from_str_radix(hex_height.trim_start_matches("0x"), 16)?;
            Ok(height)
        }
        Chain::Sui => {
            // Sui JSON-RPC sui_getLatestCheckpointSequenceNumber
            let body = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "sui_getLatestCheckpointSequenceNumber",
                "params": [],
                "id": 1
            });

            let response = client
                .post(&chain_config.rpc_url)
                .json(&body)
                .send()
                .await?;

            let rpc_response: JsonRpcResponse<String> = response.json().await?;

            if let Some(error) = rpc_response.error {
                return Err(anyhow::anyhow!("RPC error: {}", error.message));
            }

            let checkpoint = rpc_response
                .result
                .ok_or_else(|| anyhow::anyhow!("No result in response"))?;

            let height = u64::from_str_radix(checkpoint.trim_start_matches("0x"), 16)?;
            Ok(height)
        }
        Chain::Aptos => {
            // Aptos REST API - get ledger info
            let url = format!("{}/v1", chain_config.rpc_url.trim_end_matches('/'));

            let response = client.get(&url).send().await?;
            let ledger_info: serde_json::Value = response.json().await?;

            let version = ledger_info["block_height"]
                .as_str()
                .or_else(|| ledger_info["ledger_version"].as_str())
                .ok_or_else(|| anyhow::anyhow!("No block height in response"))?;

            let height = u64::from_str_radix(version.trim_start_matches("0x"), 16)
                .or_else(|_| version.parse())?;
            Ok(height)
        }
        Chain::Solana => {
            // Solana JSON-RPC - getEpochInfo
            let body = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "getEpochInfo",
                "params": [],
                "id": 1
            });

            let response = client
                .post(&chain_config.rpc_url)
                .json(&body)
                .send()
                .await?;

            let rpc_response: JsonRpcResponse<serde_json::Value> = response.json().await?;

            if let Some(error) = rpc_response.error {
                return Err(anyhow::anyhow!("RPC error: {}", error.message));
            }

            let epoch_info = rpc_response
                .result
                .ok_or_else(|| anyhow::anyhow!("No result in response"))?;

            let slot = epoch_info["absoluteSlot"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("No slot in response"))?;
            Ok(slot)
        }
    }
}

/// Get the required confirmation depth for a chain.
pub fn get_chain_confirmations(chain: &Chain) -> u64 {
    match chain {
        Chain::Bitcoin => 6,   // ~1 hour on signet
        Chain::Ethereum => 15, // ~3 minutes
        Chain::Sui => 1,       // Finality is ~1 checkpoint
        Chain::Aptos => 1,     // Finality is ~1 block (HotStuff),
        Chain::Solana => 1,    // Finality is ~1 block (Proof of History)
    }
}

/// Fetch actual gas balance from chain RPC
pub fn fetch_gas_balance(chain: &Chain, config: &Config, address: &str) -> anyhow::Result<u64> {
    let chain_config = config
        .chains
        .get(chain)
        .ok_or_else(|| anyhow::anyhow!("Chain not configured"))?;

    let runtime = tokio::runtime::Runtime::new()?;
    
    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        match chain {
            Chain::Bitcoin => {
                // Fetch UTXO balance
                let url = format!(
                    "{}/api/address/{}/balance",
                    chain_config.rpc_url.trim_end_matches('/'),
                    address
                );
                let response = client.get(&url).send().await?;
                let balance: u64 = response.text().await?.parse()?;
                Ok(balance)
            }
            Chain::Ethereum => {
                // JSON-RPC eth_getBalance
                let body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "eth_getBalance",
                    "params": [address, "latest"],
                    "id": 1
                });

                let response = client
                    .post(&chain_config.rpc_url)
                    .json(&body)
                    .send()
                    .await?;

                let rpc_response: JsonRpcResponse<String> = response.json().await?;
                
                if let Some(error) = rpc_response.error {
                    return Err(anyhow::anyhow!("RPC error: {}", error.message));
                }

                let hex_balance = rpc_response
                    .result
                    .ok_or_else(|| anyhow::anyhow!("No result in response"))?;

                let balance = u64::from_str_radix(hex_balance.trim_start_matches("0x"), 16)?;
                Ok(balance)
            }
            Chain::Sui => {
                // Sui JSON-RPC sui_getBalance
                let body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "sui_getBalance",
                    "params": [address],
                    "id": 1
                });

                let response = client
                    .post(&chain_config.rpc_url)
                    .json(&body)
                    .send()
                    .await?;

                let rpc_response: JsonRpcResponse<serde_json::Value> = response.json().await?;
                
                if let Some(error) = rpc_response.error {
                    return Err(anyhow::anyhow!("RPC error: {}", error.message));
                }

                let result = rpc_response
                    .result
                    .ok_or_else(|| anyhow::anyhow!("No result in response"))?;

                let balance = result["totalBalance"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("No balance in response"))?
                    .parse()?;

                Ok(balance)
            }
            Chain::Aptos => {
                // Aptos REST API get account balance
                let url = format!(
                    "{}/v1/accounts/{}/resource/0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
                    chain_config.rpc_url.trim_end_matches('/'),
                    address
                );

                let response = client.get(&url).send().await?;
                let account_resource: serde_json::Value = response.json().await?;
                
                let balance = account_resource["data"]["coin"]["value"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("No balance in response"))?
                    .parse()?;

                Ok(balance)
            }
            Chain::Solana => {
                // Solana JSON-RPC getBalance
                let body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "getBalance",
                    "params": [address],
                    "id": 1
                });

                let response = client
                    .post(&chain_config.rpc_url)
                    .json(&body)
                    .send()
                    .await?;

                let rpc_response: JsonRpcResponse<serde_json::Value> = response.json().await?;
                
                if let Some(error) = rpc_response.error {
                    return Err(anyhow::anyhow!("RPC error: {}", error.message));
                }

                let result = rpc_response
                    .result
                    .ok_or_else(|| anyhow::anyhow!("No result in response"))?;

                let balance = result["value"]
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("No balance in response"))?;

                Ok(balance)
            }
        }
    })
}

/// Get private key for a chain from config
pub fn get_private_key(config: &crate::config::Config, _state: &crate::state::UnifiedStateManager, chain: Chain) -> Result<String> {
    // First try to get from wallet configuration
    if let Some(wallet) = config.wallets.get(&chain) {
        if let Some(key) = &wallet.private_key {
            return Ok(key.clone());
        }
    }
    
    // If no gas account configured, check chain defaults
    // Try to load from environment variable or secure keystore file
    let env_var_name = format!("{}_PRIVATE_KEY", chain.to_string().to_uppercase());
    if let Ok(key) = std::env::var(&env_var_name) {
        return Ok(key);
    }
    
    // Try to load from keystore file if available
    let keystore_path = dirs::home_dir()
        .map(|h| h.join(".csv").join("keystore").join(format!("{}.json", chain.to_string().to_lowercase())));
    
    if let Some(path) = keystore_path {
        if path.exists() {
            // Try to load keystore - would need password in real implementation
            tracing::debug!("Found keystore at {:?}", path);
            // For now, prompt user that keystore exists but password is needed
            return Err(anyhow::anyhow!(
                "Keystore found at {:?} but password is required. Use --password flag or set {}_PRIVATE_KEY environment variable",
                path,
                chain.to_string().to_uppercase()
            ));
        }
    }
    
    Err(anyhow::anyhow!(
        "No private key found for {:?}. Configure a gas account, set {}_PRIVATE_KEY environment variable, or create a keystore.",
        chain,
        chain.to_string().to_uppercase()
    ))
}

/// Parse a hex string into a 32-byte Hash
pub fn hash_from_hex_32(hex_str: &str) -> Result<Hash> {
    let hex_clean = hex_str.trim_start_matches("0x");
    let bytes = hex::decode(hex_clean)
        .map_err(|e| anyhow::anyhow!("Invalid hex: {}", e))?;
    if bytes.len() != 32 {
        return Err(anyhow::anyhow!("Expected 32 bytes, got {}", bytes.len()));
    }
    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(&bytes);
    Ok(Hash::new(hash_bytes))
}

/// Format a Unix timestamp as a human-readable date
pub fn format_timestamp(timestamp: u64) -> String {
    use std::time::{Duration, UNIX_EPOCH};
    
    let datetime = UNIX_EPOCH + Duration::from_secs(timestamp);
    let datetime = chrono::DateTime::<chrono::Local>::from(datetime);
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

// ===== Native Ethereum transaction sender using HTTP RPC =====

/// Send Ethereum mint transaction using native HTTP RPC (no external cast command needed)
#[allow(dead_code)]
pub fn send_ethereum_mint_stub() {}


/// Build a demo Merkle proof
/// 
/// In a real implementation, this would:
/// 1. Fetch the transaction receipt for the right_id
/// 2. Get the block containing the transaction
/// 3. Compute the Merkle path from the transaction to the block root
/// 4. Return the proof with the root, path, and leaf
///
/// For now, this builds a placeholder proof with proper structure.
pub fn build_demo_merkle_proof(right_id: Hash, commitment: Hash, depth: u8) -> Vec<u8> {
    use sha2::{Sha256, Digest};
    
    // Build a simple Merkle tree structure
    // The proof consists of:
    // - 4 bytes: depth
    // - 32 bytes: root hash
    // - 32 bytes: leaf hash (commitment)
    // - depth * 32 bytes: sibling hashes
    
    let mut proof = Vec::new();
    
    // Add depth
    proof.extend_from_slice(&depth.to_le_bytes());
    
    // Compute root as hash of right_id + commitment
    let mut hasher = Sha256::new();
    hasher.update(right_id.as_bytes());
    hasher.update(commitment.as_bytes());
    let root = hasher.finalize();
    proof.extend_from_slice(&root);
    
    // Add leaf (commitment)
    proof.extend_from_slice(commitment.as_bytes());
    
    // Generate placeholder sibling hashes
    // In a real implementation, these would be actual sibling nodes
    for i in 0..depth {
        let mut sibling_hasher = Sha256::new();
        sibling_hasher.update(&[i]); // Use index as unique input
        let sibling = sibling_hasher.finalize();
        proof.extend_from_slice(&sibling);
    }
    
    proof
}
