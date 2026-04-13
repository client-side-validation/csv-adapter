//! Sui chain integration.

use csv_adapter_core::Chain;

/// Format Sui address.
pub fn format_address(hash_bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(&hash_bytes[..]))
}

/// Get chain type.
pub fn chain() -> Chain {
    Chain::Sui
}
