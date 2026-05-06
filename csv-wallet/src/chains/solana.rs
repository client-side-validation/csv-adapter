//! Solana chain integration.
//!
//! Handles Solana wallet operations and address derivation.

use csv_core::Chain;

/// Format Solana address.
pub fn format_address(pubkey_bytes: &[u8]) -> String {
    // Solana uses base58-encoded 32-byte public keys
    bs58::encode(pubkey_bytes).into_string()
}

/// Get chain type.
pub fn chain() -> Chain {
    Chain::Solana
}
