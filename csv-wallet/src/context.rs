//! Application context and state management.

use crate::chains::supported_wallet_chains;
use crate::storage::{self, LocalStorageManager, PersistedState};
use crate::wallet_core::{ChainAccount, WalletData};
use csv_adapter_core::Chain;
use dioxus::prelude::*;

/// Network type.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Network {
    Dev,
    #[default]
    Test,
    Main,
}

impl Network {
    pub fn all() -> [Network; 3] {
        [Network::Dev, Network::Test, Network::Main]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Network::Dev => "dev",
            Network::Test => "test",
            Network::Main => "main",
        }
    }
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A tracked Right.
#[derive(Clone, Debug)]
pub struct TrackedRight {
    pub id: String,
    pub chain: Chain,
    pub value: u64,
    pub status: RightStatus,
    pub owner: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RightStatus {
    Active,
    Transferred,
    Consumed,
}

impl std::fmt::Display for RightStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RightStatus::Active => write!(f, "Active"),
            RightStatus::Transferred => write!(f, "Transferred"),
            RightStatus::Consumed => write!(f, "Consumed"),
        }
    }
}

/// A cross-chain transfer record.
#[derive(Clone, Debug)]
pub struct TrackedTransfer {
    pub id: String,
    pub from_chain: Chain,
    pub to_chain: Chain,
    pub right_id: String,
    pub dest_owner: String,
    pub status: TransferStatus,
    pub created_at: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransferStatus {
    Initiated,
    Locked,
    Verifying,
    Minting,
    Completed,
    Failed,
}

impl std::fmt::Display for TransferStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferStatus::Initiated => write!(f, "Initiated"),
            TransferStatus::Locked => write!(f, "Locked"),
            TransferStatus::Verifying => write!(f, "Verifying"),
            TransferStatus::Minting => write!(f, "Minting"),
            TransferStatus::Completed => write!(f, "Completed"),
            TransferStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// A deployed contract.
#[derive(Clone, Debug)]
pub struct DeployedContract {
    pub chain: Chain,
    pub address: String,
    pub tx_hash: String,
    pub deployed_at: u64,
}

/// A seal record.
#[derive(Clone, Debug)]
pub struct SealRecord {
    pub seal_ref: String,
    pub chain: Chain,
    pub value: u64,
    pub consumed: bool,
    pub created_at: u64,
}

/// A proof record.
#[derive(Clone, Debug)]
pub struct ProofRecord {
    pub chain: Chain,
    pub right_id: String,
    pub proof_type: String,
    pub verified: bool,
}

/// An NFT (Non-Fungible Token) record.
#[derive(Clone, Debug, PartialEq)]
pub struct NftRecord {
    pub id: String,
    pub chain: Chain,
    pub collection_id: Option<String>,
    pub name: String,
    pub symbol: Option<String>,
    pub description: Option<String>,
    pub owner: String,
    pub token_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub image_url: Option<String>,
    pub external_url: Option<String>,
    pub created_at: u64,
    pub status: NftStatus,
}

/// NFT status.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NftStatus {
    Owned,
    Transferred,
    Burned,
    Listed,
}

impl std::fmt::Display for NftStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NftStatus::Owned => write!(f, "Owned"),
            NftStatus::Transferred => write!(f, "Transferred"),
            NftStatus::Burned => write!(f, "Burned"),
            NftStatus::Listed => write!(f, "Listed"),
        }
    }
}

/// A transaction record with explorer links.
#[derive(Clone, Debug, PartialEq)]
pub struct TransactionRecord {
    pub id: String,
    pub chain: Chain,
    pub tx_hash: String,
    pub tx_type: TransactionType,
    pub status: TransactionStatus,
    pub from_address: String,
    pub to_address: Option<String>,
    pub amount: Option<u64>,
    pub fee: Option<u64>,
    pub block_number: Option<u64>,
    pub confirmations: Option<u64>,
    pub created_at: u64,
    pub explorer_url: Option<String>,
}

/// Transaction type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransactionType {
    Transfer,
    ContractDeployment,
    ContractCall,
    RightCreation,
    RightTransfer,
    SealCreation,
    SealConsumption,
    CrossChainLock,
    CrossChainMint,
}

impl std::fmt::Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionType::Transfer => write!(f, "Transfer"),
            TransactionType::ContractDeployment => write!(f, "Contract Deployment"),
            TransactionType::ContractCall => write!(f, "Contract Call"),
            TransactionType::RightCreation => write!(f, "Right Creation"),
            TransactionType::RightTransfer => write!(f, "Right Transfer"),
            TransactionType::SealCreation => write!(f, "Seal Creation"),
            TransactionType::SealConsumption => write!(f, "Seal Consumption"),
            TransactionType::CrossChainLock => write!(f, "Cross-Chain Lock"),
            TransactionType::CrossChainMint => write!(f, "Cross-Chain Mint"),
        }
    }
}

/// Transaction status.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

impl std::fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionStatus::Pending => write!(f, "Pending"),
            TransactionStatus::Confirmed => write!(f, "Confirmed"),
            TransactionStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// NFT collection information.
#[derive(Clone, Debug, PartialEq)]
pub struct NftCollection {
    pub id: String,
    pub chain: Chain,
    pub name: String,
    pub symbol: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub external_url: Option<String>,
    pub total_supply: u64,
    pub owner_count: u64,
    pub floor_price: Option<f64>,
    pub created_at: u64,
}

/// A test result.
#[derive(Clone, Debug)]
pub struct TestResult {
    pub from_chain: Chain,
    pub to_chain: Chain,
    pub status: TestStatus,
    pub message: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TestStatus {
    Pending,
    Running,
    Passed,
    Failed,
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestStatus::Pending => write!(f, "Pending"),
            TestStatus::Running => write!(f, "Running"),
            TestStatus::Passed => write!(f, "Passed"),
            TestStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// Application state.
#[derive(Clone)]
pub struct AppState {
    pub wallet: WalletData,
    pub selected_chain: Chain,
    pub selected_network: Network,
    pub rights: Vec<TrackedRight>,
    pub transfers: Vec<TrackedTransfer>,
    pub contracts: Vec<DeployedContract>,
    pub seals: Vec<SealRecord>,
    pub proofs: Vec<ProofRecord>,
    pub transactions: Vec<TransactionRecord>,
    pub test_results: Vec<TestResult>,
    pub nfts: Vec<NftRecord>,
    pub nft_collections: Vec<NftCollection>,
    pub notification: Option<Notification>,
}

#[derive(Clone, Debug)]
pub struct Notification {
    pub kind: NotificationKind,
    pub message: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NotificationKind {
    Success,
    Error,
    Warning,
    Info,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            wallet: WalletData::default(),
            selected_chain: Chain::Bitcoin,
            selected_network: Network::Test,
            rights: Vec::new(),
            transfers: Vec::new(),
            contracts: Vec::new(),
            seals: Vec::new(),
            proofs: Vec::new(),
            transactions: Vec::new(),
            test_results: Vec::new(),
            nfts: Vec::new(),
            nft_collections: Vec::new(),
            notification: None,
        }
    }
}

impl PartialEq for AppState {
    fn eq(&self, other: &Self) -> bool {
        self.selected_chain == other.selected_chain
            && self.selected_network == other.selected_network
            && self.wallet.total_accounts() == other.wallet.total_accounts()
            && self.nfts.len() == other.nfts.len()
            && self.nft_collections.len() == other.nft_collections.len()
    }
}

/// Wallet context.
#[derive(Clone)]
pub struct WalletContext {
    state: Signal<AppState>,
    store: Option<LocalStorageManager>,
    loaded: Signal<bool>,
}

impl PartialEq for WalletContext {
    fn eq(&self, _other: &Self) -> bool {
        // Context is compared by reference identity, always equal for memoization
        true
    }
}

impl WalletContext {
    /// Create context with localStorage persistence.
    pub fn new(state: Signal<AppState>, loaded: Signal<bool>) -> Self {
        let store = storage::wallet_storage().ok();
        let mut ctx = Self {
            state,
            store,
            loaded,
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

    // ===== Persistence =====
    fn load_persisted(&mut self) {
        let Some(store) = &self.store else { return };

        // Load app state (rights, seals, etc.)
        if let Some(persisted) = store.try_load::<PersistedState>(storage::WALLET_STATE_KEY) {
            let mut s = self.state.write();
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
                .filter_map(|s| {
                    Some(SealRecord {
                        seal_ref: s.seal_ref,
                        chain: s.chain.parse().ok()?,
                        value: s.value,
                        consumed: s.consumed,
                        created_at: s.created_at,
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
            // The wallet JSON might be double-encoded (stored as a JSON string)
            // Try to parse it directly first
            let parse_result = WalletData::from_json(&wallet_json).or_else(|_| {
                // If that fails, try to parse it as a JSON string (double-encoded)
                serde_json::from_str::<String>(&wallet_json)
                    .ok()
                    .and_then(|inner_json| WalletData::from_json(&inner_json).ok())
                    .ok_or_else(|| "Failed to parse wallet JSON".to_string())
            });

            match parse_result {
                Ok(wallet) => {
                    self.state.write().wallet = wallet;
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
                .map(|s| storage::PersistedSeal {
                    seal_ref: s.seal_ref.clone(),
                    chain: s.chain.to_string(),
                    value: s.value,
                    consumed: s.consumed,
                    created_at: s.created_at,
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
        let _ = store.save(storage::WALLET_STATE_KEY, &persisted);

        // Save wallet data as JSON
        if let Ok(json) = s.wallet.to_json() {
            let _ = store.save(storage::WALLET_MNEMONIC_KEY, &json);
        }
    }

    // ===== Account Management =====
    pub fn add_account(&mut self, account: ChainAccount) {
        self.state.write().wallet.add_account(account);
        self.save_persisted();
    }

    pub fn remove_account(&mut self, id: &str) -> bool {
        let removed = self.state.write().wallet.remove_account(id);
        if removed {
            self.save_persisted();
        }
        removed
    }

    pub fn import_wallet_json(&mut self, json: &str) -> Result<(), String> {
        let wallet = WalletData::from_json(json)?;
        self.state.write().wallet = wallet;
        self.save_persisted();
        Ok(())
    }

    pub fn export_wallet_json(&self) -> Result<String, String> {
        self.state.read().wallet.to_json()
    }

    // ===== State Selectors =====
    pub fn wallet(&self) -> WalletData {
        self.state.read().wallet.clone()
    }

    pub fn accounts(&self) -> Vec<ChainAccount> {
        self.state.read().wallet.accounts.clone()
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

    pub fn address_for_chain(&self, chain: Chain) -> Option<String> {
        self.state
            .read()
            .wallet
            .accounts_for_chain(chain)
            .first()
            .map(|a| a.address.clone())
    }

    pub fn all_addresses(&self) -> Vec<(Chain, String)> {
        let wallet = self.state.read().wallet.clone();
        supported_wallet_chains()
            .into_iter()
            .filter_map(|c| {
                wallet
                    .accounts_for_chain(c)
                    .first()
                    .map(|a| (c, a.address.clone()))
            })
            .collect()
    }

    pub fn is_initialized(&self) -> bool {
        !self.state.read().wallet.is_empty()
    }

    pub fn selected_chain(&self) -> Chain {
        self.state.read().selected_chain
    }

    pub fn set_selected_chain(&mut self, chain: Chain) {
        self.state.write().selected_chain = chain;
        self.save_persisted();
    }

    pub fn selected_network(&self) -> Network {
        self.state.read().selected_network
    }

    pub fn set_selected_network(&mut self, network: Network) {
        self.state.write().selected_network = network;
        self.save_persisted();
    }

    // ===== Rights =====
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

    pub fn add_right(&mut self, right: TrackedRight) {
        self.state.write().rights.push(right);
        self.save_persisted();
    }

    pub fn get_right(&self, id: &str) -> Option<TrackedRight> {
        self.state
            .read()
            .rights
            .iter()
            .find(|r| r.id == id)
            .cloned()
    }

    // ===== Transfers =====
    pub fn transfers(&self) -> Vec<TrackedTransfer> {
        self.state.read().transfers.clone()
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

    // ===== Contracts =====
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

    pub fn add_contract(&mut self, contract: DeployedContract) {
        self.state.write().contracts.push(contract);
        self.save_persisted();
    }

    // ===== Seals =====
    pub fn seals(&self) -> Vec<SealRecord> {
        self.state.read().seals.clone()
    }

    pub fn seals_for_chain(&self, chain: Chain) -> Vec<SealRecord> {
        self.state
            .read()
            .seals
            .iter()
            .filter(|s| s.chain == chain)
            .cloned()
            .collect()
    }

    pub fn add_seal(&mut self, seal: SealRecord) {
        self.state.write().seals.push(seal);
        self.save_persisted();
    }

    pub fn is_seal_consumed(&self, seal_ref: &str) -> bool {
        self.state
            .read()
            .seals
            .iter()
            .any(|s| s.seal_ref == seal_ref && s.consumed)
    }

    // ===== Proofs =====
    pub fn proofs(&self) -> Vec<ProofRecord> {
        self.state.read().proofs.clone()
    }

    pub fn add_proof(&mut self, proof: ProofRecord) {
        self.state.write().proofs.push(proof);
        self.save_persisted();
    }

    // ===== Transactions =====
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

    pub fn transactions_for_chain(&self, chain: Chain) -> Vec<TransactionRecord> {
        self.state
            .read()
            .transactions
            .iter()
            .filter(|t| t.chain == chain)
            .cloned()
            .collect()
    }

    pub fn transactions_for_address(&self, address: &str) -> Vec<TransactionRecord> {
        self.state
            .read()
            .transactions
            .iter()
            .filter(|t| t.from_address == address || t.to_address.as_ref() == Some(&address.to_string()))
            .cloned()
            .collect()
    }

    pub fn add_transaction(&mut self, transaction: TransactionRecord) {
        self.state.write().transactions.push(transaction);
        self.save_persisted();
    }

    pub fn update_transaction_status(&mut self, id: &str, status: TransactionStatus) {
        let mut state = self.state.write();
        if let Some(tx) = state.transactions.iter_mut().find(|t| t.id == id) {
            tx.status = status;
        }
        drop(state);
        self.save_persisted();
    }

    /// Get explorer URL for a transaction on a specific chain
    pub fn get_explorer_url(&self, chain: Chain, tx_hash: &str) -> Option<String> {
        match chain {
            Chain::Bitcoin => Some(format!("https://blockstream.info/tx/{}", tx_hash)),
            Chain::Ethereum => Some(format!("https://etherscan.io/tx/{}", tx_hash)),
            Chain::Solana => Some(format!("https://explorer.solana.com/tx/{}", tx_hash)),
            Chain::Sui => Some(format!("https://suivision.xyz/txblock/{}", tx_hash)),
            Chain::Aptos => Some(format!("https://explorer.aptoslabs.com/txn/{}", tx_hash)),
            _ => None,
        }
    }

    /// Get address explorer URL for a specific chain
    pub fn get_address_explorer_url(&self, chain: Chain, address: &str) -> Option<String> {
        match chain {
            Chain::Bitcoin => Some(format!("https://blockstream.info/address/{}", address)),
            Chain::Ethereum => Some(format!("https://etherscan.io/address/{}", address)),
            Chain::Solana => Some(format!("https://explorer.solana.com/address/{}", address)),
            Chain::Sui => Some(format!("https://suivision.xyz/account/{}", address)),
            Chain::Aptos => Some(format!("https://explorer.aptoslabs.com/account/{}", address)),
            _ => None,
        }
    }

    // ===== Test Results =====
    pub fn test_results(&self) -> Vec<TestResult> {
        self.state.read().test_results.clone()
    }

    pub fn add_test_result(&mut self, result: TestResult) {
        self.state.write().test_results.push(result);
    }

    pub fn clear_test_results(&mut self) {
        self.state.write().test_results.clear();
    }

    // ===== Notifications =====
    pub fn notification(&self) -> Option<Notification> {
        self.state.read().notification.clone()
    }

    pub fn set_notification(&mut self, kind: NotificationKind, message: String) {
        self.state.write().notification = Some(Notification { kind, message });
    }

    pub fn clear_notification(&mut self) {
        self.state.write().notification = None;
    }

    // ===== Lock/Clear =====
    pub fn lock(&mut self) {
        self.state.write().wallet = WalletData::default();
        self.save_persisted();
    }
}

/// Wallet provider component.
#[component]
pub fn WalletProvider(children: Element) -> Element {
    let state = use_signal(AppState::default);
    let loaded = use_signal(|| false);
    let ctx = use_hook(|| WalletContext::new(state, loaded));
    use_context_provider(|| ctx.clone());

    rsx! { { children } }
}

/// Hook to access wallet context.
pub fn use_wallet_context() -> WalletContext {
    use_context::<WalletContext>()
}

/// Helper: generate a random hex ID.
pub fn generate_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    format!("0x{}", hex::encode(bytes))
}

/// Helper: truncate address for display.
pub fn truncate_address(addr: &str, chars: usize) -> String {
    if addr.len() <= chars * 2 + 2 {
        addr.to_string()
    } else {
        format!("{}...{}", &addr[..chars + 2], &addr[addr.len() - chars..])
    }
}

// ===== Chain Styling Helpers =====
pub fn chain_badge_class(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-orange-400 bg-orange-500/20 border border-orange-500/30",
        Chain::Ethereum => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-blue-400 bg-blue-500/20 border border-blue-500/30",
        Chain::Sui => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-cyan-400 bg-cyan-500/20 border border-cyan-500/30",
        Chain::Aptos => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-emerald-400 bg-emerald-500/20 border border-emerald-500/30",
        Chain::Solana => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-purple-400 bg-purple-500/20 border border-purple-500/30",
        _ => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-gray-400 bg-gray-500/20 border border-gray-500/30",
    }
}

pub fn chain_icon_emoji(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "\u{1F7E0}",
        Chain::Ethereum => "\u{1F537}",
        Chain::Sui => "\u{1F30A}",
        Chain::Aptos => "\u{1F7E2}",
        Chain::Solana => "\u{2600}",
        _ => "\u{26AA}",
    }
}

pub fn chain_name(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "Bitcoin",
        Chain::Ethereum => "Ethereum",
        Chain::Sui => "Sui",
        Chain::Aptos => "Aptos",
        Chain::Solana => "Solana",
        _ => "Unknown",
    }
}
