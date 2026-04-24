//! Utility functions for context operations.

/// Generate a random hex ID.
pub fn generate_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    format!("0x{}", hex::encode(bytes))
}
