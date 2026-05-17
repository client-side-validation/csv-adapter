//! Bitcoin chain integration.
//!
//! Handles Bitcoin wallet operations and address derivation.

use csv_core::ChainId;

/// Get Bitcoin address format.
pub fn format_address(pubkey_bytes: &[u8]) -> String {
    // Simplified Taproot address format
    format!("bc1q{}", hex::encode(&pubkey_bytes[..20]))
}

/// Get chain type.
pub fn chain() -> ChainId {
    ChainId::new("bitcoin")
}
