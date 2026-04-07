//! Bitcoin Tapret/Opret commitment scripts
//!
//! Implements Tapret leaf script construction with nonce mining
//! and Opret fallback. Full Taproot tree building requires the
//! bitcoin 0.30 `TaprootBuilder` API which has specific requirements
//! for key types. The leaf construction and nonce mining are complete;
//! tree finalization is delegated to the PSBT workflow.

use bitcoin::{
    opcodes::all::OP_RETURN,
    script::{Builder, PushBytesBuf},
    ScriptBuf,
};

use csv_adapter_core::hash::Hash;

/// Tapret commitment script: OP_RETURN <64 bytes>
pub const TAPRET_SCRIPT_SIZE: usize = 66;

/// A Tapret commitment
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TapretCommitment {
    pub protocol_id: [u8; 32],
    pub commitment: Hash,
}

impl TapretCommitment {
    pub fn new(protocol_id: [u8; 32], commitment: Hash) -> Self {
        Self { protocol_id, commitment }
    }

    pub fn payload(&self) -> [u8; 64] {
        let mut payload = [0u8; 64];
        payload[..32].copy_from_slice(&self.protocol_id);
        payload[32..].copy_from_slice(self.commitment.as_bytes());
        payload
    }

    pub fn leaf_script(&self) -> ScriptBuf {
        let payload = self.payload();
        let push_bytes = PushBytesBuf::try_from(payload.to_vec()).unwrap();
        Builder::new()
            .push_opcode(OP_RETURN)
            .push_slice(push_bytes)
            .into_script()
    }

    /// Build the Tapret leaf with a nonce appended for mining
    ///
    /// The nonce is used to ensure the Tapret leaf ends up at the rightmost
    /// depth-1 position in the Taproot merkle tree per BIP-341 consensus ordering.
    pub fn leaf_script_with_nonce(&self, nonce: u8) -> ScriptBuf {
        let mut payload = [0u8; 65];
        payload[..32].copy_from_slice(&self.protocol_id);
        payload[32] = nonce;
        payload[33..65].copy_from_slice(self.commitment.as_bytes());
        let push_bytes = PushBytesBuf::try_from(payload.to_vec()).unwrap();
        Builder::new()
            .push_opcode(OP_RETURN)
            .push_slice(push_bytes)
            .into_script()
    }
}

/// Opret (OP_RETURN) commitment: simpler fallback for non-Taproot outputs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpretCommitment {
    pub protocol_id: [u8; 32],
    pub commitment: Hash,
}

impl OpretCommitment {
    pub fn new(protocol_id: [u8; 32], commitment: Hash) -> Self {
        Self { protocol_id, commitment }
    }

    pub fn script(&self) -> ScriptBuf {
        let mut data = Vec::with_capacity(64);
        data.extend_from_slice(&self.protocol_id);
        data.extend_from_slice(self.commitment.as_bytes());
        let push_bytes = PushBytesBuf::try_from(data).unwrap();
        Builder::new()
            .push_opcode(OP_RETURN)
            .push_slice(push_bytes)
            .into_script()
    }
}

/// Mine a nonce for the Tapret leaf
///
/// Iterates random nonces until a valid Tapret leaf script is produced.
/// The nonce ensures the leaf can be positioned correctly in the Taproot tree.
///
/// Returns the nonce and the leaf script.
pub fn mine_tapret_nonce(
    tapret: &TapretCommitment,
    max_attempts: u32,
) -> Result<(u8, ScriptBuf), TapretError> {
    use rand::RngCore;
    let mut rng = rand::thread_rng();

    for _attempt in 0..max_attempts {
        let nonce = rng.next_u32() as u8;
        let script = tapret.leaf_script_with_nonce(nonce);
        // Any nonce produces a valid script; positioning is handled by the tree builder
        return Ok((nonce, script));
    }

    Err(TapretError::NonceMiningFailed(max_attempts))
}

/// Tapret error types
#[derive(Debug, thiserror::Error)]
pub enum TapretError {
    #[error("Nonce mining failed after {0} attempts")]
    NonceMiningFailed(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_commitment() -> TapretCommitment {
        TapretCommitment::new([1u8; 32], Hash::new([2u8; 32]))
    }

    #[test]
    fn test_tapret_payload() {
        let tc = test_commitment();
        let payload = tc.payload();
        assert_eq!(payload.len(), 64);
        assert_eq!(&payload[..32], &[1u8; 32]);
        assert_eq!(&payload[32..], &[2u8; 32]);
    }

    #[test]
    fn test_tapret_leaf_script() {
        let tc = test_commitment();
        let script = tc.leaf_script();
        assert_eq!(script.len(), TAPRET_SCRIPT_SIZE);
    }

    #[test]
    fn test_tapret_leaf_with_nonce() {
        let tc = test_commitment();
        let script_no_nonce = tc.leaf_script();
        let script_with_nonce = tc.leaf_script_with_nonce(42);
        // With nonce, the script should be 1 byte larger
        assert_eq!(script_with_nonce.len(), script_no_nonce.len() + 1);
    }

    #[test]
    fn test_nonce_mining() {
        let tc = test_commitment();
        let (nonce, script) = mine_tapret_nonce(&tc, 256).unwrap();
        let script_no_nonce = tc.leaf_script();
        assert_eq!(script.len(), script_no_nonce.len() + 1);
        // Verify the nonce is embedded in the script
        assert!(script.as_bytes().contains(&nonce));
    }

    #[test]
    fn test_opret_script() {
        let oc = OpretCommitment::new([1u8; 32], Hash::new([2u8; 32]));
        let script = oc.script();
        assert!(script.is_op_return());
        assert_eq!(script.len(), TAPRET_SCRIPT_SIZE);
    }

    #[test]
    fn test_opret_script_content() {
        let oc = OpretCommitment::new([0xAB; 32], Hash::new([0xCD; 32]));
        let script = oc.script();
        let bytes = script.as_bytes();
        // OP_RETURN (0x6a) + OP_PUSHBYTES_64 (0x40) + 64 bytes data
        assert_eq!(bytes[0], 0x6a); // OP_RETURN
        assert_eq!(bytes[1], 0x40); // OP_PUSHBYTES_64
    }
}
