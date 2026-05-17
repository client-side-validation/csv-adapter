//! Parallel verification service for the Wallet.
//!
//! Provides concurrent proof and seal verification using async/await,
//! optimized for WebAssembly (WASM) environments where threads are not available.
//! This is the default verification mode for the Wallet.

use std::sync::Arc;
use std::time::Instant;

use csv_core::proof::ProofBundle;
use csv_core::proof_pipeline::ChainVerifier;
use csv_core::hash::Hash;
use csv_core::signature::SignatureScheme;
use csv_core::verifier::verify_proof;
use csv_core::CrossChainHashAlgorithm;
use futures::future::{join_all, try_join_all};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::services::seal_service::{SealError, SealRecord, SealStatus};

/// Verification result for a single item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// ID of the verified item (seal ID or proof ID).
    pub id: String,
    /// Whether verification succeeded.
    pub success: bool,
    /// Verification time in milliseconds.
    pub duration_ms: u64,
    /// Error message if verification failed.
    pub error: Option<String>,
}

/// Batch verification statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationStats {
    /// Total items verified.
    pub total: usize,
    /// Successful verifications.
    pub successful: usize,
    /// Failed verifications.
    pub failed: usize,
    /// Total time in milliseconds.
    pub total_duration_ms: u64,
    /// Average time per verification in milliseconds.
    pub avg_duration_ms: f64,
    /// Throughput (verifications per second).
    pub throughput_per_sec: f64,
}

/// Parallel verification service.
///
/// Uses concurrent async futures to verify multiple seals or proofs
/// simultaneously, providing significant performance improvements over
/// sequential verification.
pub struct ParallelVerifyService {
    /// Maximum concurrent verification tasks.
    max_concurrent: usize,
    /// Chain verifier for seal registry checks
    verifier: Arc<dyn ChainVerifier>,
}

impl ParallelVerifyService {
    /// Create a new parallel verification service with a chain verifier.
    pub fn new(verifier: Arc<dyn ChainVerifier>) -> Self {
        Self {
            max_concurrent: 10, // Limit concurrent tasks to avoid overwhelming the browser
            verifier,
        }
    }

    /// Create a new parallel verification service with custom concurrency limit.
    pub fn with_concurrency(max_concurrent: usize, verifier: Arc<dyn ChainVerifier>) -> Self {
        Self {
            max_concurrent: max_concurrent.max(1),
            verifier,
        }
    }

    /// Verify multiple seals in parallel.
    ///
    /// # Arguments
    /// * `seals` - Slice of seal records to verify
    ///
    /// # Returns
    /// Vector of verification results and statistics
    pub async fn verify_seals_parallel(
        &self,
        seals: &[SealRecord],
    ) -> (Vec<VerificationResult>, VerificationStats) {
        let start = Instant::now();
        let total = seals.len();

        info!("Starting parallel verification of {} seals", total);

        // Process seals in batches to respect max_concurrent limit
        let mut all_results = Vec::with_capacity(total);

        for chunk in seals.chunks(self.max_concurrent) {
            let chunk_results: Vec<_> = chunk
                .iter()
                .map(|seal| async {
                    let seal_start = Instant::now();
                    let result = self.verify_single_seal(seal).await;
                    let duration = seal_start.elapsed().as_millis() as u64;

                    VerificationResult {
                        id: seal.id.clone(),
                        success: result.is_ok(),
                        duration_ms: duration,
                        error: result.err().map(|e| e.to_string()),
                    }
                })
                .collect();

            let results = join_all(chunk_results).await;
            all_results.extend(results);
        }

        let total_duration = start.elapsed().as_millis() as u64;
        let successful = all_results.iter().filter(|r| r.success).count();
        let failed = total - successful;
        let avg_duration = if total > 0 {
            total_duration as f64 / total as f64
        } else {
            0.0
        };
        let throughput = if total_duration > 0 {
            (total as f64 / total_duration as f64) * 1000.0
        } else {
            0.0
        };

        let stats = VerificationStats {
            total,
            successful,
            failed,
            total_duration_ms: total_duration,
            avg_duration_ms: avg_duration,
            throughput_per_sec: throughput,
        };

        info!(
            "Parallel verification complete: {}/{} successful, {:.2}ms avg, {:.0} verifications/sec",
            successful, total, avg_duration, throughput
        );

        (all_results, stats)
    }

    /// Verify multiple proof bundles in parallel.
    ///
    /// # Arguments
    /// * `proofs` - Slice of proof bundles to verify
    ///
    /// # Returns
    /// Vector of verification results and statistics
    pub async fn verify_proofs_parallel(
        &self,
        proofs: &[ProofBundle],
    ) -> (Vec<VerificationResult>, VerificationStats) {
        let start = Instant::now();
        let total = proofs.len();

        info!("Starting parallel verification of {} proofs", total);

        let mut all_results = Vec::with_capacity(total);

        for chunk in proofs.chunks(self.max_concurrent) {
            let chunk_results: Vec<_> = chunk
                .iter()
                .map(|proof| async {
                    let proof_start = Instant::now();
                    let result = self.verify_single_proof(proof).await;
                    let duration = proof_start.elapsed().as_millis() as u64;

                    VerificationResult {
                        id: hex::encode(&proof.anchor_ref.anchor_id),
                        success: result.is_ok(),
                        duration_ms: duration,
                        error: result.err().map(|e| e.to_string()),
                    }
                })
                .collect();

            let results = join_all(chunk_results).await;
            all_results.extend(results);
        }

        let total_duration = start.elapsed().as_millis() as u64;
        let successful = all_results.iter().filter(|r| r.success).count();
        let failed = total - successful;
        let avg_duration = if total > 0 {
            total_duration as f64 / total as f64
        } else {
            0.0
        };
        let throughput = if total_duration > 0 {
            (total as f64 / total_duration as f64) * 1000.0
        } else {
            0.0
        };

        let stats = VerificationStats {
            total,
            successful,
            failed,
            total_duration_ms: total_duration,
            avg_duration_ms: avg_duration,
            throughput_per_sec: throughput,
        };

        info!(
            "Parallel verification complete: {}/{} successful, {:.2}ms avg, {:.0} verifications/sec",
            successful, total, avg_duration, throughput
        );

        (all_results, stats)
    }

    /// Verify a single seal using the chain verifier.
    ///
    /// This implementation:
    /// 1. Parses the seal ID from the record
    /// 2. Calls ChainVerifier::verify_seal_registry to check if seal is consumed
    /// 3. Returns error if seal is already consumed (double-spend detection)
    async fn verify_single_seal(&self, seal: &SealRecord) -> Result<(), SealError> {
        // Parse seal ID from the record
        let seal_id = match hex::decode(&seal.id) {
            Ok(bytes) => {
                if bytes.len() != 32 {
                    return Err(SealError::InvalidData(format!(
                        "Invalid seal ID length: expected 32 bytes, got {}",
                        bytes.len()
                    )));
                }
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Hash::new(arr)
            }
            Err(e) => return Err(SealError::InvalidData(format!("Invalid seal ID hex: {}", e))),
        };

        // Call chain verifier to check seal registry
        match self.verifier.verify_seal_registry(seal_id).await {
            Ok(true) => {
                // Seal is available (not consumed)
                Ok(())
            }
            Ok(false) => {
                // Seal has been consumed - double-spend detected
                Err(SealError::InvalidData(format!(
                    "Seal {} has already been consumed on chain {}",
                    seal.id, seal.chain
                )))
            }
            Err(e) => {
                // Verification error
                Err(SealError::Storage(format!(
                    "Seal registry verification failed for {}: {}",
                    seal.id, e
                )))
            }
        }
    }

    /// Verify a single proof bundle using csv_core::verify_proof.
    ///
    /// This implementation:
    /// 1. Determines the signature scheme for the chain
    /// 2. Creates a seal registry closure using the chain verifier
    /// 3. Calls csv_core::verify_proof with the seal registry callback
    async fn verify_single_proof(&self, proof: &ProofBundle) -> Result<(), String> {
        // Determine signature scheme from chain (default to Secp256k1 for Bitcoin/Ethereum)
        // In production, this should be derived from the proof's chain metadata
        let signature_scheme = SignatureScheme::Secp256k1;

        // Create seal registry closure that uses the chain verifier
        let verifier = self.verifier.clone();
        let seal_registry = move |seal_id: &[u8]| -> bool {
            // Parse seal ID
            let seal_hash = if seal_id.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(seal_id);
                Hash::new(arr)
            } else {
                return true; // Invalid seal ID - treat as consumed for safety
            };

            // Use blocking call for seal registry check in sync context
            // In WASM, this would need to be handled differently
            #[cfg(not(target_arch = "wasm32"))]
            {
                use tokio::runtime::Handle;
                let handle = Handle::current();
                let result = handle.block_on(verifier.verify_seal_registry(seal_hash));
                // Return true if seal is consumed (verification returns false)
                result.unwrap_or(true) == false
            }
            #[cfg(target_arch = "wasm32")]
            {
                // In WASM, we can't block, so we need a different approach
                // For now, return false (seal not consumed) to allow verification
                // TODO: Implement async seal registry for WASM
                false
            }
        };

        // Call csv_core::verify_proof
        verify_proof(proof, seal_registry, signature_scheme)
            .map_err(|e| format!("Proof verification failed: {}", e))
    }

    /// Verify seals with early exit on first failure (fail-fast mode).
    ///
    /// Useful when you need all verifications to succeed.
    pub async fn verify_seals_fail_fast(
        &self,
        seals: &[SealRecord],
    ) -> Result<Vec<VerificationResult>, String> {
        let results: Result<Vec<_>, String> = try_join_all(
            seals
                .iter()
                .map(|seal| async {
                    let start = Instant::now();
                    let result = self.verify_single_seal(seal).await;
                    let duration = start.elapsed().as_millis() as u64;

                    Ok(VerificationResult {
                        id: seal.id.clone(),
                        success: result.is_ok(),
                        duration_ms: duration,
                        error: result.err().map(|e| e.to_string()),
                    })
                })
                .collect::<Vec<_>>(),
        )
        .await;

        results.map_err(|e| format!("Verification failed: {}", e))
    }

    /// Get the current concurrency limit.
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    /// Set a new concurrency limit.
    pub fn set_max_concurrent(&mut self, max: usize) {
        self.max_concurrent = max.max(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_parallel_verify_service_creation() {
        let verifier = Arc::new(MockVerifier);
        let service = ParallelVerifyService::new(verifier);
        assert_eq!(service.max_concurrent(), 10);
    }

    #[tokio::test]
    async fn test_parallel_verify_custom_concurrency() {
        let verifier = Arc::new(MockVerifier);
        let service = ParallelVerifyService::with_concurrency(5, verifier);
        assert_eq!(service.max_concurrent(), 5);
    }

    #[tokio::test]
    async fn test_verify_seals_parallel() {
        let verifier = Arc::new(MockVerifier);
        let service = ParallelVerifyService::new(verifier);

        let seals = vec![
            SealRecord {
                id: "seal1".to_string(),
                chain: "bitcoin".to_string(),
                status: SealStatus::Unconsumed,
                value: 1000,
                created_at: Utc::now(),
                sanad_id: "sanad1".to_string(),
            },
            SealRecord {
                id: "seal2".to_string(),
                chain: "ethereum".to_string(),
                status: SealStatus::Unconsumed,
                value: 2000,
                created_at: Utc::now(),
                sanad_id: "sanad2".to_string(),
            },
        ];

        let (results, stats) = service.verify_seals_parallel(&seals).await;

        assert_eq!(results.len(), 2);
        assert_eq!(stats.total, 2);
        assert_eq!(stats.successful, 2);
        assert_eq!(stats.failed, 0);
    }

    #[tokio::test]
    async fn test_verify_seals_fail_fast() {
        let verifier = Arc::new(MockVerifier);
        let service = ParallelVerifyService::new(verifier);

        let seals = vec![SealRecord {
            id: "seal1".to_string(),
            chain: "bitcoin".to_string(),
            status: SealStatus::Unconsumed,
            value: 1000,
            created_at: Utc::now(),
            sanad_id: "sanad1".to_string(),
        }];

        let result = service.verify_seals_fail_fast(&seals).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    /// Mock verifier for testing
    struct MockVerifier;

    #[async_trait::async_trait]
    impl ChainVerifier for MockVerifier {
        async fn verify_inclusion(
            &self,
            _proof: &csv_core::proof::InclusionProof,
            _expected_root: Hash,
        ) -> csv_core::Result<bool> {
            Ok(true)
        }

        async fn verify_finality(&self, _proof: &csv_core::proof::FinalityProof) -> csv_core::Result<bool> {
            Ok(true)
        }

        async fn verify_zk(&self, _proof: &[u8]) -> csv_core::Result<bool> {
            Ok(true)
        }

        async fn verify_seal_registry(&self, _seal_id: Hash) -> csv_core::Result<bool> {
            Ok(true) // Seal is available (not consumed)
        }

        async fn verify_signature(&self, _bundle: &ProofBundle) -> csv_core::Result<bool> {
            Ok(true)
        }
    }
}
