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
use crate::right::{Right, OwnershipProof};
use crate::seal::SealRef;

/// Chain identifier for cross-chain transfers.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChainId {
    Bitcoin,
    Sui,
    Aptos,
    Ethereum,
}

impl ChainId {
    /// Get a numeric identifier for serialization.
    pub fn as_u8(&self) -> u8 {
        match self {
            ChainId::Bitcoin => 0,
            ChainId::Sui => 1,
            ChainId::Aptos => 2,
            ChainId::Ethereum => 3,
        }
    }

    /// Parse a chain ID from a u8.
    pub fn from_u8(id: u8) -> Option<Self> {
        match id {
            0 => Some(ChainId::Bitcoin),
            1 => Some(ChainId::Sui),
            2 => Some(ChainId::Aptos),
            3 => Some(ChainId::Ethereum),
            _ => None,
        }
    }
}

impl core::fmt::Display for ChainId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ChainId::Bitcoin => write!(f, "Bitcoin"),
            ChainId::Sui => write!(f, "Sui"),
            ChainId::Aptos => write!(f, "Aptos"),
            ChainId::Ethereum => write!(f, "Ethereum"),
        }
    }
}

/// Event emitted when a Right is locked on the source chain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossChainLockEvent {
    /// The Right being locked
    pub right_id: Hash,
    /// The commitment hash of the Right
    pub commitment: Hash,
    /// The owner who initiated the lock
    pub owner: OwnershipProof,
    /// Destination chain
    pub destination_chain: ChainId,
    /// Destination owner (may differ from source owner)
    pub destination_owner: OwnershipProof,
    /// Source chain's seal reference (consumed)
    pub source_seal: SealRef,
    /// Source transaction hash
    pub source_tx_hash: Hash,
    /// Source block height
    pub source_block_height: u64,
    /// Timestamp of lock
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
}

/// Bitcoin Merkle proof of transaction inclusion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitcoinMerkleProof {
    pub txid: [u8; 32],
    pub merkle_branch: Vec<[u8; 32]>,
    pub block_header: Vec<u8>,
    pub block_height: u64,
    pub confirmations: u64,
}

/// Ethereum MPT proof of receipt inclusion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EthereumMPTProof {
    pub tx_hash: [u8; 32],
    pub receipt_root: [u8; 32],
    pub receipt_rlp: Vec<u8>,
    pub merkle_nodes: Vec<Vec<u8>>,
    pub block_header: Vec<u8>,
    pub log_index: u64,
    pub confirmations: u64,
}

/// Sui checkpoint proof of transaction effects.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiCheckpointProof {
    pub tx_digest: [u8; 32],
    pub checkpoint_sequence: u64,
    pub checkpoint_contents_hash: [u8; 32],
    pub effects: Vec<u8>,
    pub events: Vec<u8>,
    pub certified: bool,
}

/// Aptos ledger info proof of transaction execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AptosLedgerProof {
    pub version: u64,
    pub transaction_proof: Vec<u8>,
    pub ledger_info: Vec<u8>,
    pub events: Vec<u8>,
    pub success: bool,
}

/// Finality proof — confirms source transaction is finalized.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossChainFinalityProof {
    /// Source chain
    pub source_chain: ChainId,
    /// Block/checkpoint/ledger height
    pub height: u64,
    /// Current height on source chain
    pub current_height: u64,
    /// Whether finality is achieved
    pub is_finalized: bool,
    /// Finality depth
    pub depth: u64,
}

/// The complete proof bundle submitted to the destination chain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossChainTransferProof {
    /// The lock event data
    pub lock_event: CrossChainLockEvent,
    /// Inclusion proof (chain-specific format)
    pub inclusion_proof: InclusionProof,
    /// Finality proof
    pub finality_proof: CrossChainFinalityProof,
    /// Source chain's state root at lock block
    pub source_state_root: Hash,
}

/// Entry in the cross-chain seal registry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossChainRegistryEntry {
    /// The Right's unique ID (preserved across chains)
    pub right_id: Hash,
    /// Source chain and seal
    pub source_chain: ChainId,
    pub source_seal: SealRef,
    /// Destination chain and seal
    pub destination_chain: ChainId,
    pub destination_seal: SealRef,
    /// Lock transaction hash on source
    pub lock_tx_hash: Hash,
    /// Mint transaction hash on destination
    pub mint_tx_hash: Hash,
    /// Timestamp of transfer
    pub timestamp: u64,
}

/// Result of a cross-chain transfer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossChainTransferResult {
    /// The new Right on the destination chain
    pub destination_right: Right,
    /// The destination chain's seal reference
    pub destination_seal: SealRef,
    /// Registry entry recording the transfer
    pub registry_entry: CrossChainRegistryEntry,
}

/// Errors that can occur during cross-chain transfer.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
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
    fn verify_transfer_proof(
        &self,
        proof: &CrossChainTransferProof,
    ) -> Result<(), CrossChainError>;
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
        let transfer_proof = CrossChainTransferProof {
            lock_event,
            inclusion_proof,
            finality_proof: CrossChainFinalityProof {
                source_chain: ChainId::Bitcoin, // TODO: determine from context
                height: 0,                       // TODO: get from lock
                current_height: 0,               // TODO: get from chain
                is_finalized: false,             // TODO: verify
                depth: 0,
            },
            source_state_root: Hash::new([0u8; 32]), // TODO: get from chain
        };

        // Step 3: Verify on destination
        verifier.verify_transfer_proof(&transfer_proof)?;

        // Step 4: Mint on destination
        let result = minter.mint_right(&transfer_proof)?;

        // Step 5: Record in registry
        let entry = CrossChainRegistryEntry {
            right_id,
            source_chain: ChainId::Bitcoin, // TODO: determine from context
            source_seal: transfer_proof.lock_event.source_seal.clone(),
            destination_chain: transfer_proof.lock_event.destination_chain.clone(),
            destination_seal: result.destination_seal.clone(),
            lock_tx_hash: transfer_proof.lock_event.source_tx_hash,
            mint_tx_hash: Hash::new([0u8; 32]), // TODO: get from mint tx
            timestamp: 0,                       // TODO: current timestamp
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
        for chain in [ChainId::Bitcoin, ChainId::Sui, ChainId::Aptos, ChainId::Ethereum] {
            let id = chain.as_u8();
            assert_eq!(ChainId::from_u8(id), Some(chain));
        }
        assert_eq!(ChainId::from_u8(99), None);
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
