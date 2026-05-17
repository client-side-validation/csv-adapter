//! Bitcoin chain integration.
//!
//! Handles Bitcoin wallet operations and address derivation.

use bitcoin::{Address, Network, TweakedPublicKey, XOnlyPublicKey};
use csv_core::ChainId;

/// Get Bitcoin address format using proper Taproot (P2TR) encoding.
pub fn format_address(pubkey_bytes: &[u8], network: Network) -> String {
    let internal_key = XOnlyPublicKey::from_slice(&pubkey_bytes[..32])
        .expect("valid 32-byte x-only key");
    // Taproot key-path spend: tweak with empty script
    let tweaked = TweakedPublicKey::dangerous_assume_tweaked(internal_key);
    Address::p2tr_tweaked(tweaked, network).to_string()
}

/// Get chain type.
pub fn chain() -> ChainId {
    ChainId::new("bitcoin")
}
