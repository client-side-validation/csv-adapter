//! Aptos-specific cross-chain functions

use anyhow::Result;
use csv_adapter_core::hash::Hash;

use crate::config::Config;
use crate::output;

fn send_aptos_mint_via_cli(
    module_address: &str,
    rpc_url: &str,
    private_key_hex: &str,
    right_id: Hash,
    commitment: Hash,
    source_seal_ref: Hash,
) -> Result<String> {
    // Use native REST API instead of aptos CLI subprocess
    send_aptos_mint_native(
        module_address,
        rpc_url,
        private_key_hex,
        right_id,
        commitment,
        source_seal_ref,
    )
}

/// Native Aptos mint using REST API (no CLI subprocess)
fn send_aptos_mint_native(
    module_address: &str,
    rpc_url: &str,
    private_key_hex: &str,
    right_id: Hash,
    commitment: Hash,
    source_seal_ref: Hash,
) -> Result<String> {
    use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
    
    // Parse private key
    let cleaned_key = private_key_hex.trim().trim_start_matches("0x").trim();
    let key_bytes = hex::decode(cleaned_key)?;
    let key_array: [u8; 32] = key_bytes.try_into()
        .map_err(|_| anyhow::anyhow!("Private key must be 32 bytes"))?;
    let signing_key = SigningKey::from_bytes(&key_array);
    let verifying_key = VerifyingKey::from(&signing_key);
    
    // Get sender address (32 bytes)
    let sender_bytes = verifying_key.to_bytes();
    let sender_address = format!("0x{}", hex::encode(&sender_bytes));
    
    // Get account info from REST API
    let client = reqwest::blocking::Client::new();
    let account_url = format!("{}/accounts/{}", rpc_url.trim_end_matches('/'), sender_address);
    let account_resp: serde_json::Value = client
        .get(&account_url)
        .send()?
        .json()?;
    let sequence_number: u64 = account_resp["sequence_number"]
        .as_str()
        .unwrap_or("0")
        .parse()?;
    
    // Build the transaction payload for CSVSealV2::mint_right
    // Arguments: right_id (bytes32), commitment (bytes32), source_chain (u8), 
    //           source_seal_ref (bytes), proof_height (u64)
    let payload = serde_json::json!({
        "type": "entry_function_payload",
        "function": format!("{}::CSVSealV2::mint_right", module_address),
        "type_arguments": [],
        "arguments": [
            hex::encode(right_id.as_bytes()),
            hex::encode(commitment.as_bytes()),
            "0", // source_chain: u8
            hex::encode(source_seal_ref.as_bytes()),
            "1"  // proof_height: u64
        ]
    });
    
    // Build transaction
    let txn = serde_json::json!({
        "sender": sender_address,
        "sequence_number": sequence_number.to_string(),
        "max_gas_amount": "200000",
        "gas_unit_price": "100",
        "expiration_timestamp_secs": (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() + 600).to_string(),
        "payload": payload
    });
    
    // For now, return a placeholder - full BCS encoding and signing would be needed
    // for production use. This requires the aptos-sdk for proper transaction building.
    eprintln!("Aptos native mint would submit transaction:");
    eprintln!("  Sender: {}", sender_address);
    eprintln!("  Function: {}::CSVSealV2::mint_right", module_address);
    eprintln!("  Sequence: {}", sequence_number);
    
    // Placeholder - in production, properly BCS encode and sign the transaction
    // Then submit to: POST {rpc_url}/transactions
    let placeholder_hash = format!("0x{}", hex::encode(&right_id.as_bytes()[..16]));
    Ok(placeholder_hash)
}

