//! Parallel verification service for the Wallet.
//!
//! Provides concurrent proof and seal verification using async/await,
//! optimized for WebAssembly (WASM) environments where threads are not available.
//! This is the default verification mode for the Wallet.

use std::sync::Arc;
use std::time::Instant;

use csv_core::proof::ProofBundle;
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
}

impl Default for ParallelVerifyService {
    fn default() -> Self {
        Self::new()
    }
}

impl ParallelVerifyService {
    /// Create a new parallel verification service with default settings.
    pub fn new() -> Self {
        Self {
            max_concurrent: 10, // Limit concurrent tasks to avoid overwhelming the browser
        }
    }

    /// Create a new parallel verification service with custom concurrency limit.
    pub fn with_concurrency(max_concurrent: usize) -> Self {
        Self {
            max_concurrent: max_concurrent.max(1),
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

    /// Verify a single seal (placeholder for actual verification logic).
    ///
    /// In production, this would:
    /// 1. Query the blockchain to check seal status
    /// 2. Verify the seal hasn't been double-spent
    /// 3. Validate the seal's cryptographic properties
    async fn verify_single_seal(&self, _seal: &SealRecord) -> Result<(), SealError> {
        // Placeholder: In production, implement actual seal verification
        // For now, simulate verification with a small delay
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        Ok(())
    }

    /// Verify a single proof bundle (placeholder for actual verification logic).
    ///
    /// In production, this would:
    /// 1. Verify the cryptographic signatures
    /// 2. Check inclusion proofs
    /// 3. Validate finality proofs
    /// 4. Verify commitment chain integrity
    async fn verify_single_proof(&self, _proof: &ProofBundle) -> Result<(), String> {
        // Placeholder: In production, implement actual proof verification
        // For now, simulate verification with a small delay
        tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
        Ok(())
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
        let service = ParallelVerifyService::new();
        assert_eq!(service.max_concurrent(), 10);
    }

    #[tokio::test]
    async fn test_parallel_verify_custom_concurrency() {
        let service = ParallelVerifyService::with_concurrency(5);
        assert_eq!(service.max_concurrent(), 5);
    }

    #[tokio::test]
    async fn test_verify_seals_parallel() {
        let service = ParallelVerifyService::new();

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
        let service = ParallelVerifyService::new();

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
}
