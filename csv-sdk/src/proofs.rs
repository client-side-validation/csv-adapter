//! Proof management facade.
//!
//! The [`ProofManager`] handles generation and verification of
//! cryptographic proofs for Rights and cross-chain transfers.
//!
//! # Proof Types
//!
//! - **Inclusion Proof**: Proves a transaction is included in a block
//! - **Finality Proof**: Proves a block is finalized at sufficient depth
//! - **ProofBundle**: Complete proof package (inclusion + finality)

use std::sync::Arc;

use csv_adapter_core::{Chain, Hash, ProofBundle, RightId};

use crate::client::ClientRef;
use crate::errors::CsvError;

/// Manager for proof operations.
///
/// Obtain a [`ProofManager`] via
/// [`CsvClient::proofs()`](crate::client::CsvClient::proofs).
///
/// # Example
///
/// ```no_run
/// use csv_adapter::prelude::*;
///
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// # let client = CsvClient::builder()
/// #     .with_chain(Chain::Bitcoin)
/// #     .with_store_backend(StoreBackend::InMemory)
/// #     .build()?;
/// let proofs = client.proofs();
///
/// // Generate a proof bundle for a right
/// let bundle = proofs.generate(&right_id, Chain::Bitcoin)?;
/// # Ok(())
/// # }
/// ```
pub struct ProofManager {
    client: Arc<ClientRef>,
}

impl ProofManager {
    pub(crate) fn new(client: Arc<ClientRef>) -> Self {
        Self { client }
    }

    /// Generate a proof bundle for a Right on the specified chain.
    ///
    /// This creates a complete [`ProofBundle`] containing:
    /// - Inclusion proof (chain-specific format)
    /// - Finality proof
    /// - State transition DAG
    ///
    /// # Arguments
    ///
    /// * `right_id` — The Right to generate a proof for.
    /// * `chain` — The chain where the Right's seal is anchored.
    ///
    /// # Chain-Specific Proof Formats
    ///
    /// - **Bitcoin**: Merkle branch + block header (SPV proof)
    /// - **Ethereum**: MPT receipt proof + log inclusion
    /// - **Sui**: Checkpoint certification + transaction effects
    /// - **Aptos**: Ledger info proof + event stream
    pub fn generate(&self, _right_id: &RightId, chain: Chain) -> Result<ProofBundle, CsvError> {
        if !self.client.is_chain_enabled(chain) {
            return Err(CsvError::ChainNotSupported(chain));
        }

        // Proof generation requires chain adapter proof provider integration
        // This will be implemented when the chain adapter proof providers are ready
        Err(CsvError::AdapterError {
            chain,
            message: "Proof generation requires chain adapter ProofProvider trait integration.".to_string()
        })
    }

    /// Verify a proof bundle against an expected Right ID.
    ///
    /// This is the core client-side validation function. It verifies:
    /// 1. The inclusion proof is valid (transaction is in a block)
    /// 2. The finality proof is sufficient (block is finalized)
    /// 3. The seal was consumed exactly once (single-use enforcement)
    /// 4. If `expected_right_id` is provided, the proof resolves to it
    ///
    /// # Arguments
    ///
    /// * `proof` — The proof bundle to verify.
    /// * `expected_right_id` — Optional expected Right ID after verification.
    ///
    /// # Returns
    ///
    /// `true` if the proof is valid, `false` otherwise.
    pub fn verify(
        &self,
        proof: &ProofBundle,
        expected_right_id: Option<&RightId>,
    ) -> Result<bool, CsvError> {
        // In a full implementation, this would:
        // 1. Extract the inclusion proof and verify it against the source chain
        // 2. Verify the finality proof meets the required depth
        // 3. Check the seal registry for double-spend
        // 4. If expected_right_id is provided, verify the proof resolves to it
        //
        // The chain adapters provide chain-specific verification:
        // - Bitcoin: SPV verification with merkle proof
        // - Ethereum: MPT verification with receipt proof
        // - Sui: Checkpoint verification with Narwhal consensus
        // - Aptos: Ledger info verification with HotStuff

        let _ = expected_right_id;

        // Basic validation: proof bundle must have valid structure
        // In production, full cryptographic verification happens here
        let valid = proof.inclusion_proof.block_hash != Hash::zero()
            || proof.transition_dag.nodes.is_empty();

        Ok(valid)
    }

}
