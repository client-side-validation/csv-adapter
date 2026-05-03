//! Cross-Chain Right Transfer
//!
//! Implements the lock-and-prove protocol for transferring Rights between chains:
//! 1. Lock — Source chain consumes seal, emits CrossChainLockEvent
//! 2. Prove — Client generates inclusion proof
//! 3. Verify — Destination chain verifies proof, checks registry, mints new Right
//! 4. Registry — Records transfer, prevents cross-chain double-spend

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::hash::Hash;
use crate::protocol_version::Chain;
use crate::right::{OwnershipProof, Right};
use crate::seal::SealRef;

/// Chain identifier alias for cross-chain transfers.
///
/// This re-exports the canonical [`Chain`] from the protocol contract.
/// All cross-chain operations use this type.
pub type ChainId = Chain;

/// Event emitted when a Right is locked on the source chain for cross-chain transfer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossChainLockEvent {
    /// The Right being locked
    pub right_id: Hash,
    /// The commitment hash of the Right
    pub commitment: Hash,
    /// The owner who initiated the lock
    pub owner: OwnershipProof,
    /// Source chain where the Right is being locked
    pub source_chain: ChainId,
    /// Destination chain for the transfer
    pub destination_chain: ChainId,
    /// Destination owner (may differ from source owner)
    pub destination_owner: OwnershipProof,
    /// Source chain's seal reference (consumed during lock)
    pub source_seal: SealRef,
    /// Source transaction hash
    pub source_tx_hash: Hash,
    /// Source block height
    pub source_block_height: u64,
    /// Unix timestamp of the lock event
    pub timestamp: u64,
}

/// Inclusion proof — chain-specific format.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InclusionProof {
    /// Bitcoin: Merkle branch + block header
    Bitcoin(BitcoinMerkleProof),
    /// Ethereum: MPT receipt proof
    Ethereum(EthereumMPTProof),
    /// Sui: Checkpoint certification
    Sui(SuiCheckpointProof),
    /// Aptos: Ledger info proof
    Aptos(AptosLedgerProof),
    /// Solana: Slot-based inclusion proof
    Solana(SolanaSlotProof),
    /// ZK proof: chain-agnostic zero-knowledge seal proof
    ZkSeal(ZkSealProof),
}

/// Bitcoin Merkle proof of transaction inclusion in a block.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BitcoinMerkleProof {
    /// Transaction ID
    pub txid: [u8; 32],
    /// Merkle branch nodes
    pub merkle_branch: Vec<[u8; 32]>,
    /// Serialized block header
    pub block_header: Vec<u8>,
    /// Block height
    pub block_height: u64,
    /// Number of confirmations
    pub confirmations: u64,
}

/// Ethereum MPT proof of receipt inclusion in state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct EthereumMPTProof {
    /// Transaction hash
    pub tx_hash: [u8; 32],
    /// Receipt root hash
    pub receipt_root: [u8; 32],
    /// RLP-encoded receipt
    pub receipt_rlp: Vec<u8>,
    /// MPT proof nodes
    pub merkle_nodes: Vec<Vec<u8>>,
    /// Serialized block header
    pub block_header: Vec<u8>,
    /// Log index in the receipt
    pub log_index: u64,
    /// Number of confirmations
    pub confirmations: u64,
}

/// Sui checkpoint proof of transaction effects certification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct SuiCheckpointProof {
    /// Transaction digest
    pub tx_digest: [u8; 32],
    /// Checkpoint sequence number
    pub checkpoint_sequence: u64,
    /// Checkpoint contents hash
    pub checkpoint_contents_hash: [u8; 32],
    /// Transaction effects bytes
    pub effects: Vec<u8>,
    /// Event bytes
    pub events: Vec<u8>,
    /// Whether the checkpoint is certified
    pub certified: bool,
}

/// Aptos ledger info proof of transaction execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AptosLedgerProof {
    /// Transaction version
    pub version: u64,
    /// Transaction proof bytes
    pub transaction_proof: Vec<u8>,
    /// Ledger info bytes
    pub ledger_info: Vec<u8>,
    /// Event bytes
    pub events: Vec<u8>,
    /// Whether the transaction succeeded
    pub success: bool,
}

/// Solana slot-based proof of transaction inclusion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct SolanaSlotProof {
    /// Slot number where the transaction was included
    pub slot: u64,
    /// Transaction signature
    pub signature: Vec<u8>,
    /// Block hash of the slot
    pub block_hash: [u8; 32],
    /// Number of confirmations
    pub confirmations: u64,
    /// Whether the slot is finalized
    pub finalized: bool,
    /// Account keys involved in the transaction
    pub account_keys: Vec<Vec<u8>>,
    /// Instruction data hash
    pub instruction_data_hash: [u8; 32],
}

/// ZK seal proof for chain-agnostic verification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZkSealProof {
    /// The ZK proof bytes
    pub proof_bytes: Vec<u8>,
    /// Verifier key for proof verification
    pub verifier_key: VerifierKey,
    /// Public inputs from the proof
    pub public_inputs: ZkPublicInputs,
}

/// Verifier key for ZK proof verification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifierKey {
    /// Chain this verifier is for
    pub chain: ChainId,
    /// Verifier key bytes
    pub key_bytes: Vec<u8>,
    /// Proof system type
    pub proof_system: String,
    /// Key version
    pub version: u32,
}

/// Public inputs from a ZK seal proof.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZkPublicInputs {
    /// The seal reference being proven
    pub seal_ref: SealRef,
    /// Block hash where the seal was consumed
    pub block_hash: Hash,
    /// Commitment hash bound to the proof
    pub commitment: Hash,
    /// Source chain identifier
    pub source_chain: ChainId,
    /// Block height
    pub block_height: u64,
    /// Unix timestamp
    pub timestamp: u64,
}

/// Finality proof confirming source transaction is finalized.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossChainFinalityProof {
    /// Source chain identifier
    pub source_chain: ChainId,
    /// Block/checkpoint/ledger height of the transaction
    pub height: u64,
    /// Current height on the source chain
    pub current_height: u64,
    /// Whether finality depth has been achieved
    pub is_finalized: bool,
    /// Required finality depth in blocks
    pub depth: u64,
}

/// Complete proof bundle submitted to the destination chain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossChainTransferProof {
    /// The lock event data from the source chain
    pub lock_event: CrossChainLockEvent,
    /// Inclusion proof (chain-specific format)
    pub inclusion_proof: InclusionProof,
    /// Finality proof confirming source transaction
    pub finality_proof: CrossChainFinalityProof,
    /// Source chain's state root at the lock block
    pub source_state_root: Hash,
}

/// Entry in the cross-chain seal registry recording a completed transfer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossChainRegistryEntry {
    /// The Right's unique ID (preserved across chains)
    pub right_id: Hash,
    /// Source chain identifier
    pub source_chain: ChainId,
    /// Source chain's seal reference
    pub source_seal: SealRef,
    /// Destination chain identifier
    pub destination_chain: ChainId,
    /// Destination chain's seal reference
    pub destination_seal: SealRef,
    /// Lock transaction hash on source chain
    pub lock_tx_hash: Hash,
    /// Mint transaction hash on destination chain
    pub mint_tx_hash: Hash,
    /// Unix timestamp of the transfer
    pub timestamp: u64,
}

/// Result of a successful cross-chain transfer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossChainTransferResult {
    /// The new Right created on the destination chain
    pub destination_right: Right,
    /// The destination chain's seal reference
    pub destination_seal: SealRef,
    /// Registry entry recording the transfer
    pub registry_entry: CrossChainRegistryEntry,
}

/// Errors that can occur during cross-chain transfer.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[allow(missing_docs)]
pub enum CrossChainError {
    #[error("Right already locked on source chain")]
    AlreadyLocked,
    #[error("Right already exists on destination chain")]
    AlreadyMinted,
    #[error("Invalid inclusion proof")]
    InvalidInclusionProof,
    #[error("Insufficient finality: {0} confirmations, need {1}")]
    InsufficientFinality(u64, u64),
    #[error("Ownership proof verification failed")]
    InvalidOwnership,
    #[error("Lock event does not match expected data")]
    LockEventMismatch,
    #[error("Cross-chain registry error: {0}")]
    RegistryError(String),
    #[error("Unsupported chain pair: {0} → {1}")]
    UnsupportedChainPair(ChainId, ChainId),
}

/// Trait for locking a Right on a source chain.
///
/// Consumes the Right's seal and returns the lock event data + inclusion proof.
pub trait LockProvider {
    /// Lock a Right for cross-chain transfer.
    ///
    /// # Arguments
    /// * `right_id` — The unique identifier of the Right
    /// * `commitment` — The Right's commitment hash
    /// * `owner` — Current owner's ownership proof
    /// * `destination_chain` — Target chain ID
    /// * `destination_owner` — New owner on destination chain
    ///
    /// # Returns
    /// Lock event data and inclusion proof (chain-specific format)
    fn lock_right(
        &self,
        right_id: Hash,
        commitment: Hash,
        owner: OwnershipProof,
        destination_chain: ChainId,
        destination_owner: OwnershipProof,
    ) -> Result<(CrossChainLockEvent, InclusionProof), CrossChainError>;
}

/// Trait for verifying cross-chain transfer proofs.
pub trait TransferVerifier {
    /// Verify a cross-chain transfer proof.
    ///
    /// # Checks
    /// 1. Inclusion proof is valid (source chain finalized)
    /// 2. Seal NOT in CrossChainSealRegistry (no double-spend)
    /// 3. Ownership proof valid (owner signature matches)
    /// 4. Lock event matches expected right_id and commitment
    fn verify_transfer_proof(&self, proof: &CrossChainTransferProof)
        -> Result<(), CrossChainError>;
}

/// Trait for minting a Right on a destination chain.
pub trait MintProvider {
    /// Mint a new Right from a verified cross-chain transfer proof.
    ///
    /// Creates a new Right with the same commitment and state
    /// but a new seal on the destination chain.
    fn mint_right(
        &self,
        proof: &CrossChainTransferProof,
    ) -> Result<CrossChainTransferResult, CrossChainError>;
}

/// Cross-chain transfer orchestrator.
///
/// Coordinates lock → prove → verify → mint across chains.
pub struct CrossChainTransfer {
    /// The cross-chain seal registry
    pub registry: CrossChainRegistry,
}

impl CrossChainTransfer {
    /// Create a new cross-chain transfer orchestrator.
    pub fn new(registry: CrossChainRegistry) -> Self {
        Self { registry }
    }

    /// Execute a full cross-chain transfer.
    ///
    /// 1. Lock the Right on the source chain
    /// 2. Build the transfer proof
    /// 3. Verify on the destination chain
    /// 4. Mint the new Right
    /// 5. Record in the registry
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &mut self,
        locker: &dyn LockProvider,
        verifier: &dyn TransferVerifier,
        minter: &dyn MintProvider,
        right_id: Hash,
        commitment: Hash,
        owner: OwnershipProof,
        destination_chain: ChainId,
        destination_owner: OwnershipProof,
        current_block_height: u64,
        finality_depth: u64,
    ) -> Result<CrossChainTransferResult, CrossChainError> {
        // Step 1: Lock on source chain
        let (lock_event, inclusion_proof) = locker.lock_right(
            right_id,
            commitment,
            owner.clone(),
            destination_chain,
            destination_owner.clone(),
        )?;

        // Step 2: Build transfer proof
        let source_chain = lock_event.source_chain;
        let source_block_height = lock_event.source_block_height;
        let lock_timestamp = lock_event.timestamp;

        let is_finalized = current_block_height >= source_block_height + finality_depth;

        let transfer_proof = CrossChainTransferProof {
            lock_event,
            inclusion_proof,
            finality_proof: CrossChainFinalityProof {
                source_chain,
                height: source_block_height,
                current_height: current_block_height,
                is_finalized,
                depth: finality_depth,
            },
            source_state_root: Hash::new([0u8; 32]),
        };

        // Step 3: Verify on destination
        verifier.verify_transfer_proof(&transfer_proof)?;

        // Step 4: Mint on destination
        let result = minter.mint_right(&transfer_proof)?;

        // Step 5: Record in registry
        let entry = CrossChainRegistryEntry {
            right_id,
            source_chain,
            source_seal: transfer_proof.lock_event.source_seal.clone(),
            destination_chain: transfer_proof.lock_event.destination_chain,
            destination_seal: result.destination_seal.clone(),
            lock_tx_hash: transfer_proof.lock_event.source_tx_hash,
            mint_tx_hash: Hash::new([0u8; 32]),
            timestamp: lock_timestamp,
        };
        self.registry.record_transfer(entry)?;

        Ok(result)
    }
}

/// Cross-chain seal registry.
///
/// Tracks all cross-chain transfers to prevent double-spends.
#[derive(Default)]
pub struct CrossChainRegistry {
    entries: alloc::collections::BTreeMap<Hash, CrossChainRegistryEntry>,
}

impl CrossChainRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            entries: alloc::collections::BTreeMap::new(),
        }
    }

    /// Record a cross-chain transfer.
    pub fn record_transfer(
        &mut self,
        entry: CrossChainRegistryEntry,
    ) -> Result<(), CrossChainError> {
        // Check if this Right has already been transferred
        if self.entries.contains_key(&entry.right_id) {
            return Err(CrossChainError::AlreadyMinted);
        }

        // Check if the source seal has already been consumed
        for existing in self.entries.values() {
            if existing.source_seal == entry.source_seal {
                return Err(CrossChainError::AlreadyLocked);
            }
        }

        self.entries.insert(entry.right_id, entry);
        Ok(())
    }

    /// Check if a Right has already been transferred.
    pub fn is_right_transferred(&self, right_id: &Hash) -> bool {
        self.entries.contains_key(right_id)
    }

    /// Check if a source seal has already been consumed.
    pub fn is_seal_consumed(&self, seal: &SealRef) -> bool {
        self.entries.values().any(|e| &e.source_seal == seal)
    }

    /// Get the registry entry for a Right.
    pub fn get_entry(&self, right_id: &Hash) -> Option<&CrossChainRegistryEntry> {
        self.entries.get(right_id)
    }

    /// Get the number of recorded transfers.
    pub fn transfer_count(&self) -> usize {
        self.entries.len()
    }

    /// Get all recorded transfers.
    pub fn all_transfers(&self) -> Vec<&CrossChainRegistryEntry> {
        self.entries.values().collect()
    }
}

// Re-export for convenience
pub use crate::seal_registry::SealConsumption;

/// Cross-chain seal registry for tracking transfers across all chains
pub use crate::seal_registry::CrossChainSealRegistry;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::Hash;

    #[test]
    fn test_chain_id_roundtrip() {
        for chain in [
            ChainId::Bitcoin,
            ChainId::Sui,
            ChainId::Aptos,
            ChainId::Ethereum,
            ChainId::Solana,
        ] {
            let id_str = chain.id();
            let parsed: Result<ChainId, _> = id_str.parse();
            assert_eq!(parsed, Ok(chain));
        }
        assert!("unknown".parse::<ChainId>().is_err());
    }

    #[test]
    fn test_registry_prevents_double_mint() {
        let mut registry = CrossChainRegistry::new();
        let right_id = Hash::new([0xAB; 32]);

        let entry = CrossChainRegistryEntry {
            right_id,
            source_chain: ChainId::Bitcoin,
            source_seal: SealRef::new(vec![0x01], None).unwrap(),
            destination_chain: ChainId::Sui,
            destination_seal: SealRef::new(vec![0x02], None).unwrap(),
            lock_tx_hash: Hash::new([0x03; 32]),
            mint_tx_hash: Hash::new([0x04; 32]),
            timestamp: 1_000_000,
        };

        registry.record_transfer(entry.clone()).unwrap();

        // Second transfer of same Right should fail
        let result = registry.record_transfer(entry);
        assert!(matches!(result, Err(CrossChainError::AlreadyMinted)));
    }

    #[test]
    fn test_registry_prevents_double_lock() {
        let mut registry = CrossChainRegistry::new();
        let seal = SealRef::new(vec![0x01], None).unwrap();

        let entry1 = CrossChainRegistryEntry {
            right_id: Hash::new([0xAB; 32]),
            source_chain: ChainId::Bitcoin,
            source_seal: seal.clone(),
            destination_chain: ChainId::Sui,
            destination_seal: SealRef::new(vec![0x02], None).unwrap(),
            lock_tx_hash: Hash::new([0x03; 32]),
            mint_tx_hash: Hash::new([0x04; 32]),
            timestamp: 1_000_000,
        };

        registry.record_transfer(entry1).unwrap();

        // Second transfer using same source seal should fail
        let entry2 = CrossChainRegistryEntry {
            right_id: Hash::new([0xCD; 32]),
            source_chain: ChainId::Bitcoin,
            source_seal: seal.clone(),
            destination_chain: ChainId::Aptos,
            destination_seal: SealRef::new(vec![0x05], None).unwrap(),
            lock_tx_hash: Hash::new([0x06; 32]),
            mint_tx_hash: Hash::new([0x07; 32]),
            timestamp: 2_000_000,
        };

        let result = registry.record_transfer(entry2);
        assert!(matches!(result, Err(CrossChainError::AlreadyLocked)));
    }

    #[test]
    fn test_registry_tracks_transfers() {
        let mut registry = CrossChainRegistry::new();
        assert_eq!(registry.transfer_count(), 0);

        let entry = CrossChainRegistryEntry {
            right_id: Hash::new([0xAB; 32]),
            source_chain: ChainId::Bitcoin,
            source_seal: SealRef::new(vec![0x01], None).unwrap(),
            destination_chain: ChainId::Sui,
            destination_seal: SealRef::new(vec![0x02], None).unwrap(),
            lock_tx_hash: Hash::new([0x03; 32]),
            mint_tx_hash: Hash::new([0x04; 32]),
            timestamp: 1_000_000,
        };

        registry.record_transfer(entry).unwrap();
        assert_eq!(registry.transfer_count(), 1);
        assert!(registry.is_right_transferred(&Hash::new([0xAB; 32])));
    }
}
