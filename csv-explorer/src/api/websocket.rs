//! WebSocket handling for real-time updates
//!
//! Provides WebSocket connections for real-time transfer monitoring
//! and live indexing updates.

use crate::indexing::IndexingManager;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::ws::WebSocket;

/// WebSocket connection manager
pub struct WebSocketManager {
    connections: Arc<RwLock<Vec<WebSocketConnection>>>,
    indexing_manager: Arc<IndexingManager>,
}

/// Individual WebSocket connection
pub struct WebSocketConnection {
    id: String,
    sender: futures::channel::mpsc::UnboundedSender<warp::ws::Message>,
    subscriptions: Vec<Subscription>,
}

/// Subscription type
#[derive(Debug, Clone)]
pub struct Subscription {
    pub subscription_type: String,
    pub filters: serde_json::Value,
}

/// WebSocket message types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WebSocketMessage {
    /// Subscribe to updates
    Subscribe {
        subscription_type: String,
        filters: serde_json::Value,
    },
    /// Unsubscribe from updates
    Unsubscribe { subscription_type: String },
    /// Ping message
    Ping,
    /// Pong message
    Pong,
    /// Data update
    Update {
        subscription_type: String,
        data: serde_json::Value,
    },
    /// Error message
    Error { message: String },
}

impl WebSocketManager {
    /// Create a new WebSocket manager
    pub fn new(indexing_manager: Arc<IndexingManager>) -> Self {
        Self {
            connections: Arc::new(RwLock::new(Vec::new())),
            indexing_manager,
        }
    }

    /// Handle a new WebSocket connection
    pub async fn handle_connection(&self, websocket: WebSocket) {
        use futures::channel::mpsc;

        let (tx, mut rx) = mpsc::unbounded();
        let (mut ws_tx, mut ws_rx) = websocket.split();

        let connection_id = uuid::Uuid::new_v4().to_string();
        let connection = WebSocketConnection {
            id: connection_id.clone(),
            sender: tx,
            subscriptions: Vec::new(),
        };

        // Add connection to manager
        {
            let mut connections = self.connections.write().await;
            connections.push(connection);
        }

        // Handle incoming messages from WebSocket
        let connections = self.connections.clone();
        let indexing_manager = self.indexing_manager.clone();
        let connection_id_for_handler = connection_id.clone();

        tokio::spawn(async move {
            while let Some(message) = ws_rx.next().await {
                if let Ok(message) = message {
                    // Handle different message types
                    if let Ok(text) = message.to_str() {
                        if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(text) {
                            Self::handle_websocket_message(
                                ws_message,
                                &connection_id_for_handler,
                                &connections,
                                &indexing_manager,
                            )
                            .await;
                        }
                    } else if message.is_close() {
                        break;
                    }
                }
            }

            // Remove connection when closed
            let mut connections = connections.write().await;
            connections.retain(|conn| conn.id != connection_id_for_handler);
        });

        // Send initial connection message first
        let welcome_message = WebSocketMessage::Update {
            subscription_type: "connection".to_string(),
            data: serde_json::json!({
                "connection_id": connection_id,
                "message": "Connected to CSV Explorer WebSocket"
            }),
        };

        if let Ok(text) = serde_json::to_string(&welcome_message) {
            let _ = ws_tx.send(warp::ws::Message::text(text)).await;
        }

        // Handle outgoing messages to WebSocket
        tokio::spawn(async move {
            while let Ok(message) = rx.recv().await {
                if ws_tx.send(message).await.is_err() {
                    break;
                }
            }
        });
    }

    /// Handle WebSocket message
    async fn handle_websocket_message(
        message: WebSocketMessage,
        connection_id: &str,
        connections: &Arc<RwLock<Vec<WebSocketConnection>>>,
        indexing_manager: &Arc<IndexingManager>,
    ) {
        match message {
            WebSocketMessage::Subscribe {
                subscription_type,
                filters,
            } => {
                Self::handle_subscription(
                    connection_id,
                    &subscription_type,
                    filters,
                    connections,
                    indexing_manager,
                )
                .await;
            }
            WebSocketMessage::Unsubscribe { subscription_type } => {
                Self::handle_unsubscription(connection_id, &subscription_type, connections).await;
            }
            WebSocketMessage::Ping => {
                Self::send_pong(connection_id, connections).await;
            }
            WebSocketMessage::Pong => {
                // Handle pong response
            }
            WebSocketMessage::Update { .. } => {
                // Clients shouldn't send update messages
                Self::send_error(
                    connection_id,
                    "Invalid message type: Update".to_string(),
                    connections,
                )
                .await;
            }
            WebSocketMessage::Error { .. } => {
                // Clients shouldn't send error messages
            }
        }
    }

    /// Handle subscription request
    async fn handle_subscription(
        connection_id: &str,
        subscription_type: &str,
        filters: serde_json::Value,
        connections: &Arc<RwLock<Vec<WebSocketConnection>>>,
        indexing_manager: &Arc<IndexingManager>,
    ) {
        // Add subscription to connection
        {
            let mut connections_guard = connections.write().await;
            if let Some(connection) = connections_guard.iter_mut().find(|c| c.id == connection_id) {
                let subscription = Subscription {
                    subscription_type: subscription_type.to_string(),
                    filters,
                };

                // Check if already subscribed
                if !connection
                    .subscriptions
                    .iter()
                    .any(|s| s.subscription_type == subscription_type)
                {
                    connection.subscriptions.push(subscription);
                }
            }
        }

        // Send initial data based on subscription type
        match subscription_type {
            "sanads" => {
                if let Ok(owner) = indexing_manager.get_sanads_by_owner("default").await {
                    let update = WebSocketMessage::Update {
                        subscription_type: "sanads".to_string(),
                        data: serde_json::json!(owner),
                    };
                    Self::send_message_to_connection(connection_id, update, connections).await;
                }
            }
            "transfers" => {
                let metrics = indexing_manager.get_metrics().await;
                let update = WebSocketMessage::Update {
                    subscription_type: "transfers".to_string(),
                    data: serde_json::json!({
                        "total_transfers": metrics.transfers_indexed
                    }),
                };
                Self::send_message_to_connection(connection_id, update, connections).await;
            }
            "metrics" => {
                let metrics = indexing_manager.get_metrics().await;
                let update = WebSocketMessage::Update {
                    subscription_type: "metrics".to_string(),
                    data: serde_json::json!(metrics),
                };
                Self::send_message_to_connection(connection_id, update, connections).await;
            }
            _ => {
                Self::send_error(
                    connection_id,
                    format!("Unknown subscription type: {}", subscription_type),
                    connections,
                )
                .await;
            }
        }
    }

    /// Handle unsubscription request
    async fn handle_unsubscription(
        connection_id: &str,
        subscription_type: &str,
        connections: &Arc<RwLock<Vec<WebSocketConnection>>>,
    ) {
        let mut connections_guard = connections.write().await;
        if let Some(connection) = connections_guard.iter_mut().find(|c| c.id == connection_id) {
            connection
                .subscriptions
                .retain(|s| s.subscription_type != subscription_type);
        }
    }

    /// Send pong message
    async fn send_pong(connection_id: &str, connections: &Arc<RwLock<Vec<WebSocketConnection>>>) {
        let pong = WebSocketMessage::Pong;
        Self::send_message_to_connection(connection_id, pong, connections).await;
    }

    /// Send error message
    async fn send_error(
        connection_id: &str,
        error: String,
        connections: &Arc<RwLock<Vec<WebSocketConnection>>>,
    ) {
        let error_msg = WebSocketMessage::Error { message: error };
        Self::send_message_to_connection(connection_id, error_msg, connections).await;
    }

    /// Send message to specific connection
    async fn send_message_to_connection(
        connection_id: &str,
        message: WebSocketMessage,
        connections: &Arc<RwLock<Vec<WebSocketConnection>>>,
    ) {
        let connections_guard = connections.read().await;
        if let Some(connection) = connections_guard.iter().find(|c| c.id == connection_id) {
            if let Ok(text) = serde_json::to_string(&message) {
                let _ = connection
                    .sender
                    .unbounded_send(warp::ws::Message::text(text));
            }
        }
    }

    /// Broadcast message to all connections
    pub async fn broadcast(&self, message: WebSocketMessage) {
        let connections_guard = self.connections.read().await;

        for connection in connections_guard.iter() {
            if let Ok(text) = serde_json::to_string(&message) {
                let _ = connection
                    .sender
                    .unbounded_send(warp::ws::Message::text(text));
            }
        }
    }

    /// Broadcast message to connections subscribed to specific type
    pub async fn broadcast_to_subscribers(
        &self,
        subscription_type: &str,
        message: WebSocketMessage,
    ) {
        let connections_guard = self.connections.read().await;

        for connection in connections_guard.iter() {
            // Check if this connection is subscribed to the given type
            if connection
                .subscriptions
                .iter()
                .any(|s| s.subscription_type == subscription_type)
            {
                if let Ok(text) = serde_json::to_string(&message) {
                    let _ = connection
                        .sender
                        .unbounded_send(warp::ws::Message::text(text));
                }
            }
        }
    }

    /// Get connection count
    pub async fn get_connection_count(&self) -> usize {
        let connections_guard = self.connections.read().await;
        connections_guard.len()
    }
}

/// Handle WebSocket upgrade and connection
pub async fn handle_websocket(websocket: WebSocket, indexing_manager: Arc<IndexingManager>) {
    let ws_manager = WebSocketManager::new(indexing_manager);
    ws_manager.handle_connection(websocket).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_message_serialization() {
        let message = WebSocketMessage::Subscribe {
            subscription_type: "sanads".to_string(),
            filters: serde_json::json!({"owner": "test"}),
        };

        let json = serde_json::to_string(&message).unwrap();
        let parsed: WebSocketMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            WebSocketMessage::Subscribe {
                subscription_type,
                filters,
            } => {
                assert_eq!(subscription_type, "sanads");
                assert_eq!(filters, serde_json::json!({"owner": "test"}));
            }
            _ => panic!("Expected Subscribe message"),
        }
    }
}
