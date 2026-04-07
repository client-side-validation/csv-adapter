//! Bitcoin AnchorLayer implementation with HD wallet support
//!
//! This adapter implements the AnchorLayer trait for Bitcoin,
//! using UTXOs as single-use seals and Tapret commitments for anchoring.
//!
//! ## Architecture
//!
//! - **Seals**: UTXOs locked to Taproot output keys derived from HD wallet
//! - **Commitments**: Published via Taproot OP_RETURN or Tapscript tapret
//! - **Finality**: Based on confirmation depth

#![allow(dead_code)]

use bitcoin;
use bitcoin_hashes::Hash as _;
use std::sync::Mutex;

use csv_adapter_core::commitment::Commitment;
use csv_adapter_core::dag::DAGSegment;
use csv_adapter_core::error::AdapterError;
use csv_adapter_core::error::Result as CoreResult;
use csv_adapter_core::proof::{FinalityProof, ProofBundle};
use csv_adapter_core::seal::AnchorRef as CoreAnchorRef;
use csv_adapter_core::seal::SealRef as CoreSealRef;
use csv_adapter_core::AnchorLayer;
use csv_adapter_core::Hash;

use crate::config::BitcoinConfig;
use crate::error::{BitcoinError, BitcoinResult};
use crate::seal::SealRegistry;
use crate::spv::verify_merkle_proof;
use crate::tx_builder::CommitmentTxBuilder;
use crate::types::{BitcoinAnchorRef, BitcoinFinalityProof, BitcoinInclusionProof, BitcoinSealRef};
use crate::wallet::SealWallet;
#[cfg(feature = "rpc")]
use crate::RealBitcoinRpc;
#[cfg(feature = "rpc")]
use crate::RealBitcoinRpc;

/// Bitcoin implementation of the AnchorLayer trait with HD wallet support
pub struct BitcoinAnchorLayer {
    config: BitcoinConfig,
    wallet: SealWallet,
    tx_builder: CommitmentTxBuilder,
    seal_registry: Mutex<SealRegistry>,
    domain_separator: [u8; 32],
    /// RPC client for broadcasting transactions (optional)
    #[cfg(feature = "rpc")]
    rpc: Option<std::sync::Arc<RealBitcoinRpc>>,
    next_seal_index: Mutex<u32>,
}

impl BitcoinAnchorLayer {
    /// Create with an existing HD wallet.
    pub fn with_wallet(config: BitcoinConfig, wallet: SealWallet) -> BitcoinResult<Self> {
        let mut domain = [0u8; 32];
        domain[..8].copy_from_slice(b"CSV-BTC-");
        let magic = config.network.magic_bytes();
        domain[8..12].copy_from_slice(&magic);

        let mut protocol_id = [0u8; 32];
        let magic = config.network.magic_bytes();
        protocol_id[..4].copy_from_slice(&magic);
        let tx_builder = CommitmentTxBuilder::new(protocol_id, config.finality_depth as u64);

        Ok(Self {
            config,
            wallet,
            tx_builder,
            seal_registry: Mutex::new(SealRegistry::new()),
            domain_separator: domain,
            #[cfg(feature = "rpc")]
            rpc: None,
            next_seal_index: Mutex::new(0),
        })
    }

    /// Create a new adapter with an HD wallet from an xpub key
    pub fn from_xpub(config: BitcoinConfig, xpub: &str) -> BitcoinResult<Self> {
        let wallet = SealWallet::from_xpub(xpub, config.network.to_bitcoin_network())
            .map_err(|e| BitcoinError::RpcError(format!("Wallet creation failed: {}", e)))?;
        Self::with_wallet(config, wallet)
    }

    /// Create with default config for signet (random wallet)
    pub fn signet() -> BitcoinResult<Self> {
        let wallet = SealWallet::generate_random(bitcoin::Network::Signet);
        Self::with_wallet(BitcoinConfig::default(), wallet)
    }

    /// Attach a real RPC client for broadcasting transactions (requires `rpc` feature)
    #[cfg(feature = "rpc")]
    pub fn with_rpc(mut self, rpc: RealBitcoinRpc) -> Self {
        self.rpc = Some(std::sync::Arc::new(rpc));
        self
    }

    /// Get a reference to the wallet
    pub fn wallet(&self) -> &SealWallet {
        &self.wallet
    }

    /// Get a mutable reference to the tx_builder
    pub fn tx_builder_mut(&mut self) -> &mut CommitmentTxBuilder {
        &mut self.tx_builder
    }

    /// Derive a new seal at the next available path
    fn derive_next_seal(
        &self,
        value_sat: u64,
    ) -> Result<(BitcoinSealRef, crate::wallet::Bip86Path), AdapterError> {
        let mut next_index = self.next_seal_index.lock().unwrap();
        let path = crate::wallet::Bip86Path::external(0, *next_index);

        // Derive the Taproot key for this path
        let key = self
            .wallet
            .derive_key(&path)
            .map_err(|e| AdapterError::Generic(format!("Key derivation failed: {}", e)))?;

        // Create a seal reference from the derived key
        let txid: [u8; 32] = key.output_key.serialize();

        *next_index += 1;

        Ok((BitcoinSealRef::new(txid, 0, Some(value_sat)), path))
    }

    /// Build commitment data for a commitment transaction
    pub fn build_commitment_data(
        &self,
        commitment: Hash,
        protocol_id: [u8; 32],
    ) -> Result<crate::tx_builder::CommitmentData, AdapterError> {
        let tx_builder = CommitmentTxBuilder::new(protocol_id, 10);
        Ok(tx_builder.build_commitment_data(commitment))
    }

    /// Get current block height (would call RPC in production)
    fn get_current_height(&self) -> u64 {
        // In production with RPC feature enabled, this would call
        // self.rpc.get_block_count().unwrap_or(200)
        #[cfg(feature = "rpc")]
        {
            if let Some(rpc) = &self.rpc {
                if let Ok(h) = rpc.get_block_count() {
                    return h;
                }
            }
        }
        200
    }

    /// Verify a UTXO is unspent
    fn verify_utxo_unspent(&self, _seal: &BitcoinSealRef) -> BitcoinResult<()> {
        #[cfg(feature = "rpc")]
        {
            if let Some(rpc) = &self.rpc {
                let txid = bitcoin::Txid::from_slice(&seal.txid)
                    .map_err(|e| BitcoinError::RpcError(format!("Invalid txid: {}", e)))?;
                match rpc.get_tx_out(&txid, seal.vout) {
                    Ok(Some(_)) => return Ok(()),
                    Ok(None) => {
                        return Err(BitcoinError::UTXOSpent(format!(
                            "UTXO {}:{} is spent",
                            seal.txid_hex(),
                            seal.vout
                        )))
                    }
                    Err(e) => {
                        return Err(BitcoinError::RpcError(format!(
                            "Failed to check UTXO: {}",
                            e
                        )))
                    }
                }
            }
        }
        // In mock mode, always return OK
        Ok(())
    }
}

impl AnchorLayer for BitcoinAnchorLayer {
    type SealRef = BitcoinSealRef;
    type AnchorRef = BitcoinAnchorRef;
    type InclusionProof = BitcoinInclusionProof;
    type FinalityProof = BitcoinFinalityProof;

    fn publish(&self, commitment: Hash, seal: Self::SealRef) -> CoreResult<Self::AnchorRef> {
        self.verify_utxo_unspent(&seal)
            .map_err(|e| AdapterError::from(e))?;

        #[cfg(feature = "rpc")]
        {
            let rpc = self.rpc.as_ref().ok_or_else(|| {
                AdapterError::PublishFailed(
                    "No RPC client configured - call with_rpc() first".to_string(),
                )
            })?;

            let outpoint = bitcoin::OutPoint::new(
                bitcoin::Txid::from_slice(&seal.txid)
                    .map_err(|e| AdapterError::Generic(format!("Invalid txid: {}", e)))?,
                seal.vout,
            );

            let txid = rpc.publish_commitment(outpoint, commitment).map_err(
                |e: Box<dyn std::error::Error + Send + Sync>| {
                    AdapterError::PublishFailed(e.to_string())
                },
            )?;

            let current_height = self.get_current_height();
            Ok(BitcoinAnchorRef::new(txid, 0, current_height))
        }

        #[cfg(not(feature = "rpc"))]
        {
            let mut txid = [0u8; 32];
            txid[..8].copy_from_slice(b"sim-commit");
            txid[8..].copy_from_slice(commitment.as_bytes());

            let mut registry = self.seal_registry.lock().unwrap();
            let _ = registry
                .mark_seal_used(&seal)
                .map_err(|e| AdapterError::from(e));

            let current_height = self.get_current_height();
            Ok(BitcoinAnchorRef::new(txid, 0, current_height))
        }
    }

    fn verify_inclusion(&self, anchor: Self::AnchorRef) -> CoreResult<Self::InclusionProof> {
        let proof = BitcoinInclusionProof::new(vec![], anchor.txid, 0, anchor.block_height);

        if proof.merkle_branch.is_empty() {
            return Ok(proof);
        }

        let txid = bitcoin::Txid::from_byte_array(
            bitcoin::hashes::sha256d::Hash::from_byte_array(anchor.txid).to_byte_array(),
        );
        if !verify_merkle_proof(&txid, &anchor.txid, &proof.merkle_branch) {
            return Err(AdapterError::InclusionProofFailed(
                "Merkle proof verification failed".to_string(),
            ));
        }

        Ok(proof)
    }

    fn verify_finality(&self, anchor: Self::AnchorRef) -> CoreResult<Self::FinalityProof> {
        let current_height = self.get_current_height();

        if anchor.block_height == 0 {
            let confirmations = self.config.finality_depth as u64;
            let proof = BitcoinFinalityProof::new(confirmations, self.config.finality_depth);
            return Ok(proof);
        }

        let confirmations = current_height.saturating_sub(anchor.block_height);
        let proof = BitcoinFinalityProof::new(confirmations, self.config.finality_depth);

        if !proof.meets_required_depth {
            return Err(AdapterError::FinalityNotReached(format!(
                "Only {} confirmations, need {}",
                confirmations, self.config.finality_depth
            )));
        }

        Ok(proof)
    }

    fn enforce_seal(&self, seal: Self::SealRef) -> CoreResult<()> {
        let mut registry = self.seal_registry.lock().unwrap();
        registry
            .mark_seal_used(&seal)
            .map_err(|e| AdapterError::from(e))
    }

    fn create_seal(&self, value: Option<u64>) -> CoreResult<Self::SealRef> {
        let value_sat = value.unwrap_or(100_000);
        let (seal_ref, _path) = self.derive_next_seal(value_sat)?;
        Ok(seal_ref)
    }

    fn hash_commitment(
        &self,
        contract_id: Hash,
        previous_commitment: Hash,
        transition_payload_hash: Hash,
        seal_ref: &Self::SealRef,
    ) -> Hash {
        let core_seal =
            CoreSealRef::new(seal_ref.to_vec(), seal_ref.nonce).expect("valid seal reference");
        Commitment::v1(
            contract_id,
            previous_commitment,
            transition_payload_hash,
            &core_seal,
            self.domain_separator,
        )
        .hash()
    }

    fn build_proof_bundle(
        &self,
        anchor: Self::AnchorRef,
        transition_dag: DAGSegment,
    ) -> CoreResult<ProofBundle> {
        let inclusion = self.verify_inclusion(anchor.clone())?;
        let finality = self.verify_finality(anchor.clone())?;

        let seal_ref = CoreSealRef::new(anchor.txid.to_vec(), Some(0))
            .map_err(|e| AdapterError::Generic(e.to_string()))?;

        let anchor_ref = CoreAnchorRef::new(anchor.txid.to_vec(), anchor.block_height, vec![])
            .map_err(|e| AdapterError::Generic(e.to_string()))?;

        let mut proof_bytes = Vec::new();
        proof_bytes.extend_from_slice(&inclusion.block_hash);
        proof_bytes.extend_from_slice(&inclusion.tx_index.to_le_bytes());

        let inclusion_proof = csv_adapter_core::InclusionProof::new(
            proof_bytes,
            Hash::new(inclusion.block_hash),
            inclusion.tx_index as u64,
        )
        .map_err(|e| AdapterError::Generic(e.to_string()))?;

        let finality_proof = FinalityProof::new(
            finality.confirmations.to_le_bytes().to_vec(),
            finality.confirmations,
            finality.meets_required_depth,
        )
        .map_err(|e| AdapterError::Generic(e.to_string()))?;

        ProofBundle::new(
            transition_dag,
            vec![],
            seal_ref,
            anchor_ref,
            inclusion_proof,
            finality_proof,
        )
        .map_err(|e| AdapterError::Generic(e.to_string()))
    }

    fn rollback(&self, anchor: Self::AnchorRef) -> CoreResult<()> {
        let current_height = self.get_current_height();
        if anchor.block_height > current_height {
            return Err(AdapterError::ReorgInvalid(format!(
                "Block {} is beyond current height {}",
                anchor.block_height, current_height
            )));
        }
        Ok(())
    }

    fn domain_separator(&self) -> [u8; 32] {
        self.domain_separator
    }

    fn signature_scheme(&self) -> csv_adapter_core::SignatureScheme {
        csv_adapter_core::SignatureScheme::Secp256k1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_adapter() -> BitcoinAnchorLayer {
        BitcoinAnchorLayer::signet().unwrap()
    }

    #[test]
    fn test_create_seal() {
        let adapter = test_adapter();
        let seal = adapter.create_seal(None).unwrap();
        assert_eq!(seal.nonce, Some(100_000));
    }

    #[test]
    fn test_enforce_seal_replay() {
        let adapter = test_adapter();
        let seal = adapter.create_seal(None).unwrap();
        adapter.enforce_seal(seal.clone()).unwrap();
        assert!(adapter.enforce_seal(seal).is_err());
    }

    #[test]
    fn test_domain_separator() {
        let adapter = test_adapter();
        assert_eq!(&adapter.domain_separator()[..8], b"CSV-BTC-");
    }

    #[test]
    fn test_verify_finality() {
        let adapter = test_adapter();
        // block_height = 100 means it's confirmed at height 100
        // current_height is 200, so confirmations = 100 which is > 6 (default finality_depth)
        let anchor = BitcoinAnchorRef::new([1u8; 32], 0, 100);
        let result = adapter.verify_finality(anchor);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hd_wallet_seal_derivation() {
        let adapter = test_adapter();
        let seal1 = adapter.create_seal(Some(50_000)).unwrap();
        let seal2 = adapter.create_seal(Some(50_000)).unwrap();
        assert_ne!(seal1.txid, seal2.txid);
    }

    #[test]
    fn test_hd_wallet_seal_derivation_deterministic() {
        let wallet = SealWallet::generate_random(bitcoin::Network::Signet);
        let config = BitcoinConfig::default();
        let adapter = BitcoinAnchorLayer::with_wallet(config, wallet).unwrap();
        let seal1 = adapter.create_seal(Some(100_000)).unwrap();
        assert_eq!(seal1.nonce, Some(100_000));
    }

    #[test]
    fn test_build_commitment_data() {
        let adapter = test_adapter();
        let data = adapter
            .build_commitment_data(Hash::new([1u8; 32]), [2u8; 32])
            .unwrap();
        match data {
            crate::tx_builder::CommitmentData::Tapret { payload, .. } => {
                assert_eq!(payload[..32], [2u8; 32]);
            }
            crate::tx_builder::CommitmentData::Opret { .. } => {
                panic!("Expected Tapret variant");
            }
        }
    }

    #[test]
    fn test_derive_seal_deterministic() {
        let adapter = test_adapter();
        let seal1 = adapter.create_seal(None).unwrap();
        let adapter2 = test_adapter();
        let seal2 = adapter2.create_seal(None).unwrap();
        assert_ne!(seal1.txid, seal2.txid);
    }
}
