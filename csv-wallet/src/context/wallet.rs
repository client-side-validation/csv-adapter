//! Wallet context implementation.

use crate::context::state::AppState;
use crate::context::types::*;
use crate::storage::{self, LocalStorageManager, PersistedState};
use crate::wallet_core::{ChainAccount, WalletData};
use csv_adapter_core::Chain;
use dioxus::prelude::*;

/// Wallet context.
#[derive(Clone)]
pub struct WalletContext {
    state: Signal<AppState>,
    store: Option<LocalStorageManager>,
    loaded: Signal<bool>,
    selected_contract: Signal<Option<DeployedContract>>,
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
        selected_contract: Signal<Option<DeployedContract>>,
    ) -> Self {
        let store = storage::wallet_storage().ok();
        let mut ctx = Self {
            state,
            store,
            loaded,
            selected_contract,
        };
        ctx.load_persisted();
        ctx.loaded.set(true);
        ctx
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
    pub fn selected_contract(&self) -> Option<DeployedContract> {
        self.selected_contract.read().clone()
    }

    pub fn set_selected_contract(&mut self, contract: Option<DeployedContract>) {
        self.selected_contract.set(contract);
    }

    // ===== Persistence =====
    fn load_persisted(&mut self) {
        let Some(store) = &self.store else { return };
        let mut s = self.state.write();

        // Load app state (rights, seals, etc.)
        if let Some(persisted) = store.try_load::<PersistedState>(storage::WALLET_STATE_KEY) {
            if let Ok(c) = persisted.selected_chain.parse::<Chain>() {
                s.selected_chain = c;
            }
            s.selected_network = match persisted.selected_network.as_str() {
                "dev" => Network::Dev,
                "main" => Network::Main,
                _ => Network::Test,
            };
            s.rights = persisted
                .rights
                .into_iter()
                .filter_map(|r| {
                    Some(TrackedRight {
                        id: r.id,
                        chain: r.chain.parse().ok()?,
                        value: r.value,
                        status: match r.status.as_str() {
                            "Active" => RightStatus::Active,
                            "Transferred" => RightStatus::Transferred,
                            "Consumed" => RightStatus::Consumed,
                            _ => RightStatus::Active,
                        },
                        owner: r.owner,
                    })
                })
                .collect();
            s.transfers = persisted
                .transfers
                .into_iter()
                .filter_map(|t| {
                    Some(TrackedTransfer {
                        id: t.id,
                        from_chain: t.from_chain.parse().ok()?,
                        to_chain: t.to_chain.parse().ok()?,
                        right_id: t.right_id,
                        dest_owner: t.dest_owner,
                        status: match t.status.as_str() {
                            "Initiated" => TransferStatus::Initiated,
                            "Locked" => TransferStatus::Locked,
                            "Verifying" => TransferStatus::Verifying,
                            "Minting" => TransferStatus::Minting,
                            "Completed" => TransferStatus::Completed,
                            "Failed" => TransferStatus::Failed,
                            _ => TransferStatus::Initiated,
                        },
                        created_at: t.created_at,
                    })
                })
                .collect();
            s.seals = persisted
                .seals
                .into_iter()
                .filter_map(|s_rec| {
                    Some(SealRecord {
                        seal_ref: s_rec.seal_ref,
                        chain: s_rec.chain.parse().ok()?,
                        value: s_rec.value,
                        consumed: s_rec.consumed,
                        created_at: s_rec.created_at,
                    })
                })
                .collect();
            s.proofs = persisted
                .proofs
                .into_iter()
                .filter_map(|p| {
                    Some(ProofRecord {
                        chain: p.chain.parse().ok()?,
                        right_id: p.right_id,
                        proof_type: p.proof_type,
                        verified: p.verified,
                    })
                })
                .collect();
            s.contracts = persisted
                .contracts
                .into_iter()
                .filter_map(|c| {
                    Some(DeployedContract {
                        chain: c.chain.parse().ok()?,
                        address: c.address,
                        tx_hash: c.tx_hash,
                        deployed_at: c.deployed_at,
                    })
                })
                .collect();
        }

        // Load wallet data (per-chain accounts)
        if let Some(wallet_json) = store.get_raw(storage::WALLET_MNEMONIC_KEY).ok().flatten() {
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

        let persisted = PersistedState {
            initialized: !s.wallet.is_empty(),
            selected_chain: s.selected_chain.to_string(),
            selected_network: s.selected_network.to_string(),
            rights: s
                .rights
                .iter()
                .map(|r| storage::PersistedRight {
                    id: r.id.clone(),
                    chain: r.chain.to_string(),
                    value: r.value,
                    status: r.status.to_string(),
                    owner: r.owner.clone(),
                })
                .collect(),
            transfers: s
                .transfers
                .iter()
                .map(|t| storage::PersistedTransfer {
                    id: t.id.clone(),
                    from_chain: t.from_chain.to_string(),
                    to_chain: t.to_chain.to_string(),
                    right_id: t.right_id.clone(),
                    dest_owner: t.dest_owner.clone(),
                    status: t.status.to_string(),
                    created_at: t.created_at,
                })
                .collect(),
            seals: s
                .seals
                .iter()
                .map(|s_rec| storage::PersistedSeal {
                    seal_ref: s_rec.seal_ref.clone(),
                    chain: s_rec.chain.to_string(),
                    value: s_rec.value,
                    consumed: s_rec.consumed,
                    created_at: s_rec.created_at,
                })
                .collect(),
            proofs: s
                .proofs
                .iter()
                .map(|p| storage::PersistedProof {
                    chain: p.chain.to_string(),
                    right_id: p.right_id.clone(),
                    proof_type: p.proof_type.clone(),
                    verified: p.verified,
                })
                .collect(),
            contracts: s
                .contracts
                .iter()
                .map(|c| storage::PersistedContract {
                    chain: c.chain.to_string(),
                    address: c.address.clone(),
                    tx_hash: c.tx_hash.clone(),
                    deployed_at: c.deployed_at,
                })
                .collect(),
        };

        if let Err(e) = store.save(storage::WALLET_STATE_KEY, &persisted) {
            web_sys::console::error_1(&format!("Failed to save state: {:?}", e).into());
        }

        // Save wallet data separately
        match s.wallet.to_json() {
            Ok(wallet_json) => {
                if let Err(e) = store.set_raw(storage::WALLET_MNEMONIC_KEY, &wallet_json) {
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

    pub fn accounts_for_chain(&self, chain: Chain) -> Vec<ChainAccount> {
        self.state.read().wallet.accounts_for_chain(chain).into_iter().cloned().collect()
    }

    pub fn selected_chain(&self) -> Chain {
        self.state.read().selected_chain
    }

    pub fn set_selected_chain(&mut self, chain: Chain) {
        self.state.write().selected_chain = chain;
    }

    pub fn selected_network(&self) -> Network {
        self.state.read().selected_network
    }

    pub fn set_selected_network(&mut self, network: Network) {
        self.state.write().selected_network = network;
    }

    /// Get the first address for a chain.
    pub fn address_for_chain(&self, chain: Chain) -> Option<String> {
        self.state.read().wallet.accounts_for_chain(chain).first().map(|a| a.address.clone())
    }

    /// Refresh an account address (for chain swaps).
    pub fn refresh_account_address(&mut self, account_id: &str) -> Result<bool, ()> {
        // Find the account by ID and refresh its address
        if let Some(account) = self.state.write().wallet.accounts.iter_mut().find(|a| a.id == account_id) {
            // Generate a new address for the account
            // For now, this is a placeholder - actual implementation would derive a new address
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

    pub fn rights(&self) -> Vec<TrackedRight> {
        self.state.read().rights.clone()
    }

    pub fn rights_for_chain(&self, chain: Chain) -> Vec<TrackedRight> {
        self.state.read().rights.iter().filter(|r| r.chain == chain).cloned().collect()
    }

    pub fn transfers(&self) -> Vec<TrackedTransfer> {
        self.state.read().transfers.clone()
    }

    pub fn contracts(&self) -> Vec<DeployedContract> {
        self.state.read().contracts.clone()
    }

    pub fn contracts_for_chain(&self, chain: Chain) -> Vec<DeployedContract> {
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

    pub fn get_explorer_url(&self, chain: Chain, tx_hash: &str) -> Option<String> {
        use crate::services::explorer::ExplorerConfig;
        let explorer = ExplorerConfig::for_chain(chain)?;
        Some(explorer.tx_url(tx_hash))
    }

    pub fn get_address_explorer_url(&self, chain: Chain, address: &str) -> Option<String> {
        use crate::services::explorer::ExplorerConfig;
        let explorer = ExplorerConfig::for_chain(chain)?;
        Some(explorer.address_url(address))
    }

    pub fn notification(&self) -> Option<Notification> {
        self.state.read().notification.clone()
    }

    // ===== Setters =====
    pub fn add_account(&mut self, account: ChainAccount) {
        self.state.write().wallet.add_account(account);
        self.save_persisted();
    }

    pub fn remove_account(&mut self, chain: Chain, address: &str) -> bool {
        // Find the account ID by chain and address
        let account_id = self.state.read().wallet.accounts.iter()
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

    pub fn refresh_address(&mut self, chain: Chain, address: &str, new_address: String) {
        self.state.write().wallet.refresh_address(chain, address, new_address);
        self.save_persisted();
    }

    pub fn add_right(&mut self, right: TrackedRight) {
        let mut s = self.state.write();
        if let Some(pos) = s.rights.iter().position(|r| r.id == right.id) {
            s.rights[pos] = right;
        } else {
            s.rights.push(right);
        }
        drop(s);
        self.save_persisted();
    }

    pub fn remove_right(&mut self, id: &str) -> bool {
        let mut s = self.state.write();
        let before = s.rights.len();
        s.rights.retain(|r| r.id != id);
        let removed = s.rights.len() < before;
        drop(s);
        if removed {
            self.save_persisted();
        }
        removed
    }

    pub fn get_right(&self, id: &str) -> Option<TrackedRight> {
        self.state.read().rights.iter().find(|r| r.id == id).cloned()
    }

    pub fn add_transfer(&mut self, transfer: TrackedTransfer) {
        self.state.write().transfers.push(transfer);
        self.save_persisted();
    }

    pub fn get_transfer(&self, id: &str) -> Option<TrackedTransfer> {
        self.state.read().transfers.iter().find(|t| t.id == id).cloned()
    }

    pub fn add_contract(&mut self, contract: DeployedContract) {
        let mut s = self.state.write();
        if let Some(pos) = s.contracts.iter().position(|c| c.address == contract.address) {
            s.contracts[pos] = contract;
        } else {
            s.contracts.push(contract);
        }
        drop(s);
        self.save_persisted();
    }

    pub fn add_seal(&mut self, seal: SealRecord) {
        self.state.write().seals.push(seal);
        self.save_persisted();
    }

    pub fn consume_seal(&mut self, seal_ref: &str) -> bool {
        let mut s = self.state.write();
        if let Some(seal) = s.seals.iter_mut().find(|s| s.seal_ref == seal_ref) {
            seal.consumed = true;
            drop(s);
            self.save_persisted();
            true
        } else {
            false
        }
    }

    pub fn is_seal_consumed(&self, seal_ref: &str) -> bool {
        self.state
            .read()
            .seals
            .iter()
            .find(|s| s.seal_ref == seal_ref)
            .map(|s| s.consumed)
            .unwrap_or(false)
    }

    pub fn add_proof(&mut self, proof: ProofRecord) {
        self.state.write().proofs.push(proof);
        self.save_persisted();
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
        s.rights.clear();
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
            let _ = store.delete(storage::WALLET_STATE_KEY);
            let _ = store.delete(storage::WALLET_MNEMONIC_KEY);
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
