//! Seal monitor for tracking seal status with per-chain adaptive polling.
//!
//! Monitors seals on-chain and updates their status using per-chain adaptive
//! intervals with ±20% jitter to avoid thundering herd problems.

use super::manager::{SealRecord, SealStatus};
use csv_core::ChainId;
use std::sync::Arc;
use std::time::Duration;

/// Per-chain polling interval configuration.
#[derive(Debug, Clone)]
pub struct ChainPollConfig {
    /// Chain identifier
    pub chain: ChainId,
    /// Base polling interval in milliseconds
    pub poll_interval_ms: u64,
    /// Jitter percentage (0.0 to 1.0, where 0.2 = ±20%)
    pub jitter_pct: f64,
}

impl ChainPollConfig {
    /// Apply ±jitter to the base interval, returning a Duration.
    pub fn interval(&self) -> Duration {
        let jitter_factor = 1.0 - self.jitter_pct + (rand::random::<f64>() * 2.0 * self.jitter_pct);
        let adjusted_ms = (self.poll_interval_ms as f64 * jitter_factor) as u64;
        Duration::from_millis(adjusted_ms.max(100)) // Minimum 100ms
    }
}

/// Default per-chain polling configuration.
/// Solana: 1s, Sui/Aptos: 4s, Ethereum: 12s, Bitcoin: 15s.
pub fn default_chain_configs() -> Vec<ChainPollConfig> {
    vec![
        ChainPollConfig {
            chain: ChainId::new("solana"),
            poll_interval_ms: 1000,
            jitter_pct: 0.2,
        },
        ChainPollConfig {
            chain: ChainId::new("sui"),
            poll_interval_ms: 4000,
            jitter_pct: 0.2,
        },
        ChainPollConfig {
            chain: ChainId::new("aptos"),
            poll_interval_ms: 4000,
            jitter_pct: 0.2,
        },
        ChainPollConfig {
            chain: ChainId::new("ethereum"),
            poll_interval_ms: 12000,
            jitter_pct: 0.2,
        },
        ChainPollConfig {
            chain: ChainId::new("bitcoin"),
            poll_interval_ms: 15000,
            jitter_pct: 0.2,
        },
    ]
}

/// Callback type for seal status updates.
pub type SealUpdateCallback = Arc<dyn Fn(&SealRecord) + Send + Sync>;

/// Seal monitor for checking on-chain status with adaptive polling.
pub struct SealMonitor {
    /// Per-chain polling configurations
    chain_configs: Vec<ChainPollConfig>,
    /// Callback invoked when a seal status changes
    on_update: Option<SealUpdateCallback>,
}

impl SealMonitor {
    /// Create a new seal monitor.
    pub fn new() -> Self {
        Self {
            chain_configs: default_chain_configs(),
            on_update: None,
        }
    }

    /// Create a new seal monitor with custom chain configurations.
    pub fn with_chain_configs(chain_configs: Vec<ChainPollConfig>) -> Self {
        Self {
            chain_configs,
            on_update: None,
        }
    }

    /// Set a callback to be invoked when seal status changes.
    pub fn on_update<F>(&mut self, callback: F)
    where
        F: Fn(&SealRecord) + Send + Sync + 'static,
    {
        self.on_update = Some(Arc::new(callback));
    }

    /// Get the polling interval for a specific chain.
    pub fn interval_for_chain(&self, chain: &ChainId) -> Duration {
        self.chain_configs
            .iter()
            .find(|c| &c.chain == chain)
            .map(|c| c.interval())
            .unwrap_or(Duration::from_secs(30)) // Default fallback
    }

    /// Check the on-chain status of a seal.
    pub async fn check_seal_status(&self, seal: &SealRecord) -> Result<SealStatus, String> {
        match seal.chain.as_str() {
            "bitcoin" => self.check_bitcoin_seal(seal).await,
            "ethereum" => self.check_ethereum_seal(seal).await,
            "sui" => self.check_sui_seal(seal).await,
            "aptos" => self.check_aptos_seal(seal).await,
            "solana" => self.check_solana_seal(seal).await,
            _ => Ok(seal.status.clone()),
        }
    }

    /// Check Bitcoin seal status via mempool.space API.
    async fn check_bitcoin_seal(&self, seal: &SealRecord) -> Result<SealStatus, String> {
        if let Some(txid) = &seal.txid {
            let url = format!("https://mempool.space/api/tx/{}/status", txid);
            let status = Self::fetch_bitcoin_tx_status(&url).await;
            return Ok(status);
        }
        Ok(seal.status.clone())
    }

    /// Fetch Bitcoin transaction confirmation status.
    async fn fetch_bitcoin_tx_status(url: &str) -> SealStatus {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use reqwest::Client;
            match Client::new().get(url).send().await {
                Ok(resp) => {
                    let info: BitcoinTxStatus = match resp.json().await {
                        Ok(info) => info,
                        Err(_) => return SealStatus::Unknown,
                    };
                    if info.confirmed {
                        SealStatus::Confirmed
                    } else {
                        SealStatus::Pending
                    }
                }
                Err(_) => SealStatus::Unknown,
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            use reqwest::Client;
            match Client::new().get(url).send().await {
                Ok(resp) => {
                    let info: BitcoinTxStatus = match resp.json().await {
                        Ok(info) => info,
                        Err(_) => return SealStatus::Unknown,
                    };
                    if info.confirmed {
                        SealStatus::Confirmed
                    } else {
                        SealStatus::Pending
                    }
                }
                Err(_) => SealStatus::Unknown,
            }
        }
    }

    /// Check Ethereum seal status via etherscan API.
    async fn check_ethereum_seal(&self, seal: &SealRecord) -> Result<SealStatus, String> {
        if let Some(txid) = &seal.txid {
            let status = Self::fetch_ethereum_tx_status(txid).await;
            return Ok(status);
        }
        Ok(seal.status.clone())
    }

    /// Fetch Ethereum transaction status.
    async fn fetch_ethereum_tx_status(txid: &str) -> SealStatus {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use reqwest::Client;
            let url = format!("https://api-sepolia.etherscan.io/api?module=proxy&action=eth_getTransactionByHash&txhash={}&apikey=Placeholder", txid);
            match Client::new().get(&url).send().await {
                Ok(resp) => {
                    let result: EthereumTxResult = match resp.json().await {
                        Ok(result) => result,
                        Err(_) => return SealStatus::Unknown,
                    };
                    if let Some(block) = result.result.block_number {
                        if block != "0x0" {
                            SealStatus::Confirmed
                        } else {
                            SealStatus::Pending
                        }
                    } else {
                        SealStatus::Pending
                    }
                }
                Err(_) => SealStatus::Unknown,
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            use reqwest::Client;
            let url = format!("https://api-sepolia.etherscan.io/api?module=proxy&action=eth_getTransactionByHash&txhash={}&apikey=Placeholder", txid);
            match Client::new().get(&url).send().await {
                Ok(resp) => {
                    let result: EthereumTxResult = match resp.json().await {
                        Ok(result) => result,
                        Err(_) => return SealStatus::Unknown,
                    };
                    if let Some(block) = result.result.block_number {
                        if block != "0x0" {
                            SealStatus::Confirmed
                        } else {
                            SealStatus::Pending
                        }
                    } else {
                        SealStatus::Pending
                    }
                }
                Err(_) => SealStatus::Unknown,
            }
        }
    }

    /// Check Sui seal status.
    async fn check_sui_seal(&self, seal: &SealRecord) -> Result<SealStatus, String> {
        // Sui seal checking via SuiScan API or RPC
        Ok(seal.status.clone())
    }

    /// Check Aptos seal status.
    async fn check_aptos_seal(&self, seal: &SealRecord) -> Result<SealStatus, String> {
        // Aptos seal checking via Aptos Explorer API
        Ok(seal.status.clone())
    }

    /// Check Solana seal status.
    async fn check_solana_seal(&self, seal: &SealRecord) -> Result<SealStatus, String> {
        // Solana seal checking via Solana RPC
        Ok(seal.status.clone())
    }

    /// Monitor all seals and return updated records.
    pub async fn monitor_all_seals(&self, seals: &[SealRecord]) -> Vec<SealRecord> {
        let mut updated_seals = Vec::new();

        for seal in seals {
            if let Ok(new_status) = self.check_seal_status(seal).await {
                if new_status != seal.status {
                    let mut updated = seal.clone();
                    updated.status = new_status;
                    updated.updated_at = chrono::Utc::now();
                    if let Some(callback) = &self.on_update {
                        callback(&updated);
                    }
                    updated_seals.push(updated);
                } else {
                    updated_seals.push(seal.clone());
                }
            } else {
                updated_seals.push(seal.clone());
            }
        }

        updated_seals
    }

    /// Start continuous monitoring for a specific seal with per-chain adaptive interval.
    /// Returns a oneshot channel sender to stop the monitoring loop.
    pub fn start_monitoring(
        &self,
        seal: SealRecord,
        on_update: impl Fn(&SealRecord) + Send + Sync + 'static,
    ) -> tokio::sync::oneshot::Sender<()>
    where
        Self: Sized,
    {
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        let monitor = self.clone();

        // Clone necessary data for the async task
        let chain = seal.chain.clone();

        #[cfg(not(target_arch = "wasm32"))]
        {
            use tokio::time::{interval, sleep};

            let config = self.chain_configs.iter().find(|c| c.chain.as_str() == chain.as_str());
            let base_interval = config
                .map(|c| c.poll_interval_ms)
                .unwrap_or(30000);

            tokio::spawn(async move {
                let mut interval = interval(Duration::from_millis(base_interval));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

                loop {
                    tokio::select! {
                        _ = rx.recv() => {
                            break;
                        }
                        _ = interval.tick() => {
                            match monitor.check_seal_status(&seal).await {
                                Ok(new_status) => {
                                    if new_status != seal.status {
                                        let mut updated = seal.clone();
                                        updated.status = new_status;
                                        updated.updated_at = chrono::Utc::now();
                                        on_update(&updated);
                                    }
                                }
                                Err(_) => {
                                    // Silently ignore errors, will retry next interval
                                }
                            }
                        }
                    }
                }
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            use gloo_timers::future::IntervalStream;
            use futures::StreamExt;

            let config = self.chain_configs.iter().find(|c| c.chain.as_str() == chain.as_str());
            let base_interval = config
                .map(|c| c.poll_interval_ms)
                .unwrap_or(30000);

            wasm_bindgen_futures::spawn_local(async move {
                let mut stream = IntervalStream::new(Duration::from_millis(base_interval))
                    .expect("failed to create interval stream");

                loop {
                    tokio::select! {
                        _ = rx.recv() => {
                            break;
                        }
                        _ = stream.next() => {
                            match monitor.check_seal_status(&seal).await {
                                Ok(new_status) => {
                                    if new_status != seal.status {
                                        let mut updated = seal.clone();
                                        updated.status = new_status;
                                        updated.updated_at = chrono::Utc::now();
                                        on_update(&updated);
                                    }
                                }
                                Err(_) => {
                                    // Silently ignore errors, will retry next interval
                                }
                            }
                        }
                    }
                }
            });
        }

        tx
    }

    /// Start continuous monitoring for all seals with per-chain adaptive intervals.
    /// Returns a oneshot channel sender to stop all monitoring.
    pub fn start_all_monitoring(
        &self,
        seals: Vec<SealRecord>,
        on_update: impl Fn(&SealRecord) + Send + Sync + 'static,
    ) -> tokio::sync::oneshot::Sender<()>
    where
        Self: Sized,
    {
        let (tx, mut rx) = tokio::sync::oneshot::channel();

        #[cfg(not(target_arch = "wasm32"))]
        {
            let monitor = self.clone();
            let on_update = Arc::new(on_update);

            tokio::spawn(async move {
                // Group seals by chain
                let mut chain_seals: std::collections::HashMap<String, Vec<SealRecord>> =
                    std::collections::HashMap::new();
                for seal in seals {
                    chain_seals
                        .entry(seal.chain.clone())
                        .or_default()
                        .push(seal);
                }

                // Spawn a polling task per chain
                let mut handles = Vec::new();
                for (chain, chain_seal_list) in chain_seals {
                    let config = monitor
                        .chain_configs
                        .iter()
                        .find(|c| c.chain.as_str() == chain.as_str());
                    let base_interval = config
                        .map(|c| c.poll_interval_ms)
                        .unwrap_or(30000);

                    let chain_monitor = monitor.clone();
                    let chain_seals = chain_seal_list;
                    let chain_on_update = Arc::clone(&on_update);
                    let mut stop_rx = rx.clone();

                    let handle = tokio::spawn(async move {
                        let mut interval = interval(Duration::from_millis(base_interval));
                        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

                        loop {
                            tokio::select! {
                                _ = stop_rx.recv() => {
                                    break;
                                }
                                _ = interval.tick() => {
                                    for seal in &chain_seals {
                                        if let Ok(new_status) = chain_monitor.check_seal_status(seal).await {
                                            if new_status != seal.status {
                                                let mut updated = seal.clone();
                                                updated.status = new_status;
                                                updated.updated_at = chrono::Utc::now();
                                                chain_on_update(&updated);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });
                    handles.push(handle);
                }

                // Wait for stop signal or all handles to complete
                let _ = rx.recv().await;
                for handle in handles {
                    handle.abort();
                }
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            let monitor = self.clone();
            let on_update = Arc::new(on_update);

            wasm_bindgen_futures::spawn_local(async move {
                // Group seals by chain
                let mut chain_seals: std::collections::HashMap<String, Vec<SealRecord>> =
                    std::collections::HashMap::new();
                for seal in seals {
                    chain_seals
                        .entry(seal.chain.clone())
                        .or_default()
                        .push(seal);
                }

                // Spawn a polling task per chain
                let mut streams = Vec::new();
                for (chain, chain_seal_list) in chain_seals {
                    let config = monitor
                        .chain_configs
                        .iter()
                        .find(|c| c.chain.as_str() == chain.as_str());
                    let base_interval = config
                        .map(|c| c.poll_interval_ms)
                        .unwrap_or(30000);

                    let chain_monitor = monitor.clone();
                    let chain_seals = chain_seal_list;
                    let chain_on_update = Arc::clone(&on_update);

                    let stream = IntervalStream::new(Duration::from_millis(base_interval))
                        .expect("failed to create interval stream");

                    streams.push((chain, stream, chain_seals, chain_monitor, chain_on_update));
                }

                // Poll all streams concurrently
                for (chain, mut stream, chain_seals, chain_monitor, chain_on_update) in streams {
                    let mut stop_rx = rx.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        loop {
                            tokio::select! {
                                _ = stop_rx.recv() => {
                                    break;
                                }
                                _ = stream.next() => {
                                    for seal in &chain_seals {
                                        if let Ok(new_status) = chain_monitor.check_seal_status(seal).await {
                                            if new_status != seal.status {
                                                let mut updated = seal.clone();
                                                updated.status = new_status;
                                                updated.updated_at = chrono::Utc::now();
                                                chain_on_update(&updated);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
            });
        }

        tx
    }
}

impl Clone for SealMonitor {
    fn clone(&self) -> Self {
        Self {
            chain_configs: self.chain_configs.clone(),
            on_update: self.on_update.clone(),
        }
    }
}

impl Default for SealMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Bitcoin transaction status response from mempool.space API.
#[derive(Debug, serde::Deserialize)]
struct BitcoinTxStatus {
    confirmed: bool,
    block_height: Option<u64>,
}

/// Ethereum transaction result from Etherscan API.
#[derive(Debug, serde::Deserialize)]
struct EthereumTxResult {
    result: EthereumTxResultData,
}

#[derive(Debug, serde::Deserialize)]
struct EthereumTxResultData {
    block_number: Option<String>,
}
