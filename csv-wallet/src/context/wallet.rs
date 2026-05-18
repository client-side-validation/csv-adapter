//! Wallet context implementation.

use crate::context::state::AppState;
use crate::context::types::*;
use crate::services::subscription::{AdaptivePoller, WalletSubscriptionManager};
use crate::storage::{self, LocalStorageManager, UNIFIED_STORAGE_KEY, WALLET_MNEMONIC_KEY};
use crate::wallet_core::{ChainAccount, WalletData};
use dioxus::prelude::*;
use std::sync::{Arc, OnceLock};

#[cfg(target_arch = "wasm32")]
use csv_store::{EncryptedStorageManager, seal_nullifier_storage};

#[cfg(not(target_arch = "wasm32"))]
use crate::core::native_keystore::{NativeKeystore, NativeKeystoreError};

/// Shared seal encryption key, set during wallet unlock.
static SEAL_ENCRYPTION_KEY: OnceLock<[u8; 32]> = OnceLock::new();

/// Set the seal encryption key derived from the wallet password.
/// This should be called after wallet unlock.
#[cfg(target_arch = "wasm32")]
pub fn set_seal_encryption_key(key: [u8; 32]) {
    SEAL_ENCRYPTION_KEY.set(key).ok();
}

/// Get the seal encryption key if available.
#[cfg(target_arch = "wasm32")]
pub fn get_seal_encryption_key() -> Option<[u8; 32]> {
    SEAL_ENCRYPTION_KEY.get().copied()
}

/// Wallet context.
#[derive(Clone)]
pub struct WalletContext {
    state: Signal<AppState>,
    store: Option<LocalStorageManager>,
    loaded: Signal<bool>,
    selected_contract: Signal<Option<ContractRecord>>,
    /// WebSocket subscription manager for real-time updates
    subscription_manager: Arc<WalletSubscriptionManager>,
    /// Adaptive poller for fallback HTTP polling
    adaptive_poller: Arc<AdaptivePoller>,
    /// Optional MPC batcher for Bitcoin commitment aggregation (90% fee savings)
    bitcoin_batcher: Arc<std::sync::Mutex<Option<csv_sdk::csv_bitcoin::mpc_batch::MpcBatcher>>>,
    #[cfg(target_arch = "wasm32")]
    encrypted_seal_store: std::sync::Arc<std::sync::Mutex<Option<EncryptedStorageManager>>>,
    #[cfg(not(target_arch = "wasm32"))]
    native_keystore: std::sync::Arc<std::sync::Mutex<Option<NativeKeystore>>>,
}

impl PartialEq for WalletContext {
    fn eq(&self, _other: &Self) -> bool {
        // Context is compared by reference identity, always equal for memoization
        true
    }
}

impl WalletContext {
    /// Create context with localStorage persistence.
    pub fn new(
        state: Signal<AppState>,
        loaded: Signal<bool>,
        selected_contract: Signal<Option<ContractRecord>>,
    ) -> Self {
        let store = storage::wallet_storage().ok();

        // Initialize WebSocket subscription manager with default explorer URL
        let explorer_url =
            std::env::var("EXPLORER_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
        let subscription_manager = Arc::new(WalletSubscriptionManager::new(explorer_url));

        // Initialize adaptive poller with per-chain intervals
        let adaptive_poller = Arc::new(AdaptivePoller::new());

        // Set chain intervals in subscription manager
        let mut intervals = std::collections::HashMap::new();
        intervals.insert("solana".to_string(), 1000);
        intervals.insert("sui".to_string(), 4000);
        intervals.insert("aptos".to_string(), 4000);
        intervals.insert("ethereum".to_string(), 12000);
        intervals.insert("bitcoin".to_string(), 15000);
        subscription_manager.set_chain_intervals(intervals);

        let mut ctx = Self {
            state,
            store,
            loaded,
            selected_contract,
            subscription_manager,
            adaptive_poller,
            bitcoin_batcher: Arc::new(std::sync::Mutex::new(None)),
            #[cfg(target_arch = "wasm32")]
            encrypted_seal_store: std::sync::Arc::new(std::sync::Mutex::new(None)),
            #[cfg(not(target_arch = "wasm32"))]
            native_keystore: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };
        ctx.load_persisted();
        #[cfg(target_arch = "wasm32")]
        ctx.init_encrypted_seal_store_from_key();
        ctx.loaded.set(true);
        ctx
    }

    /// Initialize the encrypted seal storage with a derived key.
    /// This should be called after wallet unlock with the user's password.
    #[cfg(target_arch = "wasm32")]
    pub fn init_encrypted_seal_store(&self, seal_key: [u8; 32]) {
        let store = seal_nullifier_storage(seal_key);
        *self.encrypted_seal_store.lock().unwrap() = Some(store);
    }

    /// Initialize encrypted seal store from the shared key (set during unlock).
    #[cfg(target_arch = "wasm32")]
    fn init_encrypted_seal_store_from_key(&self) {
        if let Some(key) = get_seal_encryption_key() {
            self.init_encrypted_seal_store(key);
        }
    }

    /// Migrate existing plaintext seals to encrypted storage.
    #[cfg(target_arch = "wasm32")]
    pub async fn migrate_seals_to_encrypted(&self) -> Result<usize, String> {
        let sealed = self.encrypted_seal_store.lock().unwrap();
        let store = sealed
            .as_ref()
            .ok_or("Encrypted seal store not initialized")?;

        let current_seals = self.seals();
        let mut count = 0;

        for seal in current_seals {
            let key = format!("seal:{}", seal.seal_ref);
            if let Err(e) = store.save(&key, &seal).await {
                web_sys::console::error_1(
                    &format!("Failed to encrypt seal {}: {:?}", seal.seal_ref, e).into(),
                );
            } else {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Initialize the native keystore for desktop builds.
    /// This should be called during wallet setup with the user's passphrase.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn init_native_keystore(&self, _passphrase: &str) -> Result<(), NativeKeystoreError> {
        let keystore = NativeKeystore::new()?;
        *self.native_keystore.lock().unwrap() = Some(keystore);
        Ok(())
    }

    /// Get a reference to the native keystore for desktop builds.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_native_keystore(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Option<NativeKeystore>>, String> {
        self.native_keystore
            .lock()
            .map_err(|e| format!("Failed to lock keystore: {}", e))
    }

    /// Check if wallet data has been loaded from storage.
    pub fn is_loaded(&self) -> bool {
        *self.loaded.read()
    }

    /// Force reload wallet data from storage.
    pub fn reload_from_storage(&mut self) {
        web_sys::console::log_1(&"Reloading wallet from storage...".into());
        self.load_persisted();
        web_sys::console::log_1(
            &format!("Wallet reloaded. Accounts: {}", self.accounts().len()).into(),
        );
    }

    // ===== Selected Contract for Transfer =====
    pub fn selected_contract(&self) -> Option<ContractRecord> {
        self.selected_contract.read().clone()
    }

    pub fn set_selected_contract(&mut self, contract: Option<ContractRecord>) {
        self.selected_contract.set(contract);
    }

    // ===== Persistence =====
    fn load_persisted(&mut self) {
        let Some(store) = &self.store else { return };
        let mut s = self.state.write();

        // Load app state (sanads, seals, etc.)
        if let Some(persisted) =
            store.try_load::<csv_store::state::UnifiedStorage>(UNIFIED_STORAGE_KEY)
        {
            // selected_chain is now ChainId (string) - no conversion needed
            if let Some(c) = persisted.selected_chain {
                s.selected_chain = c;
            }
            s.selected_network = match persisted.selected_network {
                Some(csv_store::state::Network::Dev) => Network::Dev,
                Some(csv_store::state::Network::Main) => Network::Main,
                _ => Network::Test,
            };
            // Types are now the same - just clone
            s.sanads = persisted.sanads;
            s.transfers = persisted.transfers;
            s.seals = persisted
                .seals
                .into_iter()
                .map(|s_rec| {
                    // Check if consumed field exists (old format) or use default
                    let status = if s_rec.consumed {
                        SealStatus::Consumed
                    } else {
                        SealStatus::Active
                    };
                    SealRecord {
                        seal_ref: s_rec.seal_ref,
                        chain: s_rec.chain,
                        value: s_rec.value,
                        consumed: false,
                        sanad_id: None,
                        status,
                        created_at: s_rec.created_at,
                        content: None,
                        proof_ref: None,
                    }
                })
                .collect();
            // Proofs are now the same type - just clone
            s.proofs = persisted.proofs;
            // Contracts are now the same type - just clone
            s.contracts = persisted.contracts;
        }

        // Load wallet data (per-chain accounts)
        if let Some(wallet_json) = store.get_raw(WALLET_MNEMONIC_KEY).ok().flatten() {
            let parse_result = WalletData::from_json(&wallet_json).or_else(|_| {
                serde_json::from_str::<String>(&wallet_json)
                    .ok()
                    .and_then(|inner_json| WalletData::from_json(&inner_json).ok())
                    .ok_or_else(|| "Failed to parse wallet JSON".to_string())
            });

            match parse_result {
                Ok(wallet) => {
                    s.wallet = wallet;
                    web_sys::console::log_1(&"Wallet loaded successfully".into());
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Failed to load wallet: {}", e).into());
                }
            }
        }
    }

    fn save_persisted(&self) {
        let Some(store) = &self.store else { return };
        let s = self.state.read();

        let persisted = csv_store::state::UnifiedStorage {
            version: 1,
            initialized: !s.wallet.is_empty(),
            // selected_chain is now ChainId (string) - no conversion needed
            selected_chain: Some(s.selected_chain.clone()),
            selected_network: Some(match s.selected_network {
                Network::Dev => csv_store::state::Network::Dev,
                Network::Test => csv_store::state::Network::Test,
                Network::Main => csv_store::state::Network::Main,
            }),
            // Types are now the same - just clone
            sanads: s.sanads.to_vec(),
            transfers: s.transfers.to_vec(),
            seals: s.seals.to_vec(),
            proofs: s.proofs.to_vec(),
            contracts: s.contracts.to_vec(),
            // Default/empty fields
            chains: std::collections::HashMap::new(),
            wallet: csv_store::state::WalletConfig::default(),
            faucets: std::collections::HashMap::new(),
            transactions: Vec::new(),
            gas_accounts: Vec::new(),
            data_dir: "~/.csv/data".to_string(),
        };

        if let Err(e) = store.save(UNIFIED_STORAGE_KEY, &persisted) {
            web_sys::console::error_1(&format!("Failed to save state: {:?}", e).into());
        }

        // Save wallet data separately
        match s.wallet.to_json() {
            Ok(wallet_json) => {
                if let Err(e) = store.set_raw(WALLET_MNEMONIC_KEY, &wallet_json) {
                    web_sys::console::error_1(&format!("Failed to save wallet: {:?}", e).into());
                }
            }
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to serialize wallet: {}", e).into());
            }
        }
    }

    // ===== Getters =====
    pub fn is_initialized(&self) -> bool {
        !self.state.read().wallet.is_empty()
    }

    pub fn accounts(&self) -> Vec<ChainAccount> {
        self.state.read().wallet.all_accounts()
    }

    pub fn accounts_for_chain(&self, chain: ChainId) -> Vec<ChainAccount> {
        self.state
            .read()
            .wallet
            .accounts_for_chain(chain)
            .into_iter()
            .cloned()
            .collect()
    }

    pub fn selected_chain(&self) -> ChainId {
        self.state.read().selected_chain.clone()
    }

    pub fn set_selected_chain(&mut self, chain: ChainId) {
        self.state.write().selected_chain = chain;
    }

    pub fn selected_network(&self) -> Network {
        self.state.read().selected_network
    }

    pub fn set_selected_network(&mut self, network: Network) {
        self.state.write().selected_network = network;
    }

    /// Get the first address for a chain.
    pub fn address_for_chain(&self, chain: ChainId) -> Option<String> {
        self.state
            .read()
            .wallet
            .accounts_for_chain(chain)
            .first()
            .map(|a| a.address.clone())
    }

    /// Get the gas payment account for a chain (falls back to regular address).
    pub fn get_gas_account(&self, chain: ChainId) -> Option<String> {
        // Prefer a dedicated gas account if set, otherwise use the regular address.
        self.state
            .read()
            .wallet
            .get_gas_account(&chain)
            .clone()
            .or_else(|| self.address_for_chain(chain).clone())
    }

    /// Refresh an account address (for chain swaps).
    pub fn refresh_account_address(&mut self, account_id: &str) -> Result<bool, ()> {
        // Find the account by ID and refresh its address
        if let Some(account) = self
            .state
            .write()
            .wallet
            .accounts
            .iter_mut()
            .find(|a| a.id == account_id)
        {
            // Generate a new address for the account
            // For now, this is a basic implementation - actual implementation would derive a new address
            // based on the chain type and account's keys
            let _new_address = format!("{}_refreshed", &account.address[..8]);
            // account.address = new_address;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Export wallet as JSON string.
    pub fn export_wallet_json(&self) -> Result<String, String> {
        self.state.read().wallet.to_json()
    }

    pub fn sanads(&self) -> Vec<TrackedSanad> {
        self.state.read().sanads.clone()
    }

    pub fn sanads_for_chain(&self, chain: ChainId) -> Vec<TrackedSanad> {
        self.state
            .read()
            .sanads
            .iter()
            .filter(|r| r.chain == chain)
            .cloned()
            .collect()
    }

    pub fn transfers(&self) -> Vec<TrackedTransfer> {
        self.state.read().transfers.clone()
    }

    pub fn contracts(&self) -> Vec<ContractRecord> {
        self.state.read().contracts.clone()
    }

    pub fn contracts_for_chain(&self, chain: ChainId) -> Vec<ContractRecord> {
        self.state
            .read()
            .contracts
            .iter()
            .filter(|c| c.chain == chain)
            .cloned()
            .collect()
    }

    pub fn seals(&self) -> Vec<SealRecord> {
        self.state.read().seals.clone()
    }

    pub fn proofs(&self) -> Vec<ProofRecord> {
        self.state.read().proofs.clone()
    }

    pub fn transactions(&self) -> Vec<TransactionRecord> {
        self.state.read().transactions.clone()
    }

    pub fn transaction_by_id(&self, id: &str) -> Option<TransactionRecord> {
        self.state
            .read()
            .transactions
            .iter()
            .find(|t| t.id == id)
            .cloned()
    }

    pub fn test_results(&self) -> Vec<TestResult> {
        self.state.read().test_results.clone()
    }

    pub fn get_explorer_url(&self, chain: ChainId, tx_hash: &str) -> Option<String> {
        use crate::services::explorer::ExplorerConfig;
        let explorer = ExplorerConfig::for_chain(chain)?;
        Some(explorer.tx_url(tx_hash))
    }

    pub fn get_address_explorer_url(&self, chain: ChainId, address: &str) -> Option<String> {
        use crate::services::explorer::ExplorerConfig;
        let explorer = ExplorerConfig::for_chain(chain)?;
        Some(explorer.address_url(address))
    }

    /// Get signer for a specific chain
    pub fn get_signer_for_chain(
        &self,
        chain: ChainId,
    ) -> Option<crate::services::blockchain::NativeWallet> {
        use crate::services::blockchain::wallet_connection;
        self.accounts_for_chain(chain)
            .first()
            .map(|account| wallet_connection::native_wallet(&account.address))
    }

    /// Refresh sanads list from blockchain
    pub async fn refresh_sanads(&mut self) {
        // This will be implemented properly with chain sync
        // For now just reload persisted data
        self.reload_from_storage();
    }

    pub fn notification(&self) -> Option<Notification> {
        self.state.read().notification.clone()
    }

    // ===== Setters =====
    pub fn add_account(&mut self, account: ChainAccount) {
        self.state.write().wallet.add_account(account);
        self.save_persisted();
    }

    /// Import an account from a private key.
    pub fn import_account_from_key(
        &mut self,
        chain: ChainId,
        name: &str,
        private_key_hex: &str,
        passphrase: &str,
    ) -> Result<(), String> {
        use csv_keys::memory::{Passphrase, SecretKey};

        // Derive address from private key
        let address =
            crate::wallet_core::ChainAccount::derive_address(chain.clone(), private_key_hex)
                .map_err(|e| format!("Failed to derive address: {}", e))?;

        // Parse the private key bytes
        let hex_clean = private_key_hex
            .strip_prefix("0x")
            .unwrap_or(private_key_hex);
        let key_bytes = hex::decode(hex_clean).map_err(|e| format!("Invalid hex: {}", e))?;
        if key_bytes.len() != 32 {
            return Err(format!(
                "Private key must be 32 bytes, got {}",
                key_bytes.len()
            ));
        }

        // Create a SecretKey from the bytes
        let key_arr: [u8; 32] = key_bytes
            .try_into()
            .map_err(|_| "Invalid key length".to_string())?;
        let secret_key = SecretKey::new(key_arr);

        // Encrypt and store in browser keystore
        let keystore_id = uuid::Uuid::new_v4().to_string();
        let chain_name = chain.to_string().to_lowercase();
        let _chain_for_closure = chain.clone();
        let passphrase_obj = Passphrase::new(passphrase);

        #[cfg(target_arch = "wasm32")]
        {
            use csv_keys::browser_keystore::BrowserKeystore;
            let keystore = BrowserKeystore::new()
                .map_err(|e| format!("Failed to create keystore: {}", e))?;
            keystore
                .store_key(&keystore_id, &chain_name, &secret_key, &passphrase_obj)
                .map_err(|e| format!("Failed to store key: {}", e))?;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut ks_guard = self
                .native_keystore
                .lock()
                .map_err(|e| format!("Failed to lock keystore: {}", e))?;
            let keystore = ks_guard
                .as_mut()
                .ok_or("Native keystore not initialized. Call init_native_keystore() first.")?;
            keystore
                .store_key(
                    &keystore_id,
                    &chain_name,
                    Some(name),
                    &secret_key,
                    &passphrase_obj,
                )
                .map_err(|e| e.to_string())?;
        }

        // Create account with keystore reference
        let account = crate::wallet_core::ChainAccount::from_keystore(
            chain,
            name,
            &address,
            &keystore_id,
            None,
        );

        // Add to wallet
        self.add_account(account);

        Ok(())
    }

    /// Retrieve a stored key from the native keystore.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn retrieve_key_from_keystore(
        &self,
        key_id: &str,
        passphrase: &str,
    ) -> Result<String, String> {
        let mut ks_guard = self
            .native_keystore
            .lock()
            .map_err(|e| format!("Failed to lock keystore: {}", e))?;
        let keystore = ks_guard
            .as_mut()
            .ok_or("Native keystore not initialized. Call init_native_keystore() first.")?;
        let secret_key = keystore
            .retrieve_key(key_id, &csv_keys::memory::Passphrase::new(passphrase))
            .map_err(|e| e.to_string())?;
        Ok(hex::encode(secret_key.as_bytes()))
    }

    /// List all stored key IDs in the native keystore.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn list_stored_keys(&self) -> Result<Vec<String>, String> {
        let ks_guard = self
            .native_keystore
            .lock()
            .map_err(|e| format!("Failed to lock keystore: {}", e))?;
        let keystore = ks_guard
            .as_ref()
            .ok_or("Native keystore not initialized. Call init_native_keystore() first.")?;
        Ok(keystore.list_keys())
    }

    pub fn remove_account(&mut self, chain: ChainId, address: &str) -> bool {
        // Find the account ID by chain and address
        let account_id = self
            .state
            .read()
            .wallet
            .accounts
            .iter()
            .find(|a| a.chain == chain && a.address == address)
            .map(|a| a.id.clone());

        if let Some(id) = account_id {
            let removed = self.state.write().wallet.remove_account(&id);
            if removed {
                self.save_persisted();
            }
            removed
        } else {
            false
        }
    }

    pub fn refresh_address(&mut self, chain: ChainId, address: &str, new_address: String) {
        self.state
            .write()
            .wallet
            .refresh_address(chain, address, new_address);
        self.save_persisted();
    }

    pub fn add_sanad(&mut self, sanad: TrackedSanad) {
        let mut s = self.state.write();
        if let Some(pos) = s.sanads.iter().position(|r| r.id == sanad.id) {
            s.sanads[pos] = sanad;
        } else {
            s.sanads.push(sanad);
        }
        drop(s);
        self.save_persisted();
    }

    pub fn remove_sanad(&mut self, id: &str) -> bool {
        let mut s = self.state.write();
        let before = s.sanads.len();
        s.sanads.retain(|r| r.id != id);
        let removed = s.sanads.len() < before;
        drop(s);
        if removed {
            self.save_persisted();
        }
        removed
    }

    pub fn get_sanad(&self, id: &str) -> Option<TrackedSanad> {
        self.state
            .read()
            .sanads
            .iter()
            .find(|r| r.id == id)
            .cloned()
    }

    pub fn add_transfer(&mut self, transfer: TrackedTransfer) {
        self.state.write().transfers.push(transfer);
        self.save_persisted();
    }

    pub fn get_transfer(&self, id: &str) -> Option<TrackedTransfer> {
        self.state
            .read()
            .transfers
            .iter()
            .find(|t| t.id == id)
            .cloned()
    }

    pub fn add_contract(&mut self, contract: ContractRecord) {
        let mut s = self.state.write();
        if let Some(pos) = s
            .contracts
            .iter()
            .position(|c| c.address == contract.address)
        {
            s.contracts[pos] = contract;
        } else {
            s.contracts.push(contract);
        }
        drop(s);
        self.save_persisted();
    }

    pub fn add_seal(&mut self, seal: SealRecord) {
        self.state.write().seals.push(seal.clone());
        self.save_persisted();
        #[cfg(target_arch = "wasm32")]
        self.save_seal_to_encrypted_store(seal);
    }

    pub fn consume_seal(&mut self, seal_ref: &str) -> bool {
        let mut s = self.state.write();
        if let Some(seal) = s.seals.iter_mut().find(|s| s.seal_ref == seal_ref) {
            seal.status = SealStatus::Consumed;
            #[cfg(target_arch = "wasm32")]
            let seal_clone = seal.clone();
            drop(s);
            self.save_persisted();
            #[cfg(target_arch = "wasm32")]
            self.save_seal_to_encrypted_store(seal_clone);
            true
        } else {
            false
        }
    }

    pub fn lock_seal(&mut self, seal_ref: &str, content: SealContent) -> bool {
        let mut s = self.state.write();
        if let Some(seal) = s.seals.iter_mut().find(|s| s.seal_ref == seal_ref) {
            seal.status = SealStatus::Locked;
            seal.content = Some(serde_json::to_string(&content).unwrap_or_default());
            #[cfg(target_arch = "wasm32")]
            let seal_clone = seal.clone();
            drop(s);
            self.save_persisted();
            #[cfg(target_arch = "wasm32")]
            self.save_seal_to_encrypted_store(seal_clone);
            true
        } else {
            false
        }
    }

    /// Get seal for a specific sanad
    pub fn seal_for_sanad(&self, sanad_id: &str) -> Option<SealRecord> {
        self.state
            .read()
            .seals
            .iter()
            .find(|s| s.sanad_id.as_deref() == Some(sanad_id))
            .cloned()
    }

    pub fn remove_seal(&mut self, seal_ref: &str) -> bool {
        let mut s = self.state.write();
        let before = s.seals.len();
        s.seals.retain(|s| s.seal_ref != seal_ref);
        let removed = s.seals.len() < before;
        drop(s);
        if removed {
            self.save_persisted();
            #[cfg(target_arch = "wasm32")]
            self.remove_seal_from_encrypted_store(seal_ref);
        }
        removed
    }

    pub fn is_seal_consumed(&self, seal_ref: &str) -> bool {
        self.state
            .read()
            .seals
            .iter()
            .find(|s| s.seal_ref == seal_ref)
            .map(|s| s.status == SealStatus::Consumed)
            .unwrap_or(false)
    }

    pub fn seal_status(&self, seal_ref: &str) -> Option<SealStatus> {
        self.state
            .read()
            .seals
            .iter()
            .find(|s| s.seal_ref == seal_ref)
            .map(|s| s.status)
    }

    /// Save a seal to the encrypted IndexedDB store (wasm32 only).
    /// This is async but called synchronously here for simplicity - the save is fire-and-forget.
    /// NOTE: Disabled for WASM due to lifetime constraints with spawn_local.
    /// TODO: Implement proper async storage for WASM (AUDIT.md §10.1).
    #[cfg(target_arch = "wasm32")]
    fn save_seal_to_encrypted_store(&self, _seal: SealRecord) {
        // Encrypted store not yet fully implemented for WASM
        // See AUDIT.md §10.1 for production gaps
    }

    /// Remove a seal from the encrypted IndexedDB store (wasm32 only).
    /// NOTE: Disabled for WASM due to lifetime constraints with spawn_local.
    /// TODO: Implement proper async storage for WASM (AUDIT.md §10.1).
    #[cfg(target_arch = "wasm32")]
    fn remove_seal_from_encrypted_store(&self, _seal_ref: &str) {
        // Encrypted store not yet fully implemented for WASM
        // See AUDIT.md §10.1 for production gaps
    }

    pub fn add_proof(&mut self, proof: ProofRecord) {
        self.state.write().proofs.push(proof);
        self.save_persisted();
    }

    /// Link a proof to its seal
    pub fn link_proof_to_seal(&mut self, seal_ref: &str, proof_ref: &str) -> bool {
        let mut s = self.state.write();
        if let Some(seal) = s.seals.iter_mut().find(|s| s.seal_ref == seal_ref) {
            seal.proof_ref = Some(proof_ref.to_string());
            drop(s);
            self.save_persisted();
            true
        } else {
            false
        }
    }

    /// Get proof by reference (seal_ref or generated ID)
    pub fn proof_for_seal(&self, seal_ref: &str) -> Option<ProofRecord> {
        self.state
            .read()
            .proofs
            .iter()
            .find(|p| p.seal_ref.as_deref() == Some(seal_ref))
            .cloned()
    }

    /// Get proof by seal_ref (alias for proof_for_seal)
    pub fn get_proof(&self, seal_ref: &str) -> Option<ProofRecord> {
        self.proof_for_seal(seal_ref)
    }

    /// Get all proofs for a sanad
    pub fn proofs_for_sanad(&self, sanad_id: &str) -> Vec<ProofRecord> {
        self.state
            .read()
            .proofs
            .iter()
            .filter(|p| p.sanad_id == sanad_id)
            .cloned()
            .collect()
    }

    pub fn remove_proof(&mut self, sanad_id: &str, proof_type: &str) -> bool {
        let mut s = self.state.write();
        let before = s.proofs.len();
        s.proofs
            .retain(|p| !(p.sanad_id == sanad_id && p.proof_type == proof_type));
        let removed = s.proofs.len() < before;
        drop(s);
        if removed {
            self.save_persisted();
        }
        removed
    }

    pub fn add_transaction(&mut self, tx: TransactionRecord) {
        self.state.write().transactions.push(tx);
        self.save_persisted();
    }

    pub fn add_test_result(&mut self, result: TestResult) {
        self.state.write().test_results.push(result);
    }

    pub fn set_notification(&mut self, kind: NotificationKind, message: impl Into<String>) {
        self.state.write().notification = Some(Notification {
            kind,
            message: message.into(),
        });
    }

    pub fn clear_notification(&mut self) {
        self.state.write().notification = None;
    }

    /// Import wallet from JSON string.
    pub fn import_wallet_json(&mut self, json: &str) -> Result<(), String> {
        let wallet = WalletData::from_json(json)?;
        self.state.write().wallet = wallet;
        self.save_persisted();
        Ok(())
    }

    /// Lock the wallet (clear all data).
    pub fn lock(&mut self) {
        let mut s = self.state.write();
        s.wallet = WalletData::default();
        s.sanads.clear();
        s.transfers.clear();
        s.contracts.clear();
        s.seals.clear();
        s.proofs.clear();
        s.transactions.clear();
        s.test_results.clear();
        s.nfts.clear();
        s.nft_collections.clear();
        s.notification = None;
        drop(s);
        // Also clear storage
        if let Some(store) = &self.store {
            let _ = store.delete(UNIFIED_STORAGE_KEY);
            let _ = store.delete(WALLET_MNEMONIC_KEY);
        }
    }

    /// Get the WebSocket subscription manager.
    pub fn subscription_manager(&self) -> Arc<WalletSubscriptionManager> {
        Arc::clone(&self.subscription_manager)
    }

    /// Get the adaptive poller.
    pub fn adaptive_poller(&self) -> Arc<AdaptivePoller> {
        Arc::clone(&self.adaptive_poller)
    }

    /// Subscribe to real-time updates for a specific address and chain.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn subscribe_to_address(
        &self,
        address: &str,
        chain: Option<&str>,
    ) -> Result<(), String> {
        self.subscription_manager
            .subscribe(address, chain, None)
            .await
    }

    /// Unsubscribe from updates for a specific address.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn unsubscribe_from_address(
        &self,
        address: &str,
        chain: Option<&str>,
    ) -> Result<(), String> {
        self.subscription_manager.unsubscribe(address, chain).await
    }

    /// Get the adaptive polling interval for a specific chain.
    pub fn get_polling_interval(&self, chain: &str) -> u64 {
        self.adaptive_poller.adjusted_interval_ms(chain)
    }

    // ===== Bitcoin MPC Batcher =====

    /// Enable MPC batcher for Bitcoin commitment aggregation.
    ///
    /// When enabled, Bitcoin seals are queued and published in batches,
    /// achieving ~90% fee savings compared to individual transactions.
    ///
    /// # Example
    /// ```rust,ignore
    /// use csv_sdk::csv_bitcoin::mpc_batch::MpcBatcher;
    ///
    /// // Standard batcher: up to 10 seals, min 2, 5 min timeout
    /// wallet_ctx.enable_bitcoin_batcher(MpcBatcher::default());
    ///
    /// // High-volume batcher: up to 50 seals, min 5, 10 min timeout
    /// wallet_ctx.enable_bitcoin_batcher(MpcBatcher::high_volume());
    /// ```
    pub fn enable_bitcoin_batcher(&self, batcher: csv_sdk::csv_bitcoin::mpc_batch::MpcBatcher) {
        let mut guard = self.bitcoin_batcher.lock().unwrap();
        *guard = Some(batcher);
    }

    /// Disable the Bitcoin MPC batcher.
    ///
    /// Any pending commitments will remain queued but won't be published
    /// until batcher is re-enabled and finalize_batch() is called.
    pub fn disable_bitcoin_batcher(&self) {
        let mut guard = self.bitcoin_batcher.lock().unwrap();
        *guard = None;
    }

    /// Check if Bitcoin batcher is enabled
    pub fn is_bitcoin_batcher_enabled(&self) -> bool {
        let guard = self.bitcoin_batcher.lock().unwrap();
        guard.is_some()
    }

    /// Queue a Bitcoin seal for batched publication.
    ///
    /// # Arguments
    /// * `commitment` - The commitment hash to publish
    /// * `seal` - The Bitcoin seal point (UTXO reference)
    /// * `request_id` - Unique identifier for tracking this seal
    ///
    /// # Returns
    /// - `Ok(true)` if batch is ready to publish (reached batch_size threshold)
    /// - `Ok(false)` if queued but batch not yet ready
    /// - `Err` if batcher not enabled
    pub fn queue_bitcoin_seal(
        &self,
        commitment: csv_core::hash::Hash,
        seal: csv_sdk::csv_bitcoin::types::BitcoinSealPoint,
        request_id: String,
    ) -> Result<bool, String> {
        let guard = self.bitcoin_batcher.lock().unwrap();
        let batcher = guard
            .as_ref()
            .ok_or("Bitcoin MPC batcher not enabled. Call enable_bitcoin_batcher() first.")?;

        Ok(batcher.queue(commitment, seal, request_id))
    }

    /// Check if a Bitcoin batch is ready for publication.
    ///
    /// Returns true if pending commitments >= min_batch_size threshold.
    pub fn is_bitcoin_batch_ready(&self) -> bool {
        let guard = self.bitcoin_batcher.lock().unwrap();
        guard.as_ref().map(|b| b.has_batch_ready()).unwrap_or(false)
    }

    /// Get count of pending Bitcoin seals in the batch queue.
    pub fn pending_bitcoin_batch_count(&self) -> usize {
        let guard = self.bitcoin_batcher.lock().unwrap();
        guard.as_ref().map(|b| b.pending_count()).unwrap_or(0)
    }

    /// Build MPC tree from pending Bitcoin seals.
    ///
    /// This is the first step in finalizing a batch. Returns the MPC tree
    /// and the commitments that were included.
    ///
    /// # Returns
    /// - `Ok((tree, commitments))` if there are pending seals
    /// - `Err` if batcher not enabled or no pending seals
    pub fn build_bitcoin_mpc_tree(
        &self,
    ) -> Result<
        (
            csv_core::commit_mux::CommitMux,
            Vec<csv_sdk::csv_bitcoin::mpc_batch::PendingCommitment>,
        ),
        String,
    > {
        let guard = self.bitcoin_batcher.lock().unwrap();
        let batcher = guard.as_ref().ok_or("Bitcoin MPC batcher not enabled.")?;

        batcher
            .build_mpc_tree()
            .ok_or("No pending Bitcoin seals to batch".to_string())
    }

    /// Generate inclusion proofs for all seals in a batch.
    ///
    /// Call this after building the MPC tree to get proofs for each seal.
    pub fn generate_bitcoin_batch_proofs(
        &self,
        tree: &csv_core::commit_mux::CommitMux,
        commitments: &[csv_sdk::csv_bitcoin::mpc_batch::PendingCommitment],
    ) -> Result<Vec<(String, csv_core::commit_mux::MuxProof)>, String> {
        let guard = self.bitcoin_batcher.lock().unwrap();
        let batcher = guard.as_ref().ok_or("Bitcoin MPC batcher not enabled.")?;

        batcher
            .generate_proofs(tree, commitments)
            .map_err(|e| format!("Failed to generate MPC proofs: {}", e))
    }

    /// Peek at pending Bitcoin seals without consuming them.
    pub fn peek_pending_bitcoin_seals(&self) -> Vec<csv_sdk::csv_bitcoin::mpc_batch::PendingCommitment> {
        let guard = self.bitcoin_batcher.lock().unwrap();
        guard.as_ref().map(|b| b.peek_pending()).unwrap_or_default()
    }

    /// Clear all pending Bitcoin seals from the batch queue.
    ///
    /// # Warning
    /// This discards queued seals without publishing them. Use with caution.
    pub fn clear_bitcoin_batch_queue(&self) {
        let guard = self.bitcoin_batcher.lock().unwrap();
        if let Some(batcher) = guard.as_ref() {
            batcher.clear();
        }
    }
}

/// Wallet provider component.
#[component]
pub fn WalletProvider(children: Element) -> Element {
    let state = use_signal(AppState::default);
    let loaded = use_signal(|| false);
    let selected_contract = use_signal(|| None);

    let ctx = WalletContext::new(state, loaded, selected_contract);

    use_context_provider(|| ctx);

    rsx! {
        {children}
    }
}

/// Hook to access wallet context.
pub fn use_wallet_context() -> WalletContext {
    use_context::<WalletContext>()
}
