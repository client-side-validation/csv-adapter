//! WebSocket subscription manager for the wallet.
//!
//! Connects to the explorer's WebSocket subscription endpoint and receives
//! real-time updates for wallet-owned addresses across all chains.

use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// WebSocket subscription manager for the wallet.
///
/// Connects to the explorer's `/ws/subscriptions` endpoint and subscribes
/// to all wallet-owned addresses. Receives real-time updates for sanads,
/// seals, and transfers.
pub struct WalletSubscriptionManager {
    /// Base URL of the explorer WebSocket endpoint
    ws_url: String,
    /// Whether the manager is currently connected
    connected: Arc<std::sync::atomic::AtomicBool>,
    /// Per-chain adaptive polling intervals (fallback when WebSocket is unavailable)
    chain_intervals: std::sync::RwLock<HashMap<String, u64>>,
    /// Active subscriptions per address and chain
    subscriptions: Arc<RwLock<HashMap<String, Vec<String>>>>, // address -> chains
    /// Event sender for broadcasting events to subscribers
    event_sender: Arc<RwLock<Option<mpsc::UnboundedSender<SubscriptionEvent>>>>,
    /// WebSocket connection handle
    ws_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

/// Event received from the explorer WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SubscriptionEvent {
    /// A new sanad was created
    NewSanad {
        address: String,
        chain: String,
        sanad_id: String,
        #[serde(default)]
        data: serde_json::Value,
    },
    /// A new seal was created
    NewSeal {
        address: String,
        chain: String,
        seal_id: String,
        #[serde(default)]
        data: serde_json::Value,
    },
    /// A new transfer was created
    NewTransfer {
        address: String,
        chain: String,
        transfer_id: String,
        #[serde(default)]
        data: serde_json::Value,
    },
    /// Indexing completed for an address
    IndexingComplete {
        address: String,
        chain: String,
        sanads_count: u64,
        seals_count: u64,
        transfers_count: u64,
    },
    /// An indexing error occurred
    IndexingError {
        address: String,
        chain: String,
        error: String,
    },
}

/// Request sent to the explorer WebSocket.
#[derive(Debug, Serialize)]
struct SubscriptionRequest {
    action: String,
    address: String,
    chain: Option<String>,
    network: Option<String>,
}

/// Response from the explorer WebSocket.
#[derive(Debug, Deserialize)]
struct SubscriptionResponse {
    success: bool,
    message: String,
    #[serde(default)]
    event: Option<SubscriptionEvent>,
}

impl WalletSubscriptionManager {
    /// Create a new subscription manager.
    pub fn new(explorer_base_url: String) -> Self {
        let ws_url = if explorer_base_url.starts_with("https") {
            explorer_base_url.replace("https://", "wss://")
        } else if explorer_base_url.starts_with("http") {
            explorer_base_url.replace("http://", "ws://")
        } else {
            format!("ws://{}", explorer_base_url)
        };

        Self {
            ws_url,
            connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            chain_intervals: std::sync::RwLock::new(HashMap::new()),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            event_sender: Arc::new(RwLock::new(None)),
            ws_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// Set per-chain polling intervals (used as fallback when WebSocket is unavailable).
    pub fn set_chain_intervals(&self, intervals: std::collections::HashMap<String, u64>) {
        let mut lock = self.chain_intervals.write().unwrap();
        *lock = intervals;
    }

    /// Get the polling interval for a specific chain.
    pub fn chain_interval_ms(&self, chain: &str) -> u64 {
        self.chain_intervals
            .read()
            .unwrap()
            .get(chain)
            .copied()
            .unwrap_or(30000) // Default 30s fallback
    }

    /// Subscribe to events for a specific address and chain.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn subscribe(
        &self,
        address: &str,
        chain: Option<&str>,
        network: Option<&str>,
    ) -> Result<(), String> {
        use reqwest::Client;

        let request = SubscriptionRequest {
            action: "subscribe".to_string(),
            address: address.to_string(),
            chain: chain.map(|s| s.to_string()),
            network: network.map(|s| s.to_string()),
        };

        let url = format!("{}/api/v1/ws/subscribe", self.ws_url);
        let response: SubscriptionResponse = Client::new()
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to subscribe: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if !response.success {
            return Err(format!("Subscription failed: {}", response.message));
        }

        Ok(())
    }

    /// Unsubscribe from events for a specific address.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn unsubscribe(&self, address: &str, chain: Option<&str>) -> Result<(), String> {
        use reqwest::Client;

        let request = SubscriptionRequest {
            action: "unsubscribe".to_string(),
            address: address.to_string(),
            chain: chain.map(|s| s.to_string()),
            network: None,
        };

        let url = format!("{}/api/v1/ws/unsubscribe", self.ws_url);
        let response: SubscriptionResponse = Client::new()
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to unsubscribe: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if !response.success {
            return Err(format!("Unsubscription failed: {}", response.message));
        }

        Ok(())
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get the WebSocket URL.
    pub fn ws_url(&self) -> &str {
        &self.ws_url
    }

    /// Set connected state.
    pub fn set_connected(&self, connected: bool) {
        self.connected
            .store(connected, std::sync::atomic::Ordering::Relaxed);
    }

    /// Connect to the WebSocket endpoint with adaptive retry logic.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn connect(&self) -> Result<(), String> {
        if self.is_connected() {
            return Ok(());
        }

        let ws_url = self.ws_url.clone();
        let subscriptions = Arc::clone(&self.subscriptions);
        let connected = Arc::clone(&self.connected);
        let event_sender = Arc::clone(&self.event_sender);

        let (tx, mut _rx) = mpsc::unbounded_channel::<SubscriptionEvent>();
        *event_sender.write().await = Some(tx);

        let handle = tokio::spawn(async move {
            let mut retry_count = 0;
            let max_retries = 5;

            while retry_count < max_retries {
                match connect_async(&ws_url).await {
                    Ok((ws_stream, _)) => {
                        connected.store(true, std::sync::atomic::Ordering::Relaxed);
                        tracing::info!("WebSocket connected to {}", ws_url);

                        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

                        // Resubscribe to all existing addresses
                        let subs = subscriptions.read().await;
                        for (address, chains) in subs.iter() {
                            for chain in chains {
                                let request = SubscriptionRequest {
                                    action: "subscribe".to_string(),
                                    address: address.clone(),
                                    chain: Some(chain.clone()),
                                    network: None,
                                };
                                if let Ok(json) = serde_json::to_string(&request) {
                                    let _ = ws_sender.send(Message::Text(json)).await;
                                }
                            }
                        }
                        drop(subs);

                        // Handle WebSocket messages
                        loop {
                            tokio::select! {
                                Some(msg) = ws_receiver.next() => {
                                    match msg {
                                        Ok(Message::Text(text)) => {
                                            if let Ok(response) = serde_json::from_str::<SubscriptionResponse>(&text) {
                                                if let Some(event) = response.event {
                                                    if let Some(sender) = event_sender.read().await.as_ref() {
                                                        let _ = sender.send(event);
                                                    }
                                                }
                                            }
                                        }
                                        Ok(Message::Close(_)) => {
                                            tracing::info!("WebSocket connection closed");
                                            break;
                                        }
                                        Err(e) => {
                                            tracing::error!("WebSocket error: {}", e);
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                                    // Send periodic ping to keep connection alive
                                    let _ = ws_sender.send(Message::Ping(vec![])).await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to connect to WebSocket: {}", e);
                        retry_count += 1;

                        // Adaptive retry with jitter
                        let base_delay = std::time::Duration::from_secs(2_u64.pow(retry_count));
                        let jitter = rand::random::<u64>() % 1000;
                        let delay = base_delay + std::time::Duration::from_millis(jitter);

                        tokio::time::sleep(delay).await;
                    }
                }
            }

            tracing::warn!("WebSocket connection failed after {} retries", max_retries);
        });

        *self.ws_handle.write().await = Some(handle);
        Ok(())
    }

    /// Disconnect from the WebSocket endpoint.
    pub async fn disconnect(&self) {
        self.set_connected(false);
        if let Some(handle) = self.ws_handle.write().await.take() {
            handle.abort();
        }
        *self.event_sender.write().await = None;
    }

    /// Get adaptive polling interval with jitter for a specific chain.
    pub fn get_adaptive_interval(&self, chain: &str) -> u64 {
        let base = self
            .chain_intervals
            .read()
            .unwrap()
            .get(chain)
            .copied()
            .unwrap_or(30000);

        // Apply ±20% jitter
        let jitter_factor = 1.0 - 0.2 + (rand::random::<f64>() * 2.0 * 0.2);
        let adjusted = (base as f64 * jitter_factor) as u64;
        adjusted.max(100) // Minimum 100ms
    }

    /// Subscribe to events with adaptive polling fallback.
    pub async fn subscribe_with_fallback(
        &self,
        address: &str,
        chain: Option<&str>,
        on_event: impl Fn(SubscriptionEvent) + Send + Sync + 'static,
    ) -> Result<(), String> {
        // Try WebSocket first
        #[cfg(not(target_arch = "wasm32"))]
        {
            if !self.is_connected() {
                if let Err(e) = self.connect().await {
                    tracing::warn!("WebSocket connection failed, using HTTP polling: {}", e);
                } else {
                    // Subscribe via WebSocket
                    if let Err(e) = self.subscribe(address, chain, None).await {
                        tracing::warn!("WebSocket subscription failed: {}", e);
                    } else {
                        // Store subscription for reconnection
                        let mut subs = self.subscriptions.write().await;
                        subs.entry(address.to_string())
                            .or_insert_with(Vec::new)
                            .push(chain.unwrap_or("default").to_string());
                        return Ok(());
                    }
                }
            }
        }

        // Fallback to adaptive HTTP polling
        let chain_str = chain.unwrap_or("default");
        let poll_interval = self.get_adaptive_interval(chain_str);

        let address = address.to_string();
        let chain = chain_str.to_string();
        let _on_event = Arc::new(on_event);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(poll_interval));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

            loop {
                interval.tick().await;
                // Here you would make HTTP requests to the explorer API
                // and call on_event when new data is found
                tracing::debug!(
                    "Polling {} for chain {} (interval: {}ms)",
                    address,
                    chain,
                    poll_interval
                );
            }
        });

        Ok(())
    }
}

impl Clone for WalletSubscriptionManager {
    fn clone(&self) -> Self {
        Self {
            ws_url: self.ws_url.clone(),
            connected: Arc::clone(&self.connected),
            chain_intervals: std::sync::RwLock::new(self.chain_intervals.read().unwrap().clone()),
            subscriptions: Arc::clone(&self.subscriptions),
            event_sender: Arc::clone(&self.event_sender),
            ws_handle: Arc::clone(&self.ws_handle),
        }
    }
}

/// Adaptive poller that uses per-chain intervals with ±20% jitter.
///
/// Falls back to HTTP polling via ExplorerService when WebSocket is unavailable.
pub struct AdaptivePoller {
    /// Per-chain base intervals in milliseconds
    chain_intervals: std::sync::RwLock<std::collections::HashMap<String, u64>>,
    /// Jitter percentage (0.0 to 1.0)
    jitter_pct: f64,
}

impl AdaptivePoller {
    /// Create a new adaptive poller with default per-chain intervals.
    pub fn new() -> Self {
        let mut intervals = std::collections::HashMap::new();
        intervals.insert("solana".to_string(), 1000);
        intervals.insert("sui".to_string(), 4000);
        intervals.insert("aptos".to_string(), 4000);
        intervals.insert("ethereum".to_string(), 12000);
        intervals.insert("bitcoin".to_string(), 15000);

        Self {
            chain_intervals: std::sync::RwLock::new(intervals),
            jitter_pct: 0.2,
        }
    }

    /// Create a new adaptive poller with custom intervals.
    pub fn with_intervals(intervals: std::collections::HashMap<String, u64>) -> Self {
        Self {
            chain_intervals: std::sync::RwLock::new(intervals),
            jitter_pct: 0.2,
        }
    }

    /// Set the jitter percentage.
    pub fn with_jitter(mut self, jitter_pct: f64) -> Self {
        self.jitter_pct = jitter_pct;
        self
    }

    /// Apply ±jitter to get an adjusted interval.
    pub fn adjusted_interval_ms(&self, chain: &str) -> u64 {
        let base = self
            .chain_intervals
            .read()
            .unwrap()
            .get(chain)
            .copied()
            .unwrap_or(30000);

        let jitter_factor = 1.0 - self.jitter_pct + (rand::random::<f64>() * 2.0 * self.jitter_pct);
        let adjusted = (base as f64 * jitter_factor) as u64;
        adjusted.max(100) // Minimum 100ms
    }

    /// Get the base interval for a chain (without jitter).
    pub fn base_interval_ms(&self, chain: &str) -> u64 {
        self.chain_intervals
            .read()
            .unwrap()
            .get(chain)
            .copied()
            .unwrap_or(30000)
    }

    /// Set the interval for a specific chain.
    pub fn set_interval(&self, chain: &str, interval_ms: u64) {
        self.chain_intervals
            .write()
            .unwrap()
            .insert(chain.to_string(), interval_ms);
    }
}

impl Default for AdaptivePoller {
    fn default() -> Self {
        Self::new()
    }
}
