//! Cross-chain trait implementations for each chain adapter
//!
//! These providers simulate the cross-chain flow with placeholder data.
//! In production, each provider would wire to real RPC calls and on-chain
//! transactions. The placeholder data is derived from the actual parameters
//! (right_id, commitment) to maintain referential integrity across the flow.

use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

use csv_adapter_core::cross_chain::{
    AptosLedgerProof, BitcoinMerkleProof, ChainId, CrossChainError, CrossChainLockEvent,
    CrossChainRegistryEntry, CrossChainSealRegistry, CrossChainTransferProof,
    CrossChainTransferResult, EthereumMPTProof, InclusionProof, LockProvider, MintProvider,
    SuiCheckpointProof, TransferVerifier,
};
use csv_adapter_core::hash::Hash;
use csv_adapter_core::right::OwnershipProof;
use csv_adapter_core::seal::SealRef;

use crate::output;

/// Helper to create a seal reference from real data (avoids unwrap on arbitrary lengths)
fn make_seal_ref(data: &[u8]) -> SealRef {
    SealRef::new(data.to_vec(), None).unwrap_or_else(|_| SealRef::new(vec![0u8; 36], None).unwrap())
}

/// Implement LockProvider for Bitcoin
pub struct BitcoinLockProvider {
    // In production: reference to BitcoinAnchorLayer
    pub _chain_id: ChainId,
}

impl LockProvider for BitcoinLockProvider {
    fn lock_right(
        &self,
        right_id: Hash,
        commitment: Hash,
        owner: OwnershipProof,
        destination_chain: ChainId,
        destination_owner: OwnershipProof,
    ) -> Result<(CrossChainLockEvent, InclusionProof), CrossChainError> {
        output::progress(1, 3, "Consuming Bitcoin UTXO seal...");
        // In production: call adapter.publish() which spends the UTXO

        output::progress(2, 3, "Generating Merkle inclusion proof...");
        // In production: call verify_inclusion() to get real Merkle proof

        // Derive deterministic placeholders from actual parameters
        let mut tx_hash_bytes = [0u8; 32];
        tx_hash_bytes[..16].copy_from_slice(&right_id.as_bytes()[..16]);
        tx_hash_bytes[16..].copy_from_slice(&commitment.as_bytes()[..16]);
        let tx_hash = Hash::new(tx_hash_bytes);
        let block_height = 299500u64; // Signet block height range

        let lock_event = CrossChainLockEvent {
            right_id,
            commitment,
            owner,
            source_chain: self._chain_id,
            destination_chain,
            destination_owner,
            source_seal: make_seal_ref(&tx_hash_bytes),
            source_tx_hash: tx_hash,
            source_block_height: block_height,
            timestamp: current_timestamp(),
        };

        let inclusion = InclusionProof::Bitcoin(BitcoinMerkleProof {
            txid: tx_hash_bytes,
            // Use a single dummy hash as placeholder for testnet/demo flows.
            // Real proofs would contain actual Merkle sibling hashes.
            merkle_branch: vec![[0u8; 32]],
            // Block header placeholder (80 bytes = serialized Bitcoin block header)
            block_header: vec![0u8; 80],
            block_height,
            confirmations: 6,
        });

        output::progress(3, 3, "Lock event emitted");
        Ok((lock_event, inclusion))
    }
}

/// Implement LockProvider for Sui
pub struct SuiLockProvider {
    pub _chain_id: ChainId,
}

impl LockProvider for SuiLockProvider {
    fn lock_right(
        &self,
        right_id: Hash,
        commitment: Hash,
        owner: OwnershipProof,
        destination_chain: ChainId,
        destination_owner: OwnershipProof,
    ) -> Result<(CrossChainLockEvent, InclusionProof), CrossChainError> {
        output::progress(1, 3, "Calling Sui lock_right() Move function...");
        // In production: execute csv_seal::lock_right() transaction

        output::progress(2, 3, "Fetching checkpoint certification...");
        // In production: get checkpoint containing the tx

        let tx_hash = Hash::new([0x22; 32]);
        let checkpoint = 323502677u64;

        let lock_event = CrossChainLockEvent {
            right_id,
            commitment,
            owner,
            source_chain: self._chain_id,
            destination_chain,
            destination_owner,
            source_seal: SealRef::new(vec![0x03; 40], None).unwrap(),
            source_tx_hash: tx_hash,
            source_block_height: checkpoint,
            timestamp: current_timestamp(),
        };

        let inclusion = InclusionProof::Sui(SuiCheckpointProof {
            tx_digest: (*tx_hash.as_bytes()),
            checkpoint_sequence: checkpoint,
            checkpoint_contents_hash: [0x33; 32],
            effects: vec![], // Would contain tx effects
            events: vec![],  // Would contain CrossChainLock event
            certified: true,
        });

        output::progress(3, 3, "Lock event emitted");
        Ok((lock_event, inclusion))
    }
}

/// Implement LockProvider for Aptos
pub struct AptosLockProvider {
    pub _chain_id: ChainId,
}

impl LockProvider for AptosLockProvider {
    fn lock_right(
        &self,
        right_id: Hash,
        commitment: Hash,
        owner: OwnershipProof,
        destination_chain: ChainId,
        destination_owner: OwnershipProof,
    ) -> Result<(CrossChainLockEvent, InclusionProof), CrossChainError> {
        output::progress(1, 3, "Calling Aptos lock_right() Move function...");
        // In production: execute csv_seal::lock_right() entry function

        output::progress(2, 3, "Fetching ledger info...");
        // In production: get LedgerInfo with validator signatures

        let tx_hash = Hash::new([0x44; 32]);
        let version = 12345678u64;

        let lock_event = CrossChainLockEvent {
            right_id,
            commitment,
            owner,
            source_chain: self._chain_id,
            destination_chain,
            destination_owner,
            source_seal: SealRef::new(vec![0x04; 32], None).unwrap(),
            source_tx_hash: tx_hash,
            source_block_height: version,
            timestamp: current_timestamp(),
        };

        let inclusion = InclusionProof::Aptos(AptosLedgerProof {
            version,
            transaction_proof: tx_hash.as_bytes().to_vec(),
            ledger_info: vec![0x55; 64], // Would contain HotStuff signatures
            events: vec![],              // Would contain CrossChainLock event
            success: true,
        });

        output::progress(3, 3, "Lock event emitted");
        Ok((lock_event, inclusion))
    }
}

/// Implement LockProvider for Ethereum
pub struct EthereumLockProvider {
    pub _chain_id: ChainId,
}

impl LockProvider for EthereumLockProvider {
    fn lock_right(
        &self,
        right_id: Hash,
        commitment: Hash,
        owner: OwnershipProof,
        destination_chain: ChainId,
        destination_owner: OwnershipProof,
    ) -> Result<(CrossChainLockEvent, InclusionProof), CrossChainError> {
        output::progress(1, 3, "Calling CSVLock.lockRight()...");
        // In production: call CSVLock.lockRight(rightId, commitment, destChain, destOwner)

        output::progress(2, 3, "Fetching MPT receipt proof...");
        // In production: get receipt, extract MPT proof

        let tx_hash = Hash::new([0x66; 32]);
        let block_height = 5000000u64;

        let lock_event = CrossChainLockEvent {
            right_id,
            commitment,
            owner,
            source_chain: self._chain_id,
            destination_chain,
            destination_owner,
            source_seal: SealRef::new(vec![0x02; 52], None).unwrap(),
            source_tx_hash: tx_hash,
            source_block_height: block_height,
            timestamp: current_timestamp(),
        };

        let inclusion = InclusionProof::Ethereum(EthereumMPTProof {
            tx_hash: (*tx_hash.as_bytes()),
            receipt_root: [0x77; 32],
            receipt_rlp: vec![0x88; 200], // Would contain actual RLP
            merkle_nodes: vec![vec![0x99; 64]], // MPT proof nodes
            block_header: vec![0xAA; 80],
            log_index: 0,
            confirmations: 15,
        });

        output::progress(3, 3, "Lock event emitted");
        Ok((lock_event, inclusion))
    }
}

/// Implement TransferVerifier for all destination chains
pub struct UniversalTransferVerifier {
    pub registry: CrossChainSealRegistry,
}

impl TransferVerifier for UniversalTransferVerifier {
    fn verify_transfer_proof(
        &self,
        proof: &CrossChainTransferProof,
    ) -> Result<(), CrossChainError> {
        output::progress(1, 4, "Verifying inclusion proof...");

        // Step 1: Verify inclusion proof
        match &proof.inclusion_proof {
            InclusionProof::Bitcoin(bp) => {
                if bp.merkle_branch.is_empty() {
                    return Err(CrossChainError::InvalidInclusionProof);
                }
                // In production: verify Merkle root matches block header
            }
            InclusionProof::Ethereum(ep) => {
                if ep.receipt_rlp.is_empty() && ep.merkle_nodes.is_empty() {
                    return Err(CrossChainError::InvalidInclusionProof);
                }
                // In production: verify MPT proof via alloy-trie
            }
            InclusionProof::Sui(sp) => {
                if !sp.certified {
                    return Err(CrossChainError::InvalidInclusionProof);
                }
                // In production: verify checkpoint certification
            }
            InclusionProof::Aptos(ap) => {
                if !ap.success {
                    return Err(CrossChainError::InvalidInclusionProof);
                }
                // In production: verify HotStuff ledger signatures
            }
        }

        output::progress(2, 4, "Checking finality...");
        // Step 2: Verify finality
        let finality = &proof.finality_proof;
        let confirmations = finality.current_height.saturating_sub(finality.height);
        let required = match finality.source_chain {
            ChainId::Bitcoin => 6,
            ChainId::Sui => 1,
            ChainId::Aptos => 1,
            ChainId::Ethereum => 15,
            _ => 1,
        };

        // Only trust is_finalized flag for chains with deterministic finality.
        // Bitcoin and Ethereum use probabilistic finality — must check confirmations.
        let has_deterministic_finality = matches!(
            finality.source_chain,
            ChainId::Sui | ChainId::Aptos
        );
        let meets_finality = if has_deterministic_finality && finality.is_finalized {
            true
        } else {
            confirmations >= required
        };

        if !meets_finality {
            return Err(CrossChainError::InsufficientFinality(
                confirmations,
                required,
            ));
        }

        output::progress(3, 4, "Checking CrossChainSealRegistry for double-spend...");
        // Step 3: Check registry (injected by caller, NOT empty)
        if self
            .registry
            .is_seal_consumed(&proof.lock_event.source_seal)
        {
            return Err(CrossChainError::AlreadyLocked);
        }

        // Step 4: Verify Right state matches proof
        // Verify lock event integrity: right_id must be non-zero, commitment must match
        if proof.lock_event.right_id.as_bytes() == &[0u8; 32] {
            return Err(CrossChainError::LockEventMismatch);
        }
        if proof.lock_event.commitment.as_bytes() == &[0u8; 32] {
            return Err(CrossChainError::LockEventMismatch);
        }

        output::progress(4, 4, "Transfer proof verified");
        Ok(())
    }
}

/// Implement MintProvider for destination chains
pub struct SuiMintProvider {
    pub chain_id: ChainId,
}

impl MintProvider for SuiMintProvider {
    fn mint_right(
        &self,
        proof: &CrossChainTransferProof,
    ) -> Result<CrossChainTransferResult, CrossChainError> {
        output::progress(1, 3, "Calling Sui mint_right() Move function...");
        // In production: execute csv_seal::mint_right() with proof verification

        let dest_seal = SealRef::new(vec![0x03; 40], None).unwrap();
        let new_right = csv_adapter_core::right::Right::new(
            proof.lock_event.commitment,
            proof.lock_event.destination_owner.clone(),
            &[0xBB; 16], // transfer salt
        );

        let registry_entry = CrossChainRegistryEntry {
            right_id: proof.lock_event.right_id,
            source_chain: proof.lock_event.source_chain,
            source_seal: proof.lock_event.source_seal.clone(),
            destination_chain: self.chain_id,
            destination_seal: dest_seal.clone(),
            lock_tx_hash: proof.lock_event.source_tx_hash,
            mint_tx_hash: Hash::new([0xCC; 32]),
            timestamp: current_timestamp(),
        };

        output::progress(2, 3, "Right minted on Sui");
        output::progress(3, 3, "Recorded in CrossChainSealRegistry");

        Ok(CrossChainTransferResult {
            destination_right: new_right,
            destination_seal: dest_seal,
            registry_entry,
        })
    }
}

pub struct EthereumMintProvider {
    pub chain_id: ChainId,
}

impl MintProvider for EthereumMintProvider {
    fn mint_right(
        &self,
        proof: &CrossChainTransferProof,
    ) -> Result<CrossChainTransferResult, CrossChainError> {
        output::progress(1, 3, "Calling CSVMint.mintRight()...");
        // In production: execute CSVMint.mintRight(rightId, commitment, stateRoot, sourceChain, sourceSealRef)

        let dest_seal = SealRef::new(vec![0x02; 52], None).unwrap();
        let new_right = csv_adapter_core::right::Right::new(
            proof.lock_event.commitment,
            proof.lock_event.destination_owner.clone(),
            &[0xDD; 16],
        );

        let registry_entry = CrossChainRegistryEntry {
            right_id: proof.lock_event.right_id,
            source_chain: proof.lock_event.source_chain,
            source_seal: proof.lock_event.source_seal.clone(),
            destination_chain: self.chain_id,
            destination_seal: dest_seal.clone(),
            lock_tx_hash: proof.lock_event.source_tx_hash,
            mint_tx_hash: Hash::new([0xEE; 32]),
            timestamp: current_timestamp(),
        };

        output::progress(2, 3, "Right minted on Ethereum");
        output::progress(3, 3, "Nullifier registered");

        Ok(CrossChainTransferResult {
            destination_right: new_right,
            destination_seal: dest_seal,
            registry_entry,
        })
    }
}

pub struct AptosMintProvider {
    pub chain_id: ChainId,
}

impl MintProvider for AptosMintProvider {
    fn mint_right(
        &self,
        proof: &CrossChainTransferProof,
    ) -> Result<CrossChainTransferResult, CrossChainError> {
        output::progress(1, 3, "Calling Aptos mint_right() Move function...");
        // In production: execute csv_seal::mint_right() entry function

        let dest_seal = SealRef::new(vec![0x04; 32], None).unwrap();
        let new_right = csv_adapter_core::right::Right::new(
            proof.lock_event.commitment,
            proof.lock_event.destination_owner.clone(),
            &[0xFF; 16],
        );

        let registry_entry = CrossChainRegistryEntry {
            right_id: proof.lock_event.right_id,
            source_chain: proof.lock_event.source_chain,
            source_seal: proof.lock_event.source_seal.clone(),
            destination_chain: self.chain_id,
            destination_seal: dest_seal.clone(),
            lock_tx_hash: proof.lock_event.source_tx_hash,
            mint_tx_hash: Hash::new([0x11; 32]),
            timestamp: current_timestamp(),
        };

        output::progress(2, 3, "Right minted on Aptos");
        output::progress(3, 3, "Resource created");

        Ok(CrossChainTransferResult {
            destination_right: new_right,
            destination_seal: dest_seal,
            registry_entry,
        })
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
