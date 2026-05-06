//! Mint operations for CSV rights on Solana
//!
//! This module provides SDK-based minting using solana-sdk for transaction building
//! and JSON-RPC for submission (avoids version compatibility issues).

use crate::error::{SolanaError, SolanaResult};
use csv_adapter_core::hash::Hash as CsvHash;

/// Mint a right on Solana using JSON-RPC
///
/// This uses solana-sdk for keypair/transaction construction and direct JSON-RPC for sending.
pub fn mint_right_from_hex_key(
    rpc_url: &str,
    program_id: &str,
    private_key_hex: &str,
    right_id: CsvHash,
    commitment: CsvHash,
    state_root: CsvHash,
    source_chain: u8,
    source_seal_ref: CsvHash,
) -> SolanaResult<String> {
    use serde_json::json;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    };

    // Parse private key from hex
    let cleaned = private_key_hex.trim().trim_start_matches("0x").trim();
    let key_bytes =
        hex::decode(cleaned).map_err(|e| SolanaError::Wallet(format!("Invalid hex key: {}", e)))?;

    if key_bytes.len() != 32 {
        return Err(SolanaError::Wallet(format!(
            "Invalid key length: expected 32, got {}",
            key_bytes.len()
        )));
    }

    // Convert to fixed-size array and create keypair
    let secret_key: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| SolanaError::Wallet("Invalid key length".to_string()))?;
    let payer = Keypair::new_from_array(secret_key);

    // Parse program ID
    let program_id = program_id
        .parse::<Pubkey>()
        .map_err(|e| SolanaError::InvalidProgramId(format!("Invalid program ID: {}", e)))?;

    // Get recent blockhash via JSON-RPC
    let client = reqwest::blocking::Client::new();
    let blockhash_resp = client
        .post(rpc_url)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "getLatestBlockhash",
            "params": [],
            "id": 1
        }))
        .send()
        .map_err(|e| SolanaError::Rpc(format!("Failed to get blockhash: {}", e)))?;

    let blockhash_json: serde_json::Value = blockhash_resp
        .json()
        .map_err(|e| SolanaError::Rpc(format!("Failed to parse blockhash: {}", e)))?;

    let blockhash_str = blockhash_json
        .get("result")
        .and_then(|r| {
            r.get("value")
                .and_then(|v| v.get("blockhash").and_then(|b| b.as_str()))
        })
        .ok_or_else(|| SolanaError::Rpc("Missing blockhash in response".to_string()))?;

    let blockhash = blockhash_str
        .parse()
        .map_err(|e| SolanaError::Rpc(format!("Invalid blockhash: {}", e)))?;

    // Derive the right PDA with correct seeds: ["right", owner, right_id]
    let (right_pda, _bump) = Pubkey::find_program_address(
        &[b"right", payer.pubkey().as_ref(), right_id.as_bytes()],
        &program_id,
    );

    // Build instruction data with correct Anchor discriminator
    // Anchor discriminator = first 8 bytes of sha256("global:instruction_name")
    // For mint_right: sha256("global:mint_right")[0..8]
    let discriminator = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"global:mint_right");
        let hash = hasher.finalize();
        hash[..8].to_vec()
    };
    let mut data = discriminator;
    data.extend_from_slice(right_id.as_bytes());
    data.extend_from_slice(commitment.as_bytes());
    data.extend_from_slice(state_root.as_bytes());
    data.push(source_chain);
    data.extend_from_slice(source_seal_ref.as_bytes());

    // Build instruction
    let instruction = Instruction::new_with_bytes(
        program_id,
        &data,
        vec![
            AccountMeta::new(right_pda, false),
            AccountMeta::new_readonly(payer.pubkey(), true),
            AccountMeta::new_readonly(
                "11111111111111111111111111111111"
                    .parse::<Pubkey>()
                    .unwrap(),
                false,
            ), // system program
        ],
    );

    // Build and sign transaction
    let mut transaction =
        Transaction::new_unsigned(solana_sdk::message::Message::new_with_blockhash(
            &[instruction],
            Some(&payer.pubkey()),
            &blockhash,
        ));
    transaction.sign(&[&payer], blockhash);

    // Serialize transaction
    let tx_bytes = bincode::serialize(&transaction)
        .map_err(|e| SolanaError::Serialization(format!("Failed to serialize: {}", e)))?;
    use base64::engine::{general_purpose, Engine as _};
    let tx_base64 = general_purpose::STANDARD.encode(&tx_bytes);

    // Send via JSON-RPC
    let send_resp = client
        .post(rpc_url)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "sendTransaction",
            "params": [tx_base64, {"encoding": "base64"}],
            "id": 1
        }))
        .send()
        .map_err(|e| SolanaError::Transaction(format!("Send request failed: {}", e)))?;

    let send_json: serde_json::Value = send_resp
        .json()
        .map_err(|e| SolanaError::Transaction(format!("Failed to parse response: {}", e)))?;

    if let Some(error) = send_json.get("error") {
        return Err(SolanaError::Transaction(format!("RPC error: {}", error)));
    }

    let signature = send_json
        .get("result")
        .and_then(|r| r.as_str())
        .ok_or_else(|| SolanaError::Transaction("Missing signature in response".to_string()))?;

    Ok(signature.to_string())
}
