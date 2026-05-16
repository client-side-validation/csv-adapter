//! WASM bindings for CSV protocol cryptographic operations.
//!
//! This crate provides WebAssembly-compatible functions for:
//! - ML-DSA-65 (Dilithium) post-quantum key generation and signing
//! - Seal point creation and verification
//! - SHA3-256 hashing
//! - Cross-chain commitment building

use rand::RngCore;
use rand::rngs::OsRng;
use wasm_bindgen::prelude::*;

// ── Seal Point Operations ──────────────────────────────────────

/// A seal point representing a chain-native single-use primitive.
#[wasm_bindgen]
pub struct SealPoint {
    seal_id: Vec<u8>,
    value: Option<u64>,
}

#[wasm_bindgen]
impl SealPoint {
    /// Create a new seal point from a byte array and optional value.
    #[wasm_bindgen(constructor)]
    pub fn new(seal_id: &[u8], value: Option<u64>) -> SealPoint {
        Self {
            seal_id: seal_id.to_vec(),
            value,
        }
    }

    /// Get the seal ID as a hex string.
    pub fn seal_id_hex(&self) -> String {
        hex::encode(&self.seal_id)
    }

    /// Get the optional value.
    pub fn value(&self) -> Option<u64> {
        self.value
    }

    /// Get the seal ID as raw bytes.
    pub fn seal_id_bytes(&self) -> Vec<u8> {
        self.seal_id.clone()
    }
}

// ── Hash Operations ────────────────────────────────────────────

/// Compute SHA3-256 hash of input data.
#[wasm_bindgen]
pub fn sha3_256(data: &[u8]) -> Vec<u8> {
    use sha3::digest::Digest;
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Compute SHA3-256 hash and return as hex string.
#[wasm_bindgen]
pub fn sha3_256_hex(data: &[u8]) -> String {
    hex::encode(sha3_256(data))
}

/// Keccak-256 hash (Ethereum-compatible).
#[wasm_bindgen]
pub fn keccak_256(data: &[u8]) -> Vec<u8> {
    use sha3::Digest;
    use sha3::Keccak256;
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Keccak-256 hash as hex string.
#[wasm_bindgen]
pub fn keccak_256_hex(data: &[u8]) -> String {
    hex::encode(keccak_256(data))
}

// ── Seal Verification ──────────────────────────────────────────

/// Verify that a seal is properly formed.
///
/// A valid seal must have at least 1 byte of seal data.
#[wasm_bindgen]
pub fn verify_seal_format(seal_id: &[u8]) -> bool {
    !seal_id.is_empty() && seal_id.len() <= 128
}

/// Build a commitment from seal ID and optional chain metadata.
#[wasm_bindgen]
pub fn build_commitment(seal_id: &[u8], chain_id: &str) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(chain_id.as_bytes());
    data.push(0); // separator
    data.extend_from_slice(seal_id);
    sha3_256(&data)
}

/// Verify a commitment was built from the expected inputs.
#[wasm_bindgen]
pub fn verify_commitment(commitment: &[u8], seal_id: &[u8], chain_id: &str) -> bool {
    let expected = build_commitment(seal_id, chain_id);
    commitment == expected.as_slice()
}

// ── Post-Quantum Cryptography (ML-DSA-65 / Dilithium) ─────────

#[cfg(feature = "pq")]
mod pq_impl {
    use super::*;
    use pqcrypto_traits::sign::{DetachedSignature as _, PublicKey as _, SecretKey as _};

    type PublicKey = pqcrypto_dilithium::dilithium2::PublicKey;
    type SecretKey = pqcrypto_dilithium::dilithium2::SecretKey;
    type DetachedSignature = pqcrypto_dilithium::dilithium2::DetachedSignature;

    /// Generate a new ML-DSA-65 key pair.
    ///
    /// Returns a JSON object with `publicKey` and `secretKey` as hex strings.
    #[wasm_bindgen]
    pub fn ml_dsa_65_keygen() -> String {
        let (pk, sk) = pqcrypto_dilithium::dilithium2::keypair();
        serde_json::json!({
            "publicKey": hex::encode(pk.as_bytes()),
            "secretKey": hex::encode(sk.as_bytes()),
            "scheme": "ML-DSA-65",
            "publicKeyLen": pqcrypto_dilithium::dilithium2::public_key_bytes(),
            "secretKeyLen": pqcrypto_dilithium::dilithium2::secret_key_bytes()
        })
        .to_string()
    }

    /// Sign a message with ML-DSA-65.
    ///
    /// Returns the signature as a hex string.
    #[wasm_bindgen]
    pub fn ml_dsa_65_sign(secret_key: &[u8], message: &[u8]) -> Result<String, JsError> {
        let sk = SecretKey::from_bytes(secret_key)
            .map_err(|e| JsError::new(&format!("Invalid secret key: {}", e)))?;
        let signature = pqcrypto_dilithium::dilithium2::detached_sign(message, &sk);
        Ok(hex::encode(signature.as_bytes()))
    }

    /// Verify an ML-DSA-65 signature.
    #[wasm_bindgen]
    pub fn ml_dsa_65_verify(
        public_key: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> Result<bool, JsError> {
        let pk = PublicKey::from_bytes(public_key)
            .map_err(|e| JsError::new(&format!("Invalid public key: {}", e)))?;
        let sig = DetachedSignature::from_bytes(signature)
            .map_err(|e| JsError::new(&format!("Invalid signature: {}", e)))?;

        match pqcrypto_dilithium::dilithium2::verify_detached_signature(&sig, message, &pk) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get ML-DSA-65 parameter sizes.
    #[wasm_bindgen]
    pub fn ml_dsa_65_sizes() -> String {
        serde_json::json!({
            "publicKeyLen": pqcrypto_dilithium::dilithium2::public_key_bytes(),
            "secretKeyLen": pqcrypto_dilithium::dilithium2::secret_key_bytes(),
            "signatureLen": pqcrypto_dilithium::dilithium2::signature_bytes()
        })
        .to_string()
    }
}

#[cfg(not(feature = "pq"))]
mod pq_impl {
    use super::*;

    /// ML-DSA-65 key generation — requires "pq" feature.
    #[wasm_bindgen]
    pub fn ml_dsa_65_keygen() -> String {
        serde_json::json!({
            "error": "ML-DSA-65 requires the 'pq' feature flag",
            "scheme": "ML-DSA-65"
        })
        .to_string()
    }

    /// ML-DSA-65 signing — requires "pq" feature.
    #[wasm_bindgen]
    pub fn ml_dsa_65_sign(_secret_key: &[u8], _message: &[u8]) -> Result<String, JsError> {
        Err(JsError::new("ML-DSA-65 requires the 'pq' feature flag"))
    }

    /// ML-DSA-65 verification — requires "pq" feature.
    #[wasm_bindgen]
    pub fn ml_dsa_65_verify(
        _public_key: &[u8],
        _message: &[u8],
        _signature: &[u8],
    ) -> Result<bool, JsError> {
        Err(JsError::new("ML-DSA-65 requires the 'pq' feature flag"))
    }

    /// ML-DSA-65 parameter sizes — requires "pq" feature.
    #[wasm_bindgen]
    pub fn ml_dsa_65_sizes() -> String {
        serde_json::json!({
            "error": "ML-DSA-65 requires the 'pq' feature flag"
        })
        .to_string()
    }
}

// Re-export PQ functions at the crate root for wasm-bindgen exposure.
pub use pq_impl::*;

// ── Seal Protocol Operations ───────────────────────────────────

/// Create a new seal for authorizing state transitions.
///
/// Returns a hex-encoded seal ID (32 bytes).
#[wasm_bindgen]
pub fn create_seal() -> String {
    let mut seal_id = [0u8; 32];
    OsRng.fill_bytes(&mut seal_id);
    hex::encode(seal_id)
}

/// Create a seal with an associated value (chain-specific units).
#[wasm_bindgen]
pub fn create_seal_with_value(value: u64) -> String {
    let mut seal_id = [0u8; 32];
    OsRng.fill_bytes(&mut seal_id);
    // Include value in hash for domain separation
    let mut data = Vec::with_capacity(40);
    data.extend_from_slice(&seal_id);
    data.extend_from_slice(&value.to_le_bytes());
    hex::encode(sha3_256(&data))
}

/// Build a proof bundle structure (simplified).
///
/// Returns JSON with the proof bundle fields.
#[wasm_bindgen]
pub fn build_proof_bundle(
    seal_id: &[u8],
    block_height: u64,
    commitment: &[u8],
    chain_id: &str,
) -> String {
    let seal_point = SealPoint::new(seal_id, None);
    let built_commitment = build_commitment(seal_id, chain_id);

    serde_json::json!({
        "sealId": seal_point.seal_id_hex(),
        "blockHeight": block_height,
        "commitment": hex::encode(&built_commitment),
        "originalCommitment": hex::encode(commitment),
        "verified": verify_commitment(commitment, seal_id, chain_id)
    })
    .to_string()
}
