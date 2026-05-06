//! Cross-chain transfer command implementation (Phase 5 Compliant)
//!
//! Uses only csv-adapter facade APIs - no direct chain adapter dependencies.

use anyhow::Result;

use csv_adapter::CsvClient;
use csv_core::hash::Hash;
use csv_core::Chain;

use crate::config::{Chain as ConfigChain, Config};
use crate::output;
use crate::state::{TransferRecord, TransferStatus, UnifiedStateManager};

use super::to_core_chain;

/// Execute cross-chain transfer using only facade API
pub fn cmd_transfer(
    from: ConfigChain,
    to: ConfigChain,
    right_id: String,
    dest_owner: Option<String>,
    _config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    let from_chain = to_core_chain(from.clone());
    let to_chain = to_core_chain(to.clone());

    output::header(&format!("Cross-Chain Transfer: {:?} → {:?}", from_chain, to_chain));

    // Parse right ID
    let bytes = hex::decode(right_id.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid Right ID: {}", e))?;
    if bytes.len() < 32 {
        return Err(anyhow::anyhow!(
            "Invalid Right ID: expected at least 32 bytes, got {} bytes",
            bytes.len()
        ));
    }
    let mut right_bytes = [0u8; 32];
    right_bytes.copy_from_slice(&bytes[..32]);
    let right_id_hash = Hash::new(right_bytes);

    // Generate transfer ID
    let transfer_id = generate_transfer_id(&right_id_hash, &from_chain, &to_chain);

    // Check if we have the right
    if state.get_right(&right_id_hash.to_string()).is_none() {
        return Err(anyhow::anyhow!(
            "Right {} not found in local state",
            right_id_hash
        ));
    }

    // Get destination owner address
    let dest_owner_str = dest_owner.or_else(|| state.get_address(&from).map(|s| s.to_string()));

    if dest_owner_str.is_none() {
        return Err(anyhow::anyhow!(
            "No destination address specified and no wallet address found for {:?}",
            to_chain
        ));
    }

    let dest_addr = dest_owner_str.unwrap();

    output::info(&format!("Initiating transfer {}...", transfer_id));
    output::info(&format!("Destination address: {}", dest_addr));

    // Create client builder with source and destination chains
    let client = CsvClient::builder()
        .with_chain(from_chain)
        .with_chain(to_chain)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create CSV client: {}", e))?;

    // Use facade transfer manager
    let _transfers = client.transfers();

    // In a full implementation, this would execute the cross-chain transfer
    // using the facade's cross_chain method. For now, we record the intent.
    output::info("Cross-chain transfer requires chain adapter integration.");
    output::info("Using facade API - implementation pending in chain adapters.");

    // Clone for use in record after get_address call
    let from_chain_clone = from.clone();
    let sender = state.get_address(&from).map(|s| s.to_string());

    // Record transfer in state
    let transfer_record = TransferRecord {
        id: transfer_id.clone(),
        source_chain: from_chain_clone,
        dest_chain: to,
        right_id: right_id_hash.to_string(),
        sender_address: sender,
        destination_address: Some(dest_addr),
        source_tx_hash: None,
        source_fee: None,
        dest_tx_hash: None,
        dest_fee: None,
        destination_contract: None,
        proof: None,
        status: TransferStatus::Initiated,
        created_at: chrono::Utc::now().timestamp() as u64,
        completed_at: None,
    };

    state.add_transfer(transfer_record);

    output::success(&format!(
        "Transfer {} recorded. Status: Pending (facade integration required)",
        transfer_id
    ));

    Ok(())
}

/// Generate deterministic transfer ID
fn generate_transfer_id(right_id: &Hash, from: &Chain, to: &Chain) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(right_id.as_bytes());
    hasher.update(from.to_string().as_bytes());
    hasher.update(to.to_string().as_bytes());
    hasher.update(chrono::Utc::now().timestamp().to_le_bytes());

    format!("0x{}", hex::encode(hasher.finalize()))
}
