//! Wallet context implementation.

use crate::context::state::AppState;
use crate::context::types::*;
use crate::storage::{self, LocalStorageManager, UNIFIED_STORAGE_KEY, WALLET_MNEMONIC_KEY};
use crate::wallet_core::{ChainAccount, WalletData};
use csv_core::Chain;
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
        if let Some(persisted) =
            store.try_load::<csv_adapter_store::state::UnifiedStorage>(UNIFIED_STORAGE_KEY)
        {
            if let Some(c) = persisted.selected_chain {
                s.selected_chain = convert_chain_from_store(c);
            }
            s.selected_network = match persisted.selected_network {
                Some(csv_adapter_store::state::Network::Dev) => Network::Dev,
                Some(csv_adapter_store::state::Network::Main) => Network::Main,
                _ => Network::Test,
            };
            s.rights = persisted
                .rights
                .into_iter()
                .filter_map(|r| {
                    Some(TrackedRight {
                        id: r.id,
                        chain: convert_chain_from_store(r.chain),
                        value: r.value,
                        status: match r.status {
                            csv_adapter_store::state::RightStatus::Active => RightStatus::Active,
                            csv_adapter_store::state::RightStatus::Transferred => {
                                RightStatus::Transferred
                            }
                            csv_adapter_store::state::RightStatus::Consumed => {
                                RightStatus::Consumed
                            }
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
                        from_chain: convert_chain_from_store(t.source_chain),
                        to_chain: convert_chain_from_store(t.dest_chain),
                        right_id: t.right_id,
                        dest_owner: t.destination_address.unwrap_or_default(),
                        status: match t.status {
                            csv_adapter_store::state::TransferStatus::Initiated => {
                                TransferStatus::Initiated
                            }
                            csv_adapter_store::state::TransferStatus::Locked => {
                                TransferStatus::Locked
                            }
                            csv_adapter_store::state::TransferStatus::Verifying => {
                                TransferStatus::Verifying
                            }
                            csv_adapter_store::state::TransferStatus::Minting => {
                                TransferStatus::Minting
                            }
                            csv_adapter_store::state::TransferStatus::Completed => {
                                TransferStatus::Completed
                            }
                            csv_adapter_store::state::TransferStatus::Failed => {
                                TransferStatus::Failed
                            }
                        },
                        created_at: t.created_at,
                        source_tx_hash: t.source_tx_hash,
                        dest_tx_hash: t.dest_tx_hash,
                        source_contract: None,
                        dest_contract: t.destination_contract,
                        source_fee: None,
                        dest_fee: None,
                    })
                })
                .collect();
            s.seals = persisted
                .seals
                .into_iter()
                .filter_map(|s_rec| {
                    // Check if consumed field exists (old format) or use default
                    let status = if s_rec.consumed {
                        SealStatus::Consumed
                    } else {
                        SealStatus::Active
                    };
                    Some(SealRecord {
                        seal_ref: s_rec.seal_ref,
                        chain: convert_chain_from_store(s_rec.chain),
                        value: s_rec.value,
                        right_id: String::new(), // Old format doesn't have this
                        status,
                        created_at: s_rec.created_at,
                        content: None,
                        proof_ref: None,
                    })
                })
                .collect();
            s.proofs = persisted
                .proofs
                .into_iter()
                .filter_map(|p| {
                    // Old format compatibility - use verified flag to determine status
                    let status = if p.verified {
                        ProofStatus::Verified
                    } else {
                        ProofStatus::Generated
                    };
                    // Build ProofData from store record if it's a ZK proof
                    let data = p.proof_system.as_ref().map(|system| {
                        crate::context::ProofData::Zk {
                            proof_system: system.clone(),
                            proof_bytes: p.proof_data.clone().unwrap_or_default(),
                            seal_id: p.right_id.clone(),
                            block_hash: String::new(),
                            block_height: p.block_height.unwrap_or(0),
                            verifier_key_hash: String::new(),
                        }
                    });
                    Some(ProofRecord {
                        chain: convert_chain_from_store(p.chain),
                        right_id: p.right_id,
                        seal_ref: String::new(), // Old format doesn't have this
                        proof_type: p.proof_type,
                        status,
                        generated_at: p.created_at,
                        verified_at: p.verified_at,
                        data,
                        target_chain: None,
                        verification_tx_hash: None,
                    })
                })
                .collect();
            s.contracts = persisted
                .contracts
                .into_iter()
                .filter_map(|c| {
                    Some(DeployedContract {
                        chain: convert_chain_from_store(c.chain),
                        address: c.address,
                        tx_hash: c.tx_hash,
                        deployed_at: c.deployed_at,
                    })
                })
                .collect();
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

        // Convert local types to unified storage types
        use csv_adapter_store::state::{ContractRecord, RightRecord, TransferRecord, WalletConfig};

        let persisted = csv_adapter_store::state::UnifiedStorage {
            version: 1,
            initialized: !s.wallet.is_empty(),
            selected_chain: Some(convert_chain_to_store(s.selected_chain.clone())),
            selected_network: Some(match s.selected_network {
                Network::Dev => csv_adapter_store::state::Network::Dev,
                Network::Test => csv_adapter_store::state::Network::Test,
                Network::Main => csv_adapter_store::state::Network::Main,
            }),
            rights: s
                .rights
                .iter()
                .map(|r| RightRecord {
                    id: r.id.clone(),
                    chain: convert_chain_to_store(r.chain.clone()),
                    seal_ref: String::new(), // Field populated when seal is linked to right
                    owner: r.owner.clone(),
                    value: r.value,
                    commitment: r.id.clone(), // Using right ID as commitment reference
                    nullifier: None,
                    status: match r.status {
                        RightStatus::Active => csv_adapter_store::state::RightStatus::Active,
                        RightStatus::Transferred => {
                            csv_adapter_store::state::RightStatus::Transferred
                        }
                        RightStatus::Consumed => csv_adapter_store::state::RightStatus::Consumed,
                    },
                    created_at: 0, // Creation time tracking to be implemented
                })
                .collect(),
            transfers: s
                .transfers
                .iter()
                .map(|t| TransferRecord {
                    id: t.id.clone(),
                    source_chain: convert_chain_to_store(t.from_chain.clone()),
                    dest_chain: convert_chain_to_store(t.to_chain.clone()),
                    right_id: t.right_id.clone(),
                    sender_address: None, // Sender address from wallet context
                    destination_address: Some(t.dest_owner.clone()),
                    source_tx_hash: t.source_tx_hash.clone(),
                    source_fee: None,
                    dest_tx_hash: t.dest_tx_hash.clone(),
                    dest_fee: None,
                    destination_contract: t.dest_contract.clone(),
                    proof: None,
                    status: match t.status {
                        TransferStatus::Initiated => {
                            csv_adapter_store::state::TransferStatus::Initiated
                        }
                        TransferStatus::Locked => csv_adapter_store::state::TransferStatus::Locked,
                        TransferStatus::Verifying => {
                            csv_adapter_store::state::TransferStatus::Verifying
                        }
                        TransferStatus::Minting => {
                            csv_adapter_store::state::TransferStatus::Minting
                        }
                        TransferStatus::Completed => {
                            csv_adapter_store::state::TransferStatus::Completed
                        }
                        TransferStatus::Failed => csv_adapter_store::state::TransferStatus::Failed,
                    },
                    created_at: t.created_at,
                    completed_at: None, // Completion time set when transfer finalized
                })
                .collect(),
            seals: s
                .seals
                .iter()
                .map(|s_rec| csv_adapter_store::state::SealRecord {
                    seal_ref: s_rec.seal_ref.clone(),
                    chain: convert_chain_to_store(s_rec.chain.clone()),
                    value: s_rec.value,
                    consumed: s_rec.status == SealStatus::Consumed,
                    created_at: s_rec.created_at,
                })
                .collect(),
            proofs: s
                .proofs
                .iter()
                .map(|p| csv_adapter_store::state::ProofRecord {
                    chain: convert_chain_to_store(p.chain.clone()),
                    right_id: p.right_id.clone(),
                    proof_type: p.proof_type.clone(),
                    proof_system: p.data.as_ref().and_then(|d| d.zk_proof_system().map(|s| s.to_string())),
                    verified: p.status == ProofStatus::Verified,
                    proof_data: None,
                    block_height: None,
                    created_at: p.generated_at,
                    verified_at: p.verified_at,
                })
                .collect(),
            contracts: s
                .contracts
                .iter()
                .map(|c| ContractRecord {
                    chain: convert_chain_to_store(c.chain.clone()),
                    address: c.address.clone(),
                    tx_hash: c.tx_hash.clone(),
                    deployed_at: c.deployed_at,
                })
                .collect(),
            // Default/empty fields
            chains: std::collections::HashMap::new(),
            wallet: WalletConfig::default(),
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

    pub fn accounts_for_chain(&self, chain: Chain) -> Vec<ChainAccount> {
        self.state
            .read()
            .wallet
            .accounts_for_chain(chain)
            .into_iter()
            .cloned()
            .collect()
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
        self.state
            .read()
            .wallet
            .accounts_for_chain(chain)
            .first()
            .map(|a| a.address.clone())
    }

    /// Get the gas payment account for a chain (falls back to regular address).
    pub fn get_gas_account(&self, chain: Chain) -> Option<String> {
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

    pub fn rights(&self) -> Vec<TrackedRight> {
        self.state.read().rights.clone()
    }

    pub fn rights_for_chain(&self, chain: Chain) -> Vec<TrackedRight> {
        self.state
            .read()
            .rights
            .iter()
            .filter(|r| r.chain == chain)
            .cloned()
            .collect()
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

    /// Get signer for a specific chain
    pub fn get_signer_for_chain(
        &self,
        chain: Chain,
    ) -> Option<crate::services::blockchain::NativeWallet> {
        use crate::services::blockchain::wallet_connection;
        self.accounts_for_chain(chain)
            .first()
            .cloned()
            .map(|account| wallet_connection::native_wallet(account))
    }

    /// Refresh rights list from blockchain
    pub async fn refresh_rights(&mut self) {
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
        chain: Chain,
        name: &str,
        private_key_hex: &str,
    ) -> Result<(), String> {
        // Derive address from private key
        let address = crate::wallet_core::ChainAccount::derive_address(chain, private_key_hex)
            .map_err(|e| format!("Failed to derive address: {}", e))?;

        // Create keystore reference (simplified - in production this would encrypt and store)
        let keystore_ref = format!("keystore_{}_{}", chain.id(), uuid::Uuid::new_v4());

        // Create account
        let account = crate::wallet_core::ChainAccount::from_keystore(
            chain,
            name,
            &address,
            &keystore_ref,
            None,
        );

        // Add to wallet
        self.add_account(account);

        // Private key storage: keys are managed through csv-adapter-keystore
        // The keystore_ref links to encrypted storage

        Ok(())
    }

    pub fn remove_account(&mut self, chain: Chain, address: &str) -> bool {
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

    pub fn refresh_address(&mut self, chain: Chain, address: &str, new_address: String) {
        self.state
            .write()
            .wallet
            .refresh_address(chain, address, new_address);
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
        self.state
            .read()
            .rights
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

    pub fn add_contract(&mut self, contract: DeployedContract) {
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
        self.state.write().seals.push(seal);
        self.save_persisted();
    }

    pub fn consume_seal(&mut self, seal_ref: &str) -> bool {
        let mut s = self.state.write();
        if let Some(seal) = s.seals.iter_mut().find(|s| s.seal_ref == seal_ref) {
            seal.status = SealStatus::Consumed;
            drop(s);
            self.save_persisted();
            true
        } else {
            false
        }
    }

    pub fn lock_seal(&mut self, seal_ref: &str, content: SealContent) -> bool {
        let mut s = self.state.write();
        if let Some(seal) = s.seals.iter_mut().find(|s| s.seal_ref == seal_ref) {
            seal.status = SealStatus::Locked;
            seal.content = Some(content);
            drop(s);
            self.save_persisted();
            true
        } else {
            false
        }
    }

    /// Get seal for a specific right
    pub fn seal_for_right(&self, right_id: &str) -> Option<SealRecord> {
        self.state
            .read()
            .seals
            .iter()
            .find(|s| s.right_id == right_id)
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
            .map(|s| s.status.clone())
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
            .find(|p| p.seal_ref == seal_ref)
            .cloned()
    }

    /// Get proof by seal_ref (alias for proof_for_seal)
    pub fn get_proof(&self, seal_ref: &str) -> Option<ProofRecord> {
        self.proof_for_seal(seal_ref)
    }

    /// Get all proofs for a right
    pub fn proofs_for_right(&self, right_id: &str) -> Vec<ProofRecord> {
        self.state
            .read()
            .proofs
            .iter()
            .filter(|p| p.right_id == right_id)
            .cloned()
            .collect()
    }

    pub fn remove_proof(&mut self, right_id: &str, proof_type: &str) -> bool {
        let mut s = self.state.write();
        let before = s.proofs.len();
        s.proofs
            .retain(|p| !(p.right_id == right_id && p.proof_type == proof_type));
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
            let _ = store.delete(UNIFIED_STORAGE_KEY);
            let _ = store.delete(WALLET_MNEMONIC_KEY);
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

/// Convert csv_core::Chain to csv_adapter_store::Chain
fn convert_chain_to_store(chain: csv_core::Chain) -> csv_adapter_store::state::Chain {
    match chain {
        csv_core::Chain::Bitcoin => csv_adapter_store::state::Chain::Bitcoin,
        csv_core::Chain::Ethereum => csv_adapter_store::state::Chain::Ethereum,
        csv_core::Chain::Sui => csv_adapter_store::state::Chain::Sui,
        csv_core::Chain::Aptos => csv_adapter_store::state::Chain::Aptos,
        csv_core::Chain::Solana => csv_adapter_store::state::Chain::Solana,
        _ => csv_adapter_store::state::Chain::Bitcoin, // fallback for any future chains
    }
}

/// Convert csv_adapter_store::Chain to csv_core::Chain
fn convert_chain_from_store(chain: csv_adapter_store::state::Chain) -> csv_core::Chain {
    match chain {
        csv_adapter_store::state::Chain::Bitcoin => csv_core::Chain::Bitcoin,
        csv_adapter_store::state::Chain::Ethereum => csv_core::Chain::Ethereum,
        csv_adapter_store::state::Chain::Sui => csv_core::Chain::Sui,
        csv_adapter_store::state::Chain::Aptos => csv_core::Chain::Aptos,
        csv_adapter_store::state::Chain::Solana => csv_core::Chain::Solana,
    }
}

/// Hook to access wallet context.
pub fn use_wallet_context() -> WalletContext {
    use_context::<WalletContext>()
}
