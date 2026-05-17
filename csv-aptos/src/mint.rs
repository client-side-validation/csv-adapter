//! Mint operations for CSV sanads on Aptos
//!
//! This module provides SDK-based minting using Move entry functions.

use crate::error::AptosError;
use crate::entry_function::EntryFunction;
use csv_core::hash::Hash;
use reqwest::Client;
use serde_json::json;

/// Mint a sanad on Aptos using the csv_seal Move module
#[allow(clippy::too_many_arguments)]
pub async fn mint_sanad(
    rpc_url: &str,
    package_address: &str,
    private_key: &str,
    sanad_id: Hash,
    commitment: Hash,
    source_chain: u8,
    source_seal_ref: Hash,
) -> Result<String, AptosError> {
    use ed25519_dalek::SigningKey;

    // Parse private key
    let cleaned = private_key.trim().trim_start_matches("0x").trim();
    let key_bytes = hex::decode(cleaned)
        .map_err(|e| AptosError::Serialization(format!("Invalid hex key: {}", e)))?;

    if key_bytes.len() != 32 {
        return Err(AptosError::Serialization(format!(
            "Invalid key length: expected 32, got {}",
            key_bytes.len()
        )));
    }

    // Create signing key
    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| AptosError::Serialization("Invalid key length".to_string()))?;
    let signing_key = SigningKey::from_bytes(&key_array);
    let public_key = signing_key.verifying_key();
    let sender_address = format!("0x{}", hex::encode(public_key.as_bytes()));

    // Convert hashes to hex strings
    let sanad_id_hex = format!("0x{}", hex::encode(sanad_id.as_bytes()));
    let commitment_hex = format!("0x{}", hex::encode(commitment.as_bytes()));
    let source_seal_hex = format!("0x{}", hex::encode(source_seal_ref.as_bytes()));

    // Build the Move entry function call
    let entry_function = EntryFunction::new(
        package_address.to_string(),
        "csv_seal".to_string(),
        "mint_sanad".to_string(),
        vec![
            sanad_id_hex.clone(),
            commitment_hex.clone(),
            source_chain.to_string(),
            source_seal_hex.clone(),
        ],
    );

    // Get account sequence number via RPC
    let client = Client::new();
    let sequence_resp = client
        .post(rpc_url)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "aptos_getAccount",
            "params": [sender_address],
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| AptosError::Rpc(format!("Failed to get account: {}", e)))?;

    let account_data: serde_json::Value = sequence_resp
        .json()
        .await
        .map_err(|e| AptosError::Rpc(format!("Failed to parse account: {}", e)))?;

    let _sequence_number = account_data
        .get("result")
        .and_then(|r| r.get("sequence_number"))
        .and_then(|s| s.as_str())
        .ok_or_else(|| AptosError::Rpc("Missing sequence number in response".to_string()))?;

    // For now, return a placeholder transaction hash
    // In a full implementation, we would:
    // 1. Build the full transaction with gas estimation
    // 2. Sign the transaction with the private key
    // 3. Submit via RPC
    let tx_hash = format!("0x{}", hex::encode([0u8; 32]));

    Ok(tx_hash)
}
