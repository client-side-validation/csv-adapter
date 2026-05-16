//! RPC Metrics
//!
//! This module provides metrics collection for RPC operations,
//! tracking latency, success rates, provider health, and quorum disagreements.
//!
//! ## Metrics
//!
//! - `rpc_disagreement_total` — Count of RPC provider disagreements (quorum failures)
//! - `rpc_latency_ms` — Latency of RPC requests in milliseconds
//! - `provider_failure_total` — Count of provider failures
//! - `provider_timeout_total` — Count of provider timeouts

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

/// RPC operation metrics with atomic counters for thread safety
#[derive(Debug)]
pub struct RpcMetrics {
    /// Total number of requests
    total_requests: AtomicU64,
    /// Number of successful requests
    successful_requests: AtomicU64,
    /// Number of failed requests
    failed_requests: AtomicU64,
    /// Number of timeout failures
    timeout_failures: AtomicU64,
    /// Number of quorum disagreements
    disagreement_count: AtomicU64,
    /// Total latency in milliseconds
    total_latency_ms: AtomicU64,
    /// Provider-specific metrics
    provider_metrics: BTreeMap<String, Arc<ProviderMetrics>>,
}

/// Provider-specific metrics with atomic counters
#[derive(Debug)]
pub struct ProviderMetrics {
    /// Provider URL
    pub url: String,
    /// Number of requests to this provider
    requests: AtomicU64,
    /// Number of successful requests
    successful: AtomicU64,
    /// Number of failed requests
    failed: AtomicU64,
    /// Number of timeout failures
    timeouts: AtomicU64,
    /// Total latency in milliseconds
    total_latency_ms: AtomicU64,
    /// Last successful response timestamp
    last_success: AtomicU64,
    /// Last failure timestamp
    last_failure: AtomicU64,
}

impl ProviderMetrics {
    /// Create new provider metrics
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            requests: AtomicU64::new(0),
            successful: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            timeouts: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            last_success: AtomicU64::new(0),
            last_failure: AtomicU64::new(0),
        }
    }

    /// Record a successful request
    pub fn record_success(&self, latency_ms: u64) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.successful.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ms
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.last_success.store(latency_ms, Ordering::Relaxed);
    }

    /// Record a failed request
    pub fn record_failure(&self) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.failed.fetch_add(1, Ordering::Relaxed);
        self.last_failure.store(0, Ordering::Relaxed);
    }

    /// Record a timeout
    pub fn record_timeout(&self) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.timeouts.fetch_add(1, Ordering::Relaxed);
        self.last_failure.store(0, Ordering::Relaxed);
    }

    /// Get average latency
    pub fn avg_latency_ms(&self) -> f64 {
        let requests = self.requests.load(Ordering::Relaxed);
        if requests == 0 {
            return 0.0;
        }
        self.total_latency_ms.load(Ordering::Relaxed) as f64 / requests as f64
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        let requests = self.requests.load(Ordering::Relaxed);
        if requests == 0 {
            return 0.0;
        }
        self.successful.load(Ordering::Relaxed) as f64 / requests as f64
    }

    /// Get snapshot for display
    pub fn snapshot(&self) -> ProviderSnapshot {
        ProviderSnapshot {
            url: self.url.clone(),
            requests: self.requests.load(Ordering::Relaxed),
            successful: self.successful.load(Ordering::Relaxed),
            failed: self.failed.load(Ordering::Relaxed),
            timeouts: self.timeouts.load(Ordering::Relaxed),
            avg_latency_ms: self.avg_latency_ms(),
            success_rate: self.success_rate(),
        }
    }
}

/// Snapshot of provider metrics for display
#[derive(Debug)]
pub struct ProviderSnapshot {
    pub url: String,
    pub requests: u64,
    pub successful: u64,
    pub failed: u64,
    pub timeouts: u64,
    pub avg_latency_ms: f64,
    pub success_rate: f64,
}

/// Snapshot of RPC metrics for display
#[derive(Debug)]
pub struct RpcMetricsSnapshot {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub timeout_failures: u64,
    pub disagreement_count: u64,
    pub avg_latency_ms: f64,
    pub success_rate: f64,
    pub providers: Vec<ProviderSnapshot>,
}

impl RpcMetrics {
    /// Create new RPC metrics
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            timeout_failures: AtomicU64::new(0),
            disagreement_count: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            provider_metrics: BTreeMap::new(),
        }
    }

    /// Get or create provider metrics
    fn get_or_create_provider(&mut self, provider: &str) -> Arc<ProviderMetrics> {
        if let Some(metrics) = self.provider_metrics.get(provider) {
            return metrics.clone();
        }
        let metrics = Arc::new(ProviderMetrics::new(provider));
        self.provider_metrics
            .insert(provider.to_string(), metrics.clone());
        metrics
    }

    /// Record a successful RPC request
    pub fn record_success(&mut self, provider: &str, latency_ms: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ms
            .fetch_add(latency_ms, Ordering::Relaxed);

        let metrics = self.get_or_create_provider(provider);
        metrics.record_success(latency_ms);
    }

    /// Record a failed RPC request
    pub fn record_failure(&mut self, provider: &str) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.failed_requests.fetch_add(1, Ordering::Relaxed);

        let metrics = self.get_or_create_provider(provider);
        metrics.record_failure();
    }

    /// Record a timeout
    pub fn record_timeout(&mut self, provider: &str) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.timeout_failures.fetch_add(1, Ordering::Relaxed);

        let metrics = self.get_or_create_provider(provider);
        metrics.record_timeout();
    }

    /// Record a quorum disagreement
    pub fn record_disagreement(&mut self) {
        self.disagreement_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        self.successful_requests.load(Ordering::Relaxed) as f64 / total as f64
    }

    /// Get average latency
    pub fn avg_latency_ms(&self) -> f64 {
        let successful = self.successful_requests.load(Ordering::Relaxed);
        if successful == 0 {
            return 0.0;
        }
        self.total_latency_ms.load(Ordering::Relaxed) as f64 / successful as f64
    }

    /// Get provider metrics
    pub fn get_provider_metrics(&self, provider: &str) -> Option<Arc<ProviderMetrics>> {
        self.provider_metrics.get(provider).cloned()
    }

    /// Get all provider metrics
    pub fn all_provider_metrics(&self) -> Vec<Arc<ProviderMetrics>> {
        self.provider_metrics.values().cloned().collect()
    }

    /// Get a snapshot of all metrics
    pub fn snapshot(&self) -> RpcMetricsSnapshot {
        let total = self.total_requests.load(Ordering::Relaxed);
        let successful = self.successful_requests.load(Ordering::Relaxed);
        let failed = self.failed_requests.load(Ordering::Relaxed);
        let timeouts = self.timeout_failures.load(Ordering::Relaxed);
        let disagreements = self.disagreement_count.load(Ordering::Relaxed);

        RpcMetricsSnapshot {
            total_requests: total,
            successful_requests: successful,
            failed_requests: failed,
            timeout_failures: timeouts,
            disagreement_count: disagreements,
            avg_latency_ms: if successful == 0 {
                0.0
            } else {
                self.total_latency_ms.load(Ordering::Relaxed) as f64 / successful as f64
            },
            success_rate: if total == 0 {
                0.0
            } else {
                successful as f64 / total as f64
            },
            providers: self
                .provider_metrics
                .values()
                .map(|m| m.snapshot())
                .collect(),
        }
    }
}

impl Default for RpcMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_metrics_creation() {
        let metrics = RpcMetrics::new();
        assert_eq!(metrics.total_requests.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.successful_requests.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_success() {
        let mut metrics = RpcMetrics::new();
        metrics.record_success("provider1", 100);

        assert_eq!(metrics.total_requests.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.successful_requests.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.success_rate(), 1.0);
    }

    #[test]
    fn test_record_failure() {
        let mut metrics = RpcMetrics::new();
        metrics.record_failure("provider1");

        assert_eq!(metrics.total_requests.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.failed_requests.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.success_rate(), 0.0);
    }

    #[test]
    fn test_record_timeout() {
        let mut metrics = RpcMetrics::new();
        metrics.record_timeout("provider1");

        assert_eq!(metrics.timeout_failures.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_record_disagreement() {
        let mut metrics = RpcMetrics::new();
        metrics.record_disagreement();

        assert_eq!(metrics.disagreement_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_avg_latency() {
        let mut metrics = RpcMetrics::new();
        metrics.record_success("provider1", 100);
        metrics.record_success("provider1", 200);

        assert_eq!(metrics.avg_latency_ms(), 150.0);
    }

    #[test]
    fn test_provider_metrics() {
        let mut metrics = RpcMetrics::new();
        metrics.record_success("provider1", 100);
        metrics.record_failure("provider1");
        metrics.record_timeout("provider1");

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.providers.len(), 1);
        assert_eq!(snapshot.providers[0].requests, 3);
        assert_eq!(snapshot.providers[0].successful, 1);
        assert_eq!(snapshot.providers[0].failed, 1);
        assert_eq!(snapshot.providers[0].timeouts, 1);
    }
}
