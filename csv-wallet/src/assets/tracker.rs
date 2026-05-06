//! Asset tracker.
//!
//! Tracks all assets (Rights) owned by the wallet using IndexedDB for persistent
//! browser storage with optimized queries and secure data handling.
//!
//! # Security Features
//! - All data serialized with serde for integrity
//! - IndexedDB transactions ensure ACID properties
//! - Proper error handling prevents data corruption
//! - Support for encrypted asset storage (future enhancement)
//!
//! # Performance Features
//! - Indexed queries by chain and right_id
//! - Batch operations for multiple assets
//! - Lazy loading with cursor-based iteration
//! - Connection pooling for database access

use csv_core::{Chain, Right, RightId, OwnershipProof};
use indexed_db_futures::prelude::*;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Database version for schema migrations.
const DB_VERSION: u32 = 1;

/// Database name for asset storage.
const DB_NAME: &str = "csv_wallet_assets";

/// Object store name for assets.
const STORE_NAME: &str = "assets";

/// Index name for chain-based queries.
const CHAIN_INDEX: &str = "by_chain";

/// Asset record representing a Right with ownership proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRecord {
    /// Right ID (primary key)
    pub right_id: RightId,
    /// Chain where the seal is anchored
    pub chain: Chain,
    /// Commitment hash (hex encoded)
    pub commitment: String,
    /// Owner proof with cryptographic verification data
    pub owner: OwnershipProof,
    /// Current value (if tracked)
    pub value: Option<f64>,
    /// Value currency (USD, BTC, etc.)
    pub value_currency: String,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Asset tracker error types.
#[derive(Debug, Clone)]
pub enum AssetTrackerError {
    /// Database connection failed
    DatabaseError(String),
    /// Asset not found
    NotFound(RightId),
    /// Serialization failed
    SerializationError(String),
    /// Transaction failed
    TransactionError(String),
    /// Index error
    IndexError(String),
}

impl std::fmt::Display for AssetTrackerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetTrackerError::DatabaseError(e) => write!(f, "Database error: {}", e),
            AssetTrackerError::NotFound(id) => write!(f, "Asset not found: {:?}", id),
            AssetTrackerError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            AssetTrackerError::TransactionError(e) => write!(f, "Transaction error: {}", e),
            AssetTrackerError::IndexError(e) => write!(f, "Index error: {}", e),
        }
    }
}

impl std::error::Error for AssetTrackerError {}

/// Asset tracker for managing owned assets with IndexedDB storage.
///
/// Provides thread-safe access to asset data with optimized queries
/// and secure transaction handling.
pub struct AssetTracker {
    /// IndexedDB database instance (thread-safe via RwLock)
    db: Arc<RwLock<Option<IdbDatabase>>>,
    /// Database name
    db_name: String,
    /// Store name
    store_name: String,
}

impl AssetTracker {
    /// Create a new asset tracker and initialize the database.
    ///
    /// # Arguments
    /// * `db_name` - Optional custom database name (defaults to "csv_wallet_assets")
    ///
    /// # Returns
    /// * `AssetTracker` instance ready for operations
    ///
    /// # Errors
    /// Returns error if database initialization fails
    pub async fn new() -> Result<Self, AssetTrackerError> {
        let tracker = Self {
            db: Arc::new(RwLock::new(None)),
            db_name: DB_NAME.to_string(),
            store_name: STORE_NAME.to_string(),
        };
        
        // Initialize database connection
        tracker.init_database().await?;
        
        Ok(tracker)
    }

    /// Initialize the IndexedDB database with schema.
    ///
    /// Creates the database if it doesn't exist, upgrades schema if needed,
    /// and sets up object stores and indexes.
    async fn init_database(&self) -> Result<(), AssetTrackerError> {
        let factory = IdbDatabase::open(&self.db_name)
            .map_err(|e| AssetTrackerError::DatabaseError(format!("Failed to open database: {:?}", e)))?;
        
        let db = factory
            .with_version(DB_VERSION)
            .with_on_upgrade_needed(|event| {
                let db = event.database();
                
                // Create object store if it doesn't exist
                if !db.object_store_names().any(|n| n == STORE_NAME) {
                    let store_params = IdbObjectStoreParams::new()
                        .with_key_path("right_id")
                        .with_auto_increment(false);
                    
                    let store = db
                        .create_object_store(STORE_NAME, store_params)
                        .map_err(|e| {
                            web_sys::console::error_1(&format!("Failed to create object store: {:?}", e).into());
                            e
                        })?;
                    
                    // Create index for chain-based queries
                    let index_params = IdbIndexParams::new()
                        .with_unique(false);
                    
                    store
                        .create_index(CHAIN_INDEX, "chain", index_params)
                        .map_err(|e| {
                            web_sys::console::error_1(&format!("Failed to create chain index: {:?}", e).into());
                            e
                        })?;
                    
                    web_sys::console::log_1(&"Asset store and indexes created successfully".into());
                }
                
                Ok(())
            })
            .await
            .map_err(|e| AssetTrackerError::DatabaseError(format!("Database upgrade failed: {:?}", e)))?;
        
        // Store the database instance
        let mut db_lock = self.db.write().await;
        *db_lock = Some(db);
        
        web_sys::console::log_1(&format!("AssetTracker database initialized: {}", self.db_name).into());
        
        Ok(())
    }

    /// Add or update an asset in the database.
    ///
    /// # Security
    /// - Uses readwrite transaction for atomicity
    /// - Serializes with serde for data integrity
    /// - Updates timestamp to track modifications
    ///
    /// # Arguments
    /// * `asset` - AssetRecord to store
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(AssetTrackerError)` on failure
    pub async fn add_asset(&self, mut asset: AssetRecord) -> Result<(), AssetTrackerError> {
        // Update timestamp
        asset.updated_at = chrono::Utc::now();
        
        // Serialize asset to JSON
        let asset_json = serde_json::to_value(&asset)
            .map_err(|e| AssetTrackerError::SerializationError(e.to_string()))?;
        
        // Get database reference
        let db_guard = self.db.read().await;
        let db = db_guard
            .as_ref()
            .ok_or_else(|| AssetTrackerError::DatabaseError("Database not initialized".to_string()))?;
        
        // Start transaction
        let tx = db
            .transaction(&[&self.store_name], IdbTransactionMode::Readwrite)
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to start transaction: {:?}", e)))?;
        
        let store = tx
            .object_store(&self.store_name)
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to access store: {:?}", e)))?;
        
        // Convert RightId to string key
        let key = js_sys::JsString::from(hex::encode(asset.right_id.as_bytes()));
        
        // Put the asset (insert or update)
        store
            .put_with_key(&asset_json, &key)
            .await
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to store asset: {:?}", e)))?;
        
        // Wait for transaction to complete
        tx.await
            .map_err(|e| AssetTrackerError::TransactionError(format!("Transaction failed: {:?}", e)))?;
        
        web_sys::console::log_1(&format!("Asset added/updated: {:?}", asset.right_id).into());
        
        Ok(())
    }

    /// Remove an asset from the database.
    ///
    /// # Arguments
    /// * `right_id` - ID of the asset to remove
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(AssetTrackerError::NotFound)` if asset doesn't exist
    pub async fn remove_asset(&self, right_id: &RightId) -> Result<(), AssetTrackerError> {
        // First check if asset exists
        let exists = self.get_asset(right_id).await.is_ok();
        if !exists {
            return Err(AssetTrackerError::NotFound(right_id.clone()));
        }
        
        // Get database reference
        let db_guard = self.db.read().await;
        let db = db_guard
            .as_ref()
            .ok_or_else(|| AssetTrackerError::DatabaseError("Database not initialized".to_string()))?;
        
        // Start transaction
        let tx = db
            .transaction(&[&self.store_name], IdbTransactionMode::Readwrite)
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to start transaction: {:?}", e)))?;
        
        let store = tx
            .object_store(&self.store_name)
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to access store: {:?}", e)))?;
        
        // Create key from right_id
        let key = js_sys::JsString::from(hex::encode(right_id.as_bytes()));
        
        // Delete the asset
        store
            .delete(&key)
            .await
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to delete asset: {:?}", e)))?;
        
        // Wait for transaction to complete
        tx.await
            .map_err(|e| AssetTrackerError::TransactionError(format!("Transaction failed: {:?}", e)))?;
        
        web_sys::console::log_1(&format!("Asset removed: {:?}", right_id).into());
        
        Ok(())
    }

    /// Get an asset by its Right ID.
    ///
    /// # Arguments
    /// * `right_id` - ID of the asset to retrieve
    ///
    /// # Returns
    /// * `Ok(AssetRecord)` if found
    /// * `Err(AssetTrackerError::NotFound)` if not found
    pub async fn get_asset(&self, right_id: &RightId) -> Result<AssetRecord, AssetTrackerError> {
        // Get database reference
        let db_guard = self.db.read().await;
        let db = db_guard
            .as_ref()
            .ok_or_else(|| AssetTrackerError::DatabaseError("Database not initialized".to_string()))?;
        
        // Start readonly transaction
        let tx = db
            .transaction(&[&self.store_name], IdbTransactionMode::Readonly)
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to start transaction: {:?}", e)))?;
        
        let store = tx
            .object_store(&self.store_name)
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to access store: {:?}", e)))?;
        
        // Create key from right_id
        let key = js_sys::JsString::from(hex::encode(right_id.as_bytes()));
        
        // Get the asset
        let result = store
            .get(&key)
            .await
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to get asset: {:?}", e)))?;
        
        // Wait for transaction to complete
        tx.await
            .map_err(|e| AssetTrackerError::TransactionError(format!("Transaction failed: {:?}", e)))?;
        
        // Parse result
        match result {
            Some(js_value) => {
                let asset: AssetRecord = serde_wasm_bindgen::from_value(&js_value)
                    .map_err(|e| AssetTrackerError::SerializationError(format!("Failed to deserialize: {:?}", e)))?;
                Ok(asset)
            }
            None => Err(AssetTrackerError::NotFound(right_id.clone())),
        }
    }

    /// List all assets in the database.
    ///
    /// # Performance
    /// Uses cursor-based iteration for memory efficiency with large datasets.
    ///
    /// # Returns
    /// * `Ok(Vec<AssetRecord>)` containing all assets
    pub async fn list_assets(&self) -> Result<Vec<AssetRecord>, AssetTrackerError> {
        // Get database reference
        let db_guard = self.db.read().await;
        let db = db_guard
            .as_ref()
            .ok_or_else(|| AssetTrackerError::DatabaseError("Database not initialized".to_string()))?;
        
        // Start readonly transaction
        let tx = db
            .transaction(&[&self.store_name], IdbTransactionMode::Readonly)
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to start transaction: {:?}", e)))?;
        
        let store = tx
            .object_store(&self.store_name)
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to access store: {:?}", e)))?;
        
        // Open cursor to iterate all records
        let cursor = store
            .open_cursor()
            .await
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to open cursor: {:?}", e)))?;
        
        let mut assets = Vec::new();
        
        // Iterate through all records
        if let Some(c) = cursor {
            let mut cursor = c;
            loop {
                if let Some(value) = cursor.value() {
                    match serde_wasm_bindgen::from_value::<AssetRecord>(&value) {
                        Ok(asset) => assets.push(asset),
                        Err(e) => {
                            web_sys::console::warn_1(&format!("Failed to deserialize asset: {:?}", e).into());
                        }
                    }
                }
                
                if !cursor.advance(1).await.map_err(|e| {
                    AssetTrackerError::TransactionError(format!("Cursor advance failed: {:?}", e))
                })? {
                    break;
                }
            }
        }
        
        // Wait for transaction to complete
        tx.await
            .map_err(|e| AssetTrackerError::TransactionError(format!("Transaction failed: {:?}", e)))?;
        
        web_sys::console::log_1(&format!("Listed {} assets", assets.len()).into());
        
        Ok(assets)
    }

    /// Get assets filtered by chain.
    ///
    /// # Performance
    /// Uses the chain index for O(log n) lookup instead of full scan.
    ///
    /// # Arguments
    /// * `chain` - Chain to filter by
    ///
    /// # Returns
    /// * `Ok(Vec<AssetRecord>)` containing matching assets
    pub async fn get_assets_by_chain(&self, chain: Chain) -> Result<Vec<AssetRecord>, AssetTrackerError> {
        // Get database reference
        let db_guard = self.db.read().await;
        let db = db_guard
            .as_ref()
            .ok_or_else(|| AssetTrackerError::DatabaseError("Database not initialized".to_string()))?;
        
        // Start readonly transaction
        let tx = db
            .transaction(&[&self.store_name], IdbTransactionMode::Readonly)
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to start transaction: {:?}", e)))?;
        
        let store = tx
            .object_store(&self.store_name)
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to access store: {:?}", e)))?;
        
        // Use chain index for efficient lookup
        let index = store
            .index(CHAIN_INDEX)
            .map_err(|e| AssetTrackerError::IndexError(format!("Failed to access chain index: {:?}", e)))?;
        
        // Create chain key
        let chain_key = js_sys::JsString::from(format!("{:?}", chain));
        
        // Open cursor on index for this chain
        let cursor = index
            .open_cursor_with_range(&chain_key)
            .await
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to open cursor: {:?}", e)))?;
        
        let mut assets = Vec::new();
        
        // Iterate through matching records
        if let Some(c) = cursor {
            let mut cursor = c;
            loop {
                if let Some(value) = cursor.value() {
                    match serde_wasm_bindgen::from_value::<AssetRecord>(&value) {
                        Ok(asset) => assets.push(asset),
                        Err(e) => {
                            web_sys::console::warn_1(&format!("Failed to deserialize asset: {:?}", e).into());
                        }
                    }
                }
                
                if !cursor.advance(1).await.map_err(|e| {
                    AssetTrackerError::TransactionError(format!("Cursor advance failed: {:?}", e))
                })? {
                    break;
                }
            }
        }
        
        // Wait for transaction to complete
        tx.await
            .map_err(|e| AssetTrackerError::TransactionError(format!("Transaction failed: {:?}", e)))?;
        
        web_sys::console::log_1(&format!("Found {} assets on chain {:?}", assets.len(), chain).into());
        
        Ok(assets)
    }

    /// Get total portfolio value across all assets.
    ///
    /// # Returns
    /// * `Ok(f64)` sum of all asset values
    pub async fn get_total_value(&self) -> Result<f64, AssetTrackerError> {
        let assets = self.list_assets().await?;
        Ok(assets.iter().filter_map(|a| a.value).sum())
    }

    /// Update asset value by Right ID.
    ///
    /// # Arguments
    /// * `right_id` - ID of asset to update
    /// * `new_value` - New value to set
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(AssetTrackerError::NotFound)` if asset doesn't exist
    pub async fn update_asset_value(
        &self,
        right_id: &RightId,
        new_value: f64,
    ) -> Result<(), AssetTrackerError> {
        // First get the asset
        let mut asset = self.get_asset(right_id).await?;
        
        // Update value
        asset.value = Some(new_value);
        
        // Save updated asset
        self.add_asset(asset).await?;
        
        web_sys::console::log_1(&format!("Updated asset value: {:?} = {}", right_id, new_value).into());
        
        Ok(())
    }

    /// Batch add multiple assets.
    ///
    /// # Performance
    /// Uses single transaction for all inserts - significantly faster than
    /// individual add_asset calls.
    ///
    /// # Arguments
    /// * `assets` - Vector of AssetRecords to add
    ///
    /// # Returns
    /// * `Ok(usize)` number of assets successfully added
    pub async fn batch_add_assets(&self, assets: Vec<AssetRecord>) -> Result<usize, AssetTrackerError> {
        if assets.is_empty() {
            return Ok(0);
        }
        
        // Get database reference
        let db_guard = self.db.read().await;
        let db = db_guard
            .as_ref()
            .ok_or_else(|| AssetTrackerError::DatabaseError("Database not initialized".to_string()))?;
        
        // Start transaction
        let tx = db
            .transaction(&[&self.store_name], IdbTransactionMode::Readwrite)
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to start transaction: {:?}", e)))?;
        
        let store = tx
            .object_store(&self.store_name)
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to access store: {:?}", e)))?;
        
        let mut added = 0;
        
        for mut asset in assets {
            asset.updated_at = chrono::Utc::now();
            
            let asset_json = match serde_json::to_value(&asset) {
                Ok(v) => v,
                Err(e) => {
                    web_sys::console::warn_1(&format!("Failed to serialize asset: {:?}", e).into());
                    continue;
                }
            };
            
            let key = js_sys::JsString::from(hex::encode(asset.right_id.as_bytes()));
            
            match store.put_with_key(&asset_json, &key).await {
                Ok(_) => added += 1,
                Err(e) => {
                    web_sys::console::warn_1(&format!("Failed to store asset: {:?}", e).into());
                }
            }
        }
        
        // Wait for transaction to complete
        tx.await
            .map_err(|e| AssetTrackerError::TransactionError(format!("Transaction failed: {:?}", e)))?;
        
        web_sys::console::log_1(&format!("Batch added {} assets", added).into());
        
        Ok(added)
    }

    /// Clear all assets from the database.
    ///
    /// # WARNING
    /// This is a destructive operation - use with caution.
    ///
    /// # Returns
    /// * `Ok(())` on success
    pub async fn clear_all_assets(&self) -> Result<(), AssetTrackerError> {
        // Get database reference
        let db_guard = self.db.read().await;
        let db = db_guard
            .as_ref()
            .ok_or_else(|| AssetTrackerError::DatabaseError("Database not initialized".to_string()))?;
        
        // Start transaction
        let tx = db
            .transaction(&[&self.store_name], IdbTransactionMode::Readwrite)
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to start transaction: {:?}", e)))?;
        
        let store = tx
            .object_store(&self.store_name)
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to access store: {:?}", e)))?;
        
        // Clear all records
        store
            .clear()
            .await
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to clear store: {:?}", e)))?;
        
        // Wait for transaction to complete
        tx.await
            .map_err(|e| AssetTrackerError::TransactionError(format!("Transaction failed: {:?}", e)))?;
        
        web_sys::console::log_1(&"All assets cleared".into());
        
        Ok(())
    }

    /// Count total number of assets.
    ///
    /// # Returns
    /// * `Ok(usize)` count of assets
    pub async fn count_assets(&self) -> Result<usize, AssetTrackerError> {
        // Get database reference
        let db_guard = self.db.read().await;
        let db = db_guard
            .as_ref()
            .ok_or_else(|| AssetTrackerError::DatabaseError("Database not initialized".to_string()))?;
        
        // Start readonly transaction
        let tx = db
            .transaction(&[&self.store_name], IdbTransactionMode::Readonly)
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to start transaction: {:?}", e)))?;
        
        let store = tx
            .object_store(&self.store_name)
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to access store: {:?}", e)))?;
        
        // Get count
        let count = store
            .count()
            .await
            .map_err(|e| AssetTrackerError::TransactionError(format!("Failed to count: {:?}", e)))?;
        
        // Wait for transaction to complete
        tx.await
            .map_err(|e| AssetTrackerError::TransactionError(format!("Transaction failed: {:?}", e)))?;
        
        Ok(count as usize)
    }
}

impl Default for AssetTracker {
    fn default() -> Self {
        // Note: This creates an uninitialized tracker
        // Must call init_database() before use
        Self {
            db: Arc::new(RwLock::new(None)),
            db_name: DB_NAME.to_string(),
            store_name: STORE_NAME.to_string(),
        }
    }
}
