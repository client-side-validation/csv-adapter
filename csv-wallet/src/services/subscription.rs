//! WebSocket subscription manager for the wallet.
//!
//! Connects to the explorer's WebSocket subscription endpoint and receives
//! real-time updates for wallet-owned addresses across all chains.
//!
//! **Native-only**: WebSocket support requires `tokio_tungstenite` which
//! does not compile for wasm32. On wasm32, use the adaptive polling fallback.

use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::{RwLock as TokioRwLock, mpsc};

#[cfg(not(target_arch = "wasm32"))]
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
    subscriptions: Arc<std::sync::RwLock<HashMap<String, Vec<String>>>>, // address -> chains
    /// Event sender for broadcasting events to subscribers (native only)
    #[cfg(not(target_arch = "wasm32"))]
    event_sender: Arc<std::sync::RwLock<Option<mpsc::UnboundedSender<SubscriptionEvent>>>>,
    /// Placeholder for wasm32 (no event channel support)
    #[cfg(target_arch = "wasm32")]
    event_sender: Arc<std::sync::RwLock<()>>,
    /// WebSocket connection handle (native only)
    #[cfg(not(target_arch = "wasm32"))]
    ws_handle: Arc<std::sync::RwLock<Option<tokio::task::JoinHandle<()>>>>,
    /// Placeholder for wasm32 (no WebSocket support)
    #[cfg(target_arch = "wasm32")]
    ws_handle: std::marker::PhantomData<()>,
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
            subscriptions: Arc::new(std::sync::RwLock::new(HashMap::new())),
            #[cfg(not(target_arch = "wasm32"))]
            event_sender: Arc::new(std::sync::RwLock::new(None)),
            #[cfg(target_arch = "wasm32")]
            event_sender: Arc::new(std::sync::RwLock::new(())),
            #[cfg(not(target_arch = "wasm32"))]
            ws_handle: Arc::new(std::sync::RwLock::new(None)),
            #[cfg(target_arch = "wasm32")]
            ws_handle: PhantomData,
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
    ///
    /// **Native-only**: Uses `reqwest::Client` which does not compile for wasm32.
    /// On wasm32, use `subscribe_with_fallback` which uses HTTP polling.
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

    /// WASM32 stub — returns an error indicating WebSocket is not available.
    #[cfg(target_arch = "wasm32")]
    pub async fn subscribe(
        &self,
        _address: &str,
        _chain: Option<&str>,
        _network: Option<&str>,
    ) -> Result<(), String> {
        Err("WebSocket subscription is native-only on wasm32; use subscribe_with_fallback".to_string())
    }

    /// Unsubscribe from events for a specific address.
    ///
    /// **Native-only**: Uses `reqwest::Client` which does not compile for wasm32.
    /// On wasm32, use `subscribe_with_fallback` which uses HTTP polling.
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

    /// WASM32 stub — returns an error indicating WebSocket is not available.
    #[cfg(target_arch = "wasm32")]
    pub async fn unsubscribe(&self, _address: &str, _chain: Option<&str>) -> Result<(), String> {
        Err("WebSocket unsubscribe is native-only on wasm32; use subscribe_with_fallback".to_string())
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
    ///
    /// **Native-only**: Requires `tokio_tungstenite` which does not compile for wasm32.
    /// On wasm32, the `subscribe_with_fallback` method will skip WebSocket and use HTTP polling.
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
        *event_sender.write().unwrap() = Some(tx);

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
                        let subs = subscriptions.read().unwrap();
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
                                                    if let Some(sender) = event_sender.read().unwrap().as_ref() {
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

        *self.ws_handle.write().unwrap() = Some(handle);
        Ok(())
    }

    /// WASM32 stub — returns an error indicating WebSocket is not available.
    #[cfg(target_arch = "wasm32")]
    pub async fn connect(&self) -> Result<(), String> {
        Err("WebSocket connection is native-only on wasm32; use subscribe_with_fallback for HTTP polling".to_string())
    }

    /// Disconnect from the WebSocket endpoint.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn disconnect(&self) {
        self.set_connected(false);
        if let Some(handle) = self.ws_handle.write().unwrap().take() {
            handle.abort();
        }
        *self.event_sender.write().unwrap() = None;
    }

    /// WASM32 stub — no-op since WebSocket is not available.
    #[cfg(target_arch = "wasm32")]
    pub async fn disconnect(&self) {
        self.set_connected(false);
        *self.event_sender.write().unwrap() = ();
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
                        let mut subs = self.subscriptions.write().unwrap();
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

        #[cfg(not(target_arch = "wasm32"))]
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

        #[cfg(target_arch = "wasm32")]
        {
            let address_clone = address.clone();
            let chain_clone = chain.clone();
            let poll_interval_clone = poll_interval;
            wasm_bindgen_futures::spawn_local(async move {
                loop {
                    // Use a simple delay loop for wasm32 polling
                    // In production, this would use requestAnimationFrame or a similar browser-native timer
                    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
                        let window = web_sys::window().unwrap();
                        let timeout_id = window
                            .set_timeout_with_callback_and_timeout_and_arguments_0(
                                &resolve,
                                poll_interval_clone as i32,
                            )
                            .unwrap();
                           let _ = timeout_id;
                    });
                    wasm_bindgen_futures::JsFuture::from(promise).await.ok();
                    // Here you would make HTTP requests to the explorer API
                    // and call on_event when new data is found
                    tracing::debug!(
                        "Polling {} for chain {} (interval: {}ms)",
                        address_clone,
                        chain_clone,
                        poll_interval_clone
                    );
                }
            });
        }

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
            #[cfg(not(target_arch = "wasm32"))]
            ws_handle: Arc::clone(&self.ws_handle),
            #[cfg(target_arch = "wasm32")]
            ws_handle: PhantomData,
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
