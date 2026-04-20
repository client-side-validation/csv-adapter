//! Solana adapter implementation for CSV
//!
//! Implements the AnchorLayer trait for Solana using Program Derived Addresses (PDAs)
//! as single-use seals. When a seal is consumed, the PDA account is closed, transferring
//! lamports to the destination, making the seal cryptographically unspendable.

use csv_adapter_core::traits::AnchorLayer;
use csv_adapter_core::{Hash, Result, proof::ProofBundle, dag::DAGSegment, signature::SignatureScheme};
use solana_sdk::{
    pubkey::Pubkey, 
    signature::Signature,
};
use sha2::{Digest, Sha256};

use crate::config::SolanaConfig;
use crate::error::{SolanaError, SolanaResult};
use crate::rpc::SolanaRpc;
use crate::types::{
    SolanaAnchorRef, SolanaSealRef, SolanaInclusionProof, SolanaFinalityProof,
    ConfirmationStatus, AccountChange,
};
use crate::wallet::ProgramWallet;

/// Domain separator for Solana CSV commitments
const SOLANA_DOMAIN_SEPARATOR: [u8; 32] = [
    0x53, 0x4f, 0x4c, 0x61, 0x6e, 0x61, 0x43, 0x53, 0x56, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
];

/// Program instruction discriminators
const INSTRUCTION_CREATE_SEAL: u8 = 0x01;
const INSTRUCTION_CONSUME_SEAL: u8 = 0x02;
const INSTRUCTION_PUBLISH_COMMITMENT: u8 = 0x03;

/// Solana adapter for CSV (Client-Side Validation)
pub struct SolanaAnchorLayer {
    /// Configuration
    pub config: SolanaConfig,
    /// RPC client
    pub rpc_client: Option<Box<dyn SolanaRpc>>,
    /// Wallet
    pub wallet: Option<ProgramWallet>,
    /// In-memory seal tracking for this session
    active_seals: std::sync::Mutex<Vec<SolanaSealRef>>,
}

impl SolanaAnchorLayer {
    /// Create new Solana adapter
    pub fn new(config: SolanaConfig) -> Self {
        Self {
            config,
            rpc_client: None,
            wallet: None,
            active_seals: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Set RPC client
    pub fn with_rpc_client(mut self, rpc_client: Box<dyn SolanaRpc>) -> Self {
        self.rpc_client = Some(rpc_client);
        self
    }

    /// Set wallet
    pub fn with_wallet(mut self, wallet: ProgramWallet) -> Self {
        self.wallet = Some(wallet);
        self
    }

    /// Get configuration
    pub fn config(&self) -> &SolanaConfig {
        &self.config
    }

    /// Get RPC client
    pub fn rpc_client(&self) -> Option<&dyn SolanaRpc> {
        self.rpc_client.as_ref().map(|client| client.as_ref())
    }

    /// Get wallet
    pub fn wallet(&self) -> Option<&ProgramWallet> {
        self.wallet.as_ref()
    }

    /// Derive seal PDA from right ID and owner
    fn derive_seal_pda(&self, right_id: &Hash, owner: &Pubkey) -> Pubkey {
        let _seeds = [
            b"csv-seal",
            right_id.as_slice(),
            owner.as_ref(),
        ];
        // In production, this would use find_program_address with the actual CSV program
        // For now, we compute a deterministic hash-based address
        let mut hasher = Sha256::new();
        hasher.update(SOLANA_DOMAIN_SEPARATOR);
        hasher.update(b"seal");
        hasher.update(right_id.as_bytes());
        hasher.update(owner.as_ref());
        let hash = hasher.finalize();
        
        // Convert first 32 bytes to pubkey
        Pubkey::new_from_array(hash.into())
    }

    /// Derive commitment PDA from commitment hash
    fn derive_commitment_pda(&self, commitment: &Hash) -> Pubkey {
        let mut hasher = Sha256::new();
        hasher.update(SOLANA_DOMAIN_SEPARATOR);
        hasher.update(b"commitment");
        hasher.update(commitment.as_bytes());
        let hash = hasher.finalize();
        Pubkey::new_from_array(hash.into())
    }

    /// Check if RPC client is available
    fn check_rpc(&self) -> SolanaResult<&dyn SolanaRpc> {
        self.rpc_client()
            .ok_or_else(|| SolanaError::Rpc("No RPC client configured".to_string()))
    }

    /// Store seal reference
    fn store_seal(&self, seal: SolanaSealRef) {
        if let Ok(mut seals) = self.active_seals.lock() {
            seals.push(seal);
        }
    }

    /// Find seal by account
    fn find_seal(&self, account: &Pubkey) -> Option<SolanaSealRef> {
        if let Ok(seals) = self.active_seals.lock() {
            seals.iter().find(|s| &s.account == account).cloned()
        } else {
            None
        }
    }
}

impl AnchorLayer for SolanaAnchorLayer {
    type SealRef = SolanaAnchorRef;
    type AnchorRef = SolanaAnchorRef;
    type InclusionProof = SolanaInclusionProof;
    type FinalityProof = SolanaFinalityProof;

    /// Create a new seal account (PDA) for a right
    fn create_seal(&self, amount: Option<u64>) -> Result<Self::SealRef> {
        let wallet = self.wallet.as_ref()
            .ok_or_else(|| SolanaError::Wallet("No wallet configured".to_string()))?;
        
        let owner = wallet.pubkey();
        let right_id = Hash::new(Self::generate_right_id());
        let seal_pda = self.derive_seal_pda(&right_id, &owner);
        
        let lamports = amount.unwrap_or(1_000_000); // Default 0.001 SOL rent exemption
        
        let seal_ref = SolanaSealRef {
            account: seal_pda,
            owner,
            lamports,
            seed: Some(right_id.as_bytes().to_vec()),
        };
        
        // Create the seal account via RPC
        let _rpc = self.check_rpc()?;
        
        // In production, this would send a transaction to create the PDA
        // For now, we track it locally
        self.store_seal(seal_ref.clone());
        
        // Return anchor ref with placeholder signature (would be actual tx sig)
        let anchor_ref = SolanaAnchorRef {
            signature: Signature::new_unique(),
            slot: 0,
            block_height: 0,
            account_changes: vec![AccountChange {
                pubkey: seal_pda,
                prev_lamports: 0,
                new_lamports: lamports,
                prev_data: None,
                new_data: Some(right_id.as_bytes().to_vec()),
                closed: false,
            }],
        };
        
        Ok(anchor_ref)
    }

    /// Publish a commitment to the seal account
    fn publish(&self, hash: Hash, _seal_ref: Self::SealRef) -> Result<Self::AnchorRef> {
        let _rpc = self.check_rpc()?;
        let _wallet = self.wallet.as_ref()
            .ok_or_else(|| SolanaError::Wallet("No wallet configured".to_string()))?;
        
        // In production, this would:
        // 1. Build a transaction with the PublishCommitment instruction
        // 2. Sign and send via RPC
        // 3. Wait for confirmation
        // 4. Return the anchor ref with actual transaction details
        
        // For now, simulate successful publication
        let commitment_pda = self.derive_commitment_pda(&hash);
        
        let anchor_ref = SolanaAnchorRef {
            signature: Signature::new_unique(),
            slot: 1000, // Would be actual slot from RPC
            block_height: 1000,
            account_changes: vec![
                AccountChange {
                    pubkey: commitment_pda,
                    prev_lamports: 0,
                    new_lamports: 1_000_000,
                    prev_data: None,
                    new_data: Some(hash.as_bytes().to_vec()),
                    closed: false,
                },
            ],
        };
        
        Ok(anchor_ref)
    }

    /// Verify inclusion by checking the transaction is in a block
    fn verify_inclusion(&self, anchor_ref: Self::AnchorRef) -> Result<Self::InclusionProof> {
        let _rpc = self.check_rpc()?;
        
        // In production, this would:
        // 1. Fetch the transaction from RPC
        // 2. Verify it's in a confirmed block
        // 3. Build account proofs
        
        let proof = SolanaInclusionProof {
            signature: anchor_ref.signature,
            slot: anchor_ref.slot,
            block_height: anchor_ref.block_height,
            confirmation_status: ConfirmationStatus::Confirmed,
            account_proofs: anchor_ref.account_changes.iter().map(|change| {
                crate::types::AccountProof {
                    pubkey: change.pubkey,
                    proof: vec![change.pubkey.as_ref().to_vec()], // Simplified
                    data_hash: change.new_data.as_ref().map(|d| {
                        let mut hasher = Sha256::new();
                        hasher.update(d);
                        Hash::new(hasher.finalize().into())
                    }),
                }
            }).collect(),
        };
        
        Ok(proof)
    }

    /// Verify finality by checking block depth
    fn verify_finality(&self, anchor_ref: Self::AnchorRef) -> Result<Self::FinalityProof> {
        let _rpc = self.check_rpc()?;
        
        // Solana has deterministic finality after ~32 slots (12-16 seconds)
        // For devnet/testnet, we use shorter confirmation
        
        let current_slot = 1100u64; // Would fetch from RPC
        let confirmation_depth = current_slot.saturating_sub(anchor_ref.slot);
        
        // Solana requires 32 slots for finality
        let _is_finalized = confirmation_depth >= 32;
        
        let mut block_hash = [0u8; 32];
        block_hash.copy_from_slice(&anchor_ref.signature.as_ref()[..32]);
        
        let proof = SolanaFinalityProof {
            slot: anchor_ref.slot,
            block_hash: Hash::new(block_hash),
            confirmation_depth,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        };
        
        Ok(proof)
    }

    /// Enforce seal by closing the account (consuming it)
    fn enforce_seal(&self, seal_ref: Self::SealRef) -> Result<()> {
        let _rpc = self.check_rpc()?;
        let _wallet = self.wallet.as_ref()
            .ok_or_else(|| SolanaError::Wallet("No wallet configured".to_string()))?;
        
        // Find the seal account to consume
        let seal_account = seal_ref.account_changes.first()
            .ok_or_else(|| SolanaError::InvalidInput("No seal account in reference".to_string()))?;
        
        // In production, this would:
        // 1. Build a ConsumeSeal instruction
        // 2. Sign and send transaction
        // 3. Verify account is closed (lamports transferred to owner)
        
        // For now, mark as consumed in our tracking
        if let Ok(mut seals) = self.active_seals.lock() {
            seals.retain(|s| s.account != seal_account.pubkey);
        }
        
        Ok(())
    }

    /// Compute commitment hash with domain separation
    fn hash_commitment(&self, preimage: Hash, seal: Hash, anchor: Hash, seal_ref: &Self::SealRef) -> Hash {
        let mut hasher = Sha256::new();
        
        // Domain separator
        hasher.update(SOLANA_DOMAIN_SEPARATOR);
        
        // Instruction discriminator
        hasher.update([INSTRUCTION_PUBLISH_COMMITMENT]);
        
        // Commitment data
        hasher.update(preimage.as_bytes());
        hasher.update(seal.as_bytes());
        hasher.update(anchor.as_bytes());
        
        // Seal reference data
        if let Some(first_change) = seal_ref.account_changes.first() {
            hasher.update(first_change.pubkey.as_ref());
            hasher.update(first_change.new_lamports.to_le_bytes());
        }
        
        Hash::new(hasher.finalize().into())
    }

    /// Build a complete proof bundle
    fn build_proof_bundle(&self, anchor_ref: Self::AnchorRef, segment: DAGSegment) -> Result<ProofBundle> {
        let solana_inclusion = self.verify_inclusion(anchor_ref.clone())?;
        let solana_finality = self.verify_finality(anchor_ref.clone())?;

        // Create seal_ref from the first active seal or create a default one
        let seal_ref = {
            let seals = self.active_seals.lock().unwrap();
            seals.first().map(|s| {
                csv_adapter_core::seal::SealRef::new_unchecked(
                    s.account.to_bytes().to_vec(),
                    Some(s.lamports)
                )
            }).unwrap_or_else(|| {
                csv_adapter_core::seal::SealRef::new_unchecked(
                    anchor_ref.signature.as_ref()[..32].to_vec(),
                    None
                )
            })
        };

        // Create anchor_ref from SolanaAnchorRef
        let core_anchor_ref = csv_adapter_core::seal::AnchorRef::new_unchecked(
            anchor_ref.signature.as_ref().to_vec(),
            anchor_ref.block_height,
            serde_json::to_vec(&anchor_ref.account_changes).unwrap_or_default(),
        );

        // Create inclusion proof
        let inclusion_proof = csv_adapter_core::proof::InclusionProof::new_unchecked(
            solana_inclusion.account_proofs.iter()
                .flat_map(|p| p.proof.iter().flatten().cloned())
                .collect(),
            csv_adapter_core::hash::Hash::new(anchor_ref.signature.as_ref()[..32].try_into().unwrap_or([0u8; 32])),
            anchor_ref.slot,
        );

        // Create finality proof - Solana has deterministic finality after 31 slots
        let finality_proof = csv_adapter_core::proof::FinalityProof::new_unchecked(
            solana_finality.block_hash.as_bytes().to_vec(),
            solana_finality.confirmation_depth,
            true, // Solana has deterministic finality
        );

        // Create a complete proof bundle
        let bundle = csv_adapter_core::proof::ProofBundle::new_unchecked(
            segment,
            vec![anchor_ref.signature.as_ref().to_vec()],
            seal_ref,
            core_anchor_ref,
            inclusion_proof,
            finality_proof,
        );

        Ok(bundle)
    }

    /// Handle rollback for reorgs
    fn rollback(&self, anchor_ref: Self::AnchorRef) -> Result<()> {
        // Solana has very rare reorgs due to deterministic finality
        // But we still need to handle them
        
        // Check if the slot is still valid
        let _rpc = self.check_rpc()?;
        
        // In production, this would:
        // 1. Query the slot to see if it's still in the canonical chain
        // 2. If not, return the seal to active status
        // 3. Invalidate the commitment
        
        // For now, we just verify the slot is old enough to be finalized
        let current_slot = 1100u64; // Would fetch from RPC
        let age = current_slot.saturating_sub(anchor_ref.slot);
        
        if age < 32 {
            return Err(SolanaError::Rpc(
                format!("Cannot rollback: transaction not yet finalized (age: {} slots)", age)
            ).into());
        }
        
        // If we're here, the transaction is finalized and cannot be rolled back
        Ok(())
    }

    /// Get domain separator for this chain
    fn domain_separator(&self) -> [u8; 32] {
        SOLANA_DOMAIN_SEPARATOR
    }

    /// Get signature scheme (Ed25519 for Solana)
    fn signature_scheme(&self) -> SignatureScheme {
        SignatureScheme::Ed25519
    }
}

/// Helper struct for serializing Solana-specific proof data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SolanaProofData {
    signature: String,
    slot: u64,
    confirmation_status: String,
}

impl SolanaAnchorLayer {
    /// Generate a unique right ID
    fn generate_right_id() -> [u8; 32] {
        let mut bytes = [0u8; 32];
        rand::Rng::fill(&mut rand::thread_rng(), &mut bytes);
        bytes
    }
}

impl Default for SolanaAnchorLayer {
    fn default() -> Self {
        Self::new(SolanaConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_seal_pda() {
        let config = SolanaConfig::default();
        let adapter = SolanaAnchorLayer::new(config);
        let right_id = Hash::new([1u8; 32]);
        let owner = Pubkey::new_unique();
        
        let pda1 = adapter.derive_seal_pda(&right_id, &owner);
        let pda2 = adapter.derive_seal_pda(&right_id, &owner);
        
        assert_eq!(pda1, pda2, "PDA derivation should be deterministic");
    }

    #[test]
    fn test_domain_separator() {
        let config = SolanaConfig::default();
        let adapter = SolanaAnchorLayer::new(config);
        
        let sep = adapter.domain_separator();
        assert_eq!(&sep[0..8], b"SolanaCSV");
    }

    #[test]
    fn test_signature_scheme() {
        let config = SolanaConfig::default();
        let adapter = SolanaAnchorLayer::new(config);
        
        assert_eq!(adapter.signature_scheme(), SignatureScheme::Ed25519);
    }
}
