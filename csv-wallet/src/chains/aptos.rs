//! Aptos chain integration.

use csv_core::ChainId;

/// Format Aptos address.
pub fn format_address(hash_bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(hash_bytes))
}

/// Get chain type.
pub fn chain() -> ChainId {
    ChainId::new("aptos")
}
