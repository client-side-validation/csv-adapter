//! Aptos chain integration.

use csv_core::Chain;

/// Format Aptos address.
pub fn format_address(hash_bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(hash_bytes))
}

/// Get chain type.
pub fn chain() -> Chain {
    Chain::Aptos
}
