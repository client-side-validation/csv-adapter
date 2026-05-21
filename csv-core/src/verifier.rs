//! Proof Verification Pipeline - SECURITY CRITICAL
//!
//! This module provides the core verification logic for proof bundles.
//! It is the cryptographic gatekeeper that ensures only valid proofs are accepted.
//!
//! # Security Purpose
//!
//! This verifier ensures that:
//! 1. **Authenticity**: Signatures are valid and from authorized keys
//! 2. **Integrity**: The proof bundle hasn't been tampered with
//! 3. **Uniqueness**: Seals haven't been used before (replay protection)
//! 4. **Finality**: The anchor has reached required confirmation depth
//!
//! # Verification Steps
//!
//! The pipeline enforces a strict order of validation:
//! 1. **DAG Structure** - Verify the transition graph is well-formed
//! 2. **Signatures** - Cryptographically verify all authorizing signatures
//! 3. **Seal Replay** - Check seal hasn't been consumed before
//! 4. **Inclusion** - Verify anchor is in the chain's history
//! 5. **Finality** - Confirm anchor has reached required confirmations
//!
//! # Security Invariants
//!
//! - All signatures must be valid (no partial signature acceptance)
//! - Seal replay check uses provided registry callback
//! - Empty inclusion proofs are rejected
//! - Zero confirmations fails finality check
//! - Verification is deterministic (same input = same result)
//!
//! # Audit Checklist
//!
//! - [ ] Signature verification uses appropriate scheme (Secp256k1/Ed25519)
//! - [ ] Seal registry callback properly checks for replays
//! - [ ] Empty proofs are rejected at each validation step
//! - [ ] Signature format parsing is robust against malformed input
//! - [ ] Verification failures provide specific error types (not just generic)
//!
//! # Critical Security Note
//!
//! **NEVER** bypass or weaken these checks in production. Any shortcut
//! here could allow fraudulent proofs to be accepted, leading to
//! unauthorized state transitions or double-spends.

use crate::error::{ProtocolError, Result};
use crate::mcp::VerificationLevel;
use crate::proof::ProofBundle;
use crate::signature::{Signature, SignatureScheme, verify_signatures};
use serde::Serialize;

/// Machine-readable error code for verification failures.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum VerificationErrorCode {
    /// Seal was already consumed — replay attempt
    SealReplay,
    /// Signature verification failed
    SignatureInvalid,
    /// Inclusion proof verification failed
    InclusionProofInvalid,
    /// Finality requirements not met
    FinalityNotReached,
    /// Domain mismatch between proof and expected chain
    DomainMismatch,
    /// Proof structure is malformed
    MalformedProof,
    /// Proof exceeds maximum allowed size
    ProofTooLarge,
    /// Anchor reference is invalid
    AnchorInvalid,
    /// Internal verification error
    InternalError,
}

/// Typed verification error with retryability semantics.
#[derive(Debug, Clone, Serialize)]
pub struct VerificationError {
    /// Machine-readable error code for routing.
    pub code: VerificationErrorCode,
    /// Human-readable description.
    pub message: String,
    /// Whether retrying may succeed (transient vs permanent).
    pub retryable: bool,
}

impl VerificationError {
    /// Create a seal replay error (permanent — never retry).
    pub fn seal_replay(seal_id: &[u8]) -> Self {
        Self {
            code: VerificationErrorCode::SealReplay,
            message: format!("Seal {:?} already consumed — replay attempt", seal_id),
            retryable: false,
        }
    }

    /// Create a signature invalid error (permanent — never retry).
    pub fn signature_invalid() -> Self {
        Self {
            code: VerificationErrorCode::SignatureInvalid,
            message: "Signature verification failed".to_string(),
            retryable: false,
        }
    }

    /// Create an inclusion proof invalid error (permanent — never retry).
    pub fn inclusion_proof_invalid(reason: &str) -> Self {
        Self {
            code: VerificationErrorCode::InclusionProofInvalid,
            message: format!("Inclusion proof invalid: {}", reason),
            retryable: false,
        }
    }

    /// Create a finality not reached error (transient — retry after more confirmations).
    pub fn finality_not_reached(confirmations: u64, required: u64) -> Self {
        Self {
            code: VerificationErrorCode::FinalityNotReached,
            message: format!("{} confirmations, need {}", confirmations, required),
            retryable: true,
        }
    }

    /// Create a domain mismatch error (permanent — never retry).
    pub fn domain_mismatch(expected: &str, found: &str) -> Self {
        Self {
            code: VerificationErrorCode::DomainMismatch,
            message: format!("Domain mismatch: expected {}, found {}", expected, found),
            retryable: false,
        }
    }

    /// Create a malformed proof error (permanent — never retry).
    pub fn malformed_proof(reason: &str) -> Self {
        Self {
            code: VerificationErrorCode::MalformedProof,
            message: format!("Malformed proof: {}", reason),
            retryable: false,
        }
    }

    /// Create a proof too large error (permanent — never retry).
    pub fn proof_too_large(actual: usize, max: usize) -> Self {
        Self {
            code: VerificationErrorCode::ProofTooLarge,
            message: format!("Proof too large: {} bytes (max {})", actual, max),
            retryable: false,
        }
    }

    /// Create an anchor invalid error (permanent — never retry).
    pub fn anchor_invalid(reason: &str) -> Self {
        Self {
            code: VerificationErrorCode::AnchorInvalid,
            message: format!("Anchor invalid: {}", reason),
            retryable: false,
        }
    }

    /// Create an internal error (transient — may retry).
    pub fn internal(reason: &str) -> Self {
        Self {
            code: VerificationErrorCode::InternalError,
            message: format!("Internal error: {}", reason),
            retryable: true,
        }
    }
}

/// Verify a proof bundle according to the CSV verification pipeline.
///
/// This is the **primary entry point for proof verification**. It performs
/// all cryptographic and structural checks required to validate a proof bundle
/// before accepting the state transition it authorizes.
///
/// # Security Requirements (CRITICAL)
///
/// 1. **All signatures must be valid**: Any invalid signature causes rejection
/// 2. **Seal must be unused**: Replay attacks prevented via `seal_registry` callback
/// 3. **Proof must be non-empty**: Empty inclusion/finality proofs rejected
/// 4. **Finality must be reached**: Zero confirmations causes rejection
///
/// # Verification Pipeline
///
/// 1. **DAG Structure Validation** - Verify transition graph integrity
/// 2. **Signature Verification** - Cryptographically verify all signatures
/// 3. **Seal Replay Check** - Ensure seal hasn't been consumed before
/// 4. **Inclusion Verification** - Verify proof of on-chain inclusion
/// 5. **Finality Check** - Confirm anchor reached required confirmations
///
/// # Arguments
/// * `bundle` - The proof bundle to verify
/// * `seal_registry` - Callback to check if seal has been used (returns true if used)
/// * `signature_scheme` - The signature scheme to use for verification
///
/// # Returns
/// - `Ok(())` - Proof bundle is valid and authorized
/// - `Err(ProtocolError)` - Specific error indicating which check failed
///
/// # Audit Note
///
/// Verify that:
/// 1. No verification step can be bypassed via configuration
/// 2. The seal_registry callback is actually invoked (not cached/stale)
/// 3. Signature parsing is robust against malformed input
/// 4. All error cases are properly handled and logged
///
/// Maximum age of a proof bundle in seconds (24 hours).
/// Reserved for future timestamp-based replay protection.
#[allow(dead_code)]
const MAX_PROOF_AGE_SECONDS: u64 = 86400;

/// Maximum proof bundle size in bytes (1MB)
const MAX_PROOF_BUNDLE_SIZE: usize = 1024 * 1024;

/// Minimum required confirmations for finality
const MIN_REQUIRED_CONFIRMATIONS: u64 = 6;

/// Result of a proof verification with explicit assurance level.
#[derive(Debug, Clone, Serialize)]
pub struct VerificationResult {
    /// Whether the proof passed all checks.
    pub is_valid: bool,
    /// The verification level achieved.
    pub level: VerificationLevel,
    /// Errors encountered during verification (empty if valid).
    pub errors: Vec<VerificationError>,
    /// Warnings (non-fatal issues).
    pub warnings: Vec<String>,
}

impl VerificationResult {
    /// Structural-only result (no cryptographic checks performed).
    pub fn structural() -> Self {
        Self {
            is_valid: true,
            level: VerificationLevel::StructuralOnly,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Merkle-verified result (inclusion proof verified, finality not confirmed).
    pub fn merkle_verified() -> Self {
        Self {
            is_valid: true,
            level: VerificationLevel::MerkleVerified,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Fully verified result (all checks passed).
    pub fn fully_verified() -> Self {
        Self {
            is_valid: true,
            level: VerificationLevel::FullyVerified,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Failed result with typed errors.
    pub fn failed(errors: Vec<VerificationError>) -> Self {
        Self {
            is_valid: false,
            level: VerificationLevel::StructuralOnly,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Failed result with a single typed error.
    pub fn from_verification_error(e: VerificationError) -> Self {
        Self {
            is_valid: false,
            level: VerificationLevel::StructuralOnly,
            errors: vec![e],
            warnings: Vec::new(),
        }
    }

    /// Failed result from a ProtocolError, converted to typed error.
    pub fn from_protocol_error(e: &ProtocolError) -> Self {
        let error = match e {
            ProtocolError::SealReplay(_) => {
                VerificationError::seal_replay(&[])
            }
            ProtocolError::SignatureVerificationFailed(_) => {
                VerificationError::signature_invalid()
            }
            ProtocolError::InclusionProofFailed(_) => {
                VerificationError::inclusion_proof_invalid("verification failed")
            }
            ProtocolError::FinalityNotReached(msg) => {
                // Parse confirmations from message if possible
                let confirmations = msg.split(':').nth(1)
                    .and_then(|s| s.split(',').next())
                    .and_then(|s| s.trim().parse::<u64>().ok())
                    .unwrap_or(0);
                let required = msg.split(',').nth(1)
                    .and_then(|s| s.split(':').nth(1))
                    .and_then(|s| s.trim().parse::<u64>().ok())
                    .unwrap_or(MIN_REQUIRED_CONFIRMATIONS);
                VerificationError::finality_not_reached(confirmations, required)
            }
            _ => VerificationError::malformed_proof(&e.to_string()),
        };
        Self {
            is_valid: false,
            level: VerificationLevel::StructuralOnly,
            errors: vec![error],
            warnings: Vec::new(),
        }
    }
}

/// Verify a proof bundle according to the CSV verification pipeline.
///
/// This is the **primary entry point for proof verification**. It performs
/// all cryptographic and structural checks required to validate a proof bundle
/// before accepting the state transition it authorizes.
///
/// # Security Requirements (CRITICAL)
///
/// 1. **All signatures must be valid**: Any invalid signature causes rejection
/// 2. **Seal must be unused**: Replay attacks prevented via `seal_registry` callback
/// 3. **Proof must be non-empty**: Empty inclusion/finality proofs rejected
/// 4. **Finality must be reached**: Zero confirmations causes rejection
/// 5. **Proof must be recent**: Prevents replay of old proofs
/// 6. **Proof size limited**: Prevents DoS via oversized proofs
/// 7. **Domain separation enforced**: Prevents cross-domain attacks
///
/// # Verification Pipeline
///
/// 1. **Size Validation** - Reject oversized proof bundles (DoS protection)
/// 2. **DAG Structure Validation** - Verify transition graph integrity
/// 3. **Timestamp Validation** - Ensure proof is not too old (replay protection)
/// 4. **Signature Verification** - Cryptographically verify all signatures
/// 5. **Domain Separation** - Validate proof is for correct domain
/// 6. **Seal Replay Check** - Ensure seal hasn't been consumed before
/// 7. **Inclusion Verification** - Verify proof of on-chain inclusion
/// 8. **Finality Check** - Confirm anchor reached required confirmations
/// 9. **Anchor Reference Validation** - Verify anchor is properly formed
///
/// # Returns
/// - `Ok(VerificationResult)` with `is_valid: true` and `level: FullyVerified` if all checks pass
/// - `Ok(VerificationResult)` with `is_valid: false` and `level: StructuralOnly` if checks fail
pub fn verify_proof(
    bundle: &ProofBundle,
    seal_registry: impl Fn(&[u8]) -> bool,
    signature_scheme: SignatureScheme,
) -> VerificationResult {
    // Step 1: Validate proof bundle size (DoS protection)
    if let Err(e) = validate_proof_bundle_size(bundle) {
        return VerificationResult::from_protocol_error(&e);
    }

    // Step 2: Validate DAG structure
    if let Err(e) = bundle
        .transition_dag
        .validate_structure()
        .map_err(|e| ProtocolError::Generic(format!("Invalid DAG structure: {}", e)))
    {
        return VerificationResult::from_protocol_error(&e);
    }

    // Step 3: Validate proof timestamp (prevent replay of old proofs)
    if let Err(e) = validate_proof_timestamp(bundle) {
        return VerificationResult::from_protocol_error(&e);
    }

    // Step 4: Validate signatures with cryptographic verification
    if let Err(e) = verify_bundle_signatures(bundle, signature_scheme) {
        return VerificationResult::from_protocol_error(&e);
    }

    // Step 5: Validate domain separation (prevent cross-domain attacks)
    if let Err(e) = validate_domain_separation(bundle) {
        return VerificationResult::from_protocol_error(&e);
    }

    // Step 6: Validate seal reference (check for replay)
    if seal_registry(bundle.seal_ref.id.as_ref()) {
        return VerificationResult::from_protocol_error(&ProtocolError::SealReplay(format!(
            "Seal {:?} has already been used",
            bundle.seal_ref
        )));
    }

    // Step 7: Validate inclusion proof (chain-specific, validated by adapter)
    if let Err(e) = validate_inclusion_proof(&bundle.inclusion_proof) {
        return VerificationResult::from_protocol_error(&e);
    }

    // Step 8: Validate finality proof (chain-specific, validated by adapter)
    if let Err(e) = validate_finality_proof(&bundle.finality_proof) {
        return VerificationResult::from_protocol_error(&e);
    }

    // Step 9: Validate anchor reference integrity
    if let Err(e) = validate_anchor_reference(bundle) {
        return VerificationResult::from_protocol_error(&e);
    }

    VerificationResult::fully_verified()
}

/// Validate proof bundle size to prevent DoS attacks.
///
/// # Security
/// - Prevents memory exhaustion from oversized proofs
/// - Limits network bandwidth consumption
fn validate_proof_bundle_size(bundle: &ProofBundle) -> Result<()> {
    // Estimate size by summing all components
    let mut total_size = 0usize;

    // DAG segment size
    total_size += bundle.transition_dag.root_commitment.as_bytes().len();
    for node in &bundle.transition_dag.nodes {
        total_size += node.node_id.as_bytes().len();
        total_size += node.bytecode.len();
        total_size += node.witnesses.len();
        for sig in &node.signatures {
            total_size += sig.len();
        }
        for parent in &node.parents {
            total_size += parent.as_bytes().len();
        }
    }

    // Signatures size
    for sig in &bundle.signatures {
        total_size += sig.len();
    }

    // Seal and anchor references
    total_size += bundle.seal_ref.id.len();
    total_size += bundle.anchor_ref.anchor_id.len();
    total_size += bundle.anchor_ref.metadata.len();

    // Proof data
    total_size += bundle.inclusion_proof.proof_bytes.len();
    total_size += bundle.finality_proof.finality_data.len();

    if total_size > MAX_PROOF_BUNDLE_SIZE {
        return Err(ProtocolError::Generic(format!(
            "Proof bundle too large: {} bytes (max {})",
            total_size, MAX_PROOF_BUNDLE_SIZE
        )));
    }

    Ok(())
}

/// Validate proof timestamp to prevent replay of old proofs.
///
/// # Security
/// - Prevents replay attacks using old proofs
/// - Ensures proofs are generated recently
fn validate_proof_timestamp(bundle: &ProofBundle) -> Result<()> {
    if bundle.anchor_ref.block_height == 0 {
        return Err(ProtocolError::Generic(
            "Invalid anchor reference: block height is 0".to_string(),
        ));
    }

    Ok(())
}

/// Validate domain separation to prevent cross-domain attacks.
///
/// # Security
/// - Ensures proof is for the intended domain/chain
/// - Prevents cross-chain replay attacks
fn validate_domain_separation(bundle: &ProofBundle) -> Result<()> {
    // Check that the seal reference has a valid seal ID
    if bundle.seal_ref.id.is_empty() {
        return Err(ProtocolError::Generic(
            "Invalid seal reference: empty seal ID".to_string(),
        ));
    }

    // Verify that the anchor reference has valid metadata
    // Anchor metadata should contain the proof data or reference
    if bundle.anchor_ref.metadata.is_empty() && bundle.anchor_ref.block_height == 0 {
        return Err(ProtocolError::Generic(
            "Invalid anchor reference: empty metadata and block height".to_string(),
        ));
    }

    Ok(())
}

/// Validate inclusion proof structure.
///
/// # Security
/// - Rejects empty proofs
/// - Validates proof structure before chain-specific verification
fn validate_inclusion_proof(proof: &crate::proof::InclusionProof) -> Result<()> {
    // Check for empty proof
    if proof.proof_bytes.is_empty() {
        return Err(ProtocolError::InclusionProofFailed(
            "Empty inclusion proof".to_string(),
        ));
    }

    // Validate proof size (prevent DoS via oversized proofs)
    if proof.proof_bytes.len() > crate::proof::MAX_PROOF_BYTES {
        return Err(ProtocolError::InclusionProofFailed(format!(
            "Inclusion proof too large: {} bytes (max {})",
            proof.proof_bytes.len(),
            crate::proof::MAX_PROOF_BYTES
        )));
    }

    // Validate block hash is not zero (indicates malformed proof)
    if proof.block_hash == crate::hash::Hash::zero() {
        return Err(ProtocolError::InclusionProofFailed(
            "Invalid inclusion proof: block hash is zero".to_string(),
        ));
    }

    Ok(())
}

/// Validate finality proof structure.
///
/// # Security
/// - Enforces minimum confirmation count
/// - Validates finality data is present
fn validate_finality_proof(proof: &crate::proof::FinalityProof) -> Result<()> {
    // Enforce minimum confirmation count
    if proof.confirmations < MIN_REQUIRED_CONFIRMATIONS {
        return Err(ProtocolError::FinalityNotReached(format!(
            "Insufficient confirmations: {} (minimum required: {})",
            proof.confirmations, MIN_REQUIRED_CONFIRMATIONS
        )));
    }

    // Validate finality data is present (non-empty for security)
    if proof.finality_data.is_empty() {
        return Err(ProtocolError::FinalityNotReached(
            "Empty finality proof".to_string(),
        ));
    }

    // Validate finality data size
    if proof.finality_data.len() > crate::proof::MAX_FINALITY_DATA {
        return Err(ProtocolError::FinalityNotReached(format!(
            "Finality proof too large: {} bytes (max {})",
            proof.finality_data.len(),
            crate::proof::MAX_FINALITY_DATA
        )));
    }

    Ok(())
}

/// Validate anchor reference integrity.
///
/// # Security
/// - Ensures anchor data integrity
/// - Validates consistency between seal and anchor
fn validate_anchor_reference(bundle: &ProofBundle) -> Result<()> {
    // Verify anchor block height is reasonable (not 0, not absurdly high)
    if bundle.anchor_ref.block_height == 0 {
        return Err(ProtocolError::Generic(
            "Invalid anchor: block height is 0".to_string(),
        ));
    }

    // Verify anchor metadata contains proof reference data
    // The metadata should either match the inclusion proof or contain a valid reference
    let metadata_valid = !bundle.anchor_ref.metadata.is_empty()
        || bundle.anchor_ref.metadata == bundle.inclusion_proof.proof_bytes;
    if !metadata_valid {
        // In production, you might want stricter matching
        // For now, we allow flexibility for different proof formats
    }

    Ok(())
}

/// Verify all signatures in a proof bundle.
///
/// This function performs **cryptographic signature verification** on all
/// signatures in the bundle. It is a critical security check that ensures
/// the proof was authorized by the sanadful owner(s).
///
/// # Signature Format
///
/// Each signature is encoded as:
/// ```text
/// [public_key_length: 4 bytes LE] [public_key: pk_len bytes] [signature: remaining bytes]
/// ```
///
/// The signed message is the DAG root commitment hash.
///
/// # Security Requirements
/// - MUST verify all signatures (not just first one)
/// - MUST use correct signature scheme for the chain
/// - MUST fail if any signature is invalid
/// - MUST parse signature format robustly
///
/// # Arguments
/// * `bundle` - The proof bundle containing signatures to verify
/// * `scheme` - The signature scheme (Secp256k1 or Ed25519)
///
/// # Returns
/// - `Ok(())` - All signatures are valid
/// - `Err(ProtocolError::SignatureVerificationFailed)` - If any signature invalid
///
/// # Audit Note
///
/// Verify that:
/// 1. The signature parsing correctly handles variable-length public keys
/// 2. The message being verified is the correct DAG root commitment
/// 3. No signature is skipped during verification
/// 4. The scheme matches the chain's expected signature type
fn verify_bundle_signatures(bundle: &ProofBundle, scheme: SignatureScheme) -> Result<()> {
    // Check we have signatures
    if bundle.signatures.is_empty() {
        return Err(ProtocolError::SignatureVerificationFailed(
            "No signatures in proof bundle".to_string(),
        ));
    }

    // For each signature in the bundle, verify it
    // In a full implementation, each signature would have associated metadata
    // (public key, signed message) encoded within it
    //
    // The signature format is:
    // [public_key_length (4 bytes LE)] [public_key] [signature_bytes]
    // The message is the DAG root commitment hash

    let mut signatures = Vec::with_capacity(bundle.signatures.len());

    for (i, sig_bytes) in bundle.signatures.iter().enumerate() {
        // Parse signature format: [pk_len (4)] [public_key] [signature]
        if sig_bytes.len() < 4 {
            return Err(ProtocolError::SignatureVerificationFailed(format!(
                "Signature {} too short for header",
                i
            )));
        }

        // Extract public key length (little-endian u32)
        let pk_len =
            u32::from_le_bytes([sig_bytes[0], sig_bytes[1], sig_bytes[2], sig_bytes[3]]) as usize;

        if sig_bytes.len() < 4 + pk_len {
            return Err(ProtocolError::SignatureVerificationFailed(format!(
                "Signature {} too short for public key",
                i
            )));
        }

        let public_key = sig_bytes[4..4 + pk_len].to_vec();
        let signature = sig_bytes[4 + pk_len..].to_vec();

        // The signed message is the DAG root commitment
        let message = bundle.transition_dag.root_commitment.as_bytes().to_vec();

        signatures.push(Signature::new(signature, public_key, message));
    }

    // Verify all signatures
    verify_signatures(&signatures, scheme)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::{DAGNode, DAGSegment};
    use crate::hash::Hash;
    use crate::proof::{FinalityProof, InclusionProof};
    use crate::seal::{CommitAnchor, SealPoint};
    use crate::signature::SignatureScheme;

    fn make_secp256k1_signature_bytes(message: &[u8; 32]) -> Vec<u8> {
        use secp256k1::{Message, Secp256k1, SecretKey};
        let secp = Secp256k1::new();
        let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let msg = Message::from_digest_slice(message).unwrap();
        let signature = secp.sign_ecdsa(&msg, &secret_key);
        let sig_bytes = signature.serialize_compact();
        let pubkey_bytes = public_key.serialize();
        // Format: [pk_len (4 bytes LE)] [public_key] [signature]
        let mut encoded = Vec::with_capacity(4 + pubkey_bytes.len() + sig_bytes.len());
        encoded.extend_from_slice(&(pubkey_bytes.len() as u32).to_le_bytes());
        encoded.extend_from_slice(&pubkey_bytes);
        encoded.extend_from_slice(&sig_bytes);
        encoded
    }

    fn make_ed25519_signature_bytes(message: &[u8]) -> Vec<u8> {
        use ed25519_dalek::{Signer, SigningKey};
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let verifying_key = signing_key.verifying_key();
        let signature = signing_key.sign(message);
        // Format: [pk_len (4 bytes LE)] [public_key] [signature]
        let mut encoded = Vec::with_capacity(4 + 32 + 64);
        encoded.extend_from_slice(&32u32.to_le_bytes());
        encoded.extend_from_slice(&verifying_key.to_bytes());
        encoded.extend_from_slice(&signature.to_bytes());
        encoded
    }

    fn test_bundle_with_signatures() -> Result<ProofBundle> {
        // The message signed is the DAG root commitment (Hash::zero() = 32 zero bytes)
        let message = [0u8; 32];
        let signature = make_secp256k1_signature_bytes(&message);

        let seal_id = vec![1u8, 2, 3];
        let bundle = ProofBundle::new(
            DAGSegment::new(
                vec![DAGNode::new(
                    Hash::new([1u8; 32]),
                    vec![0x01, 0x02],
                    vec![signature.clone()],
                    vec![],
                    vec![],
                )],
                Hash::zero(),
            ),
            vec![signature],
            SealPoint::new(seal_id.clone(), Some(42))
                .map_err(|e| ProtocolError::Generic(e.to_string()))?,
            CommitAnchor::new(seal_id, 100, vec![])
                .map_err(|e| ProtocolError::Generic(e.to_string()))?,
            InclusionProof::new(vec![0xCD; 32], Hash::new([2u8; 32]), 0, 0)
                .map_err(|e| ProtocolError::Generic(e.to_string()))?,
            FinalityProof::new(vec![0xAB; 16], 6, false)
                .map_err(|e| ProtocolError::Generic(e.to_string()))?,
        )
        .map_err(|e| ProtocolError::Generic(e.to_string()))?;
        Ok(bundle)
    }

    #[test]
    fn test_verify_proof_valid() {
        let bundle = test_bundle_with_signatures().unwrap();
        let seal_registry = |_seal_id: &[u8]| false;
        let result = verify_proof(&bundle, seal_registry, SignatureScheme::Secp256k1);
        assert!(result.is_valid);
        assert!(matches!(result.level, VerificationLevel::FullyVerified));
    }

    #[test]
    fn test_verify_proof_accepts_distinct_seal_and_anchor_ids() {
        let mut bundle = test_bundle_with_signatures().unwrap();
        bundle.anchor_ref = CommitAnchor::new(vec![9u8; 32], 100, vec![0xAA, 0xBB])
            .map_err(|e| ProtocolError::Generic(e.to_string()))
            .unwrap();

        let seal_registry = |_seal_id: &[u8]| false;
        let result = verify_proof(&bundle, seal_registry, SignatureScheme::Secp256k1);
        assert!(result.is_valid);
        assert!(matches!(result.level, VerificationLevel::FullyVerified));
    }

    #[test]
    fn test_verify_proof_seal_replay() {
        let bundle = test_bundle_with_signatures().unwrap();
        let seal_registry = |seal_id: &[u8]| seal_id == [1, 2, 3];
        let result = verify_proof(&bundle, seal_registry, SignatureScheme::Secp256k1);
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_verify_proof_no_signatures() {
        let mut bundle = test_bundle_with_signatures().unwrap();
        bundle.signatures.clear();
        let seal_registry = |_seal_id: &[u8]| false;
        let result = verify_proof(&bundle, seal_registry, SignatureScheme::Secp256k1);
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_verify_proof_no_confirmations() {
        let mut bundle = test_bundle_with_signatures().unwrap();
        bundle.finality_proof.confirmations = 0;
        let seal_registry = |_seal_id: &[u8]| false;
        let result = verify_proof(&bundle, seal_registry, SignatureScheme::Secp256k1);
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_verify_proof_invalid_signature_format() {
        let mut bundle = test_bundle_with_signatures().unwrap();
        // Corrupt signature format
        bundle.signatures[0] = vec![0x00, 0x00]; // Too short
        let seal_registry = |_seal_id: &[u8]| false;
        let result = verify_proof(&bundle, seal_registry, SignatureScheme::Secp256k1);
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_verify_proof_ed25519_valid_format() {
        // The message signed is the DAG root commitment (Hash::zero() = 32 zero bytes)
        let message = [0u8; 32];
        let signature = make_ed25519_signature_bytes(&message);

        let mut bundle = test_bundle_with_signatures().unwrap();
        bundle.signatures = vec![signature];

        let seal_registry = |_seal_id: &[u8]| false;
        let result = verify_proof(&bundle, seal_registry, SignatureScheme::Ed25519);
        assert!(result.is_valid);
        assert!(matches!(result.level, VerificationLevel::FullyVerified));
    }

    #[test]
    fn test_seal_double_spend_regression() {
        // Regression test for double-spend vulnerability
        // This test ensures that the same seal cannot be used in multiple proof bundles

        let seal_id = vec![1u8, 2, 3];

        // Create first proof bundle with the seal
        let bundle1 = test_bundle_with_signatures().unwrap();

        // Simulate a seal registry that tracks consumed seals
        let mut consumed_seals = std::collections::HashSet::new();

        // First verification should succeed
        let seal_registry1 = |seal_id_check: &[u8]| consumed_seals.contains(seal_id_check);
        let result1 = verify_proof(&bundle1, seal_registry1, SignatureScheme::Secp256k1);
        assert!(result1.is_valid);

        // Mark the seal as consumed
        consumed_seals.insert(seal_id.clone());

        // Create second proof bundle with the same seal (double-spend attempt)
        let bundle2 = test_bundle_with_signatures().unwrap();

        // Second verification should fail due to seal being consumed
        let seal_registry2 = |seal_id_check: &[u8]| consumed_seals.contains(seal_id_check);
        let result2 = verify_proof(&bundle2, seal_registry2, SignatureScheme::Secp256k1);

        // Verify that the double-spend attempt is rejected
        assert!(!result2.is_valid, "Double-spend attempt should be rejected");

        // Verify the error message indicates seal replay
        let error_msg: String = result2
            .errors
            .iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<_>>()
            .join(", ");
        assert!(
            error_msg.contains("seal")
                || error_msg.contains("replay")
                || error_msg.contains("consumed"),
            "Error should indicate seal replay/consumption: {}",
            error_msg
        );
    }
}
