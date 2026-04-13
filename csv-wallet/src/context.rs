//! Application context and state management.

use dioxus::prelude::*;
use csv_adapter_core::Chain;
use crate::wallet_core::ExtendedWallet;
use crate::routes::Route;
use crate::storage::{self, LocalStorageManager, PersistedState, PersistedRight, PersistedTransfer, PersistedSeal, PersistedProof, PersistedContract};
use std::collections::HashMap;

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
    pub wallet: Option<ExtendedWallet>,
    pub addresses: HashMap<Chain, String>,
    pub initialized: bool,
    pub selected_chain: Chain,
    pub selected_network: Network,
    pub rights: Vec<TrackedRight>,
    pub transfers: Vec<TrackedTransfer>,
    pub contracts: Vec<DeployedContract>,
    pub seals: Vec<SealRecord>,
    pub proofs: Vec<ProofRecord>,
    pub test_results: Vec<TestResult>,
    pub pending_secret: Option<String>,
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
        let addresses = HashMap::new();
        Self {
            wallet: None,
            addresses,
            initialized: false,
            selected_chain: Chain::Bitcoin,
            selected_network: Network::Test,
            rights: Vec::new(),
            transfers: Vec::new(),
            contracts: Vec::new(),
            seals: Vec::new(),
            proofs: Vec::new(),
            test_results: Vec::new(),
            pending_secret: None,
            notification: None,
        }
    }
}

impl PartialEq for AppState {
    fn eq(&self, other: &Self) -> bool {
        self.initialized == other.initialized
            && self.wallet.is_some() == other.wallet.is_some()
            && self.selected_chain == other.selected_chain
            && self.selected_network == other.selected_network
    }
}

/// Wallet context.
#[derive(Clone)]
pub struct WalletContext {
    state: Signal<AppState>,
    store: Option<LocalStorageManager>,
}

impl WalletContext {
    /// Try to create context with localStorage; falls back to memory-only.
    pub fn new(state: Signal<AppState>) -> Self {
        let store = storage::wallet_storage().ok();
        let mut ctx = Self { state, store };
        ctx.load_persisted();
        ctx
    }

    // ===== Persistence =====
    fn load_persisted(&mut self) {
        let Some(store) = &self.store else { return };

        if let Some(persisted) = store.try_load::<PersistedState>(storage::WALLET_STATE_KEY) {
            let mut s = self.state.write();

            // Restore selection
            if let Ok(c) = persisted.selected_chain.parse::<Chain>() {
                s.selected_chain = c;
            }
            s.selected_network = match persisted.selected_network.as_str() {
                "dev" => Network::Dev,
                "main" => Network::Main,
                _ => Network::Test,
            };

            // Restore rights
            s.rights = persisted.rights.into_iter().filter_map(|r| {
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
            }).collect();

            s.transfers = persisted.transfers.into_iter().filter_map(|t| {
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
            }).collect();

            s.seals = persisted.seals.into_iter().filter_map(|s| {
                Some(SealRecord {
                    seal_ref: s.seal_ref,
                    chain: s.chain.parse().ok()?,
                    value: s.value,
                    consumed: s.consumed,
                    created_at: s.created_at,
                })
            }).collect();

            s.proofs = persisted.proofs.into_iter().filter_map(|p| {
                Some(ProofRecord {
                    chain: p.chain.parse().ok()?,
                    right_id: p.right_id,
                    proof_type: p.proof_type,
                    verified: p.verified,
                })
            }).collect();

            s.contracts = persisted.contracts.into_iter().filter_map(|c| {
                Some(DeployedContract {
                    chain: c.chain.parse().ok()?,
                    address: c.address,
                    tx_hash: c.tx_hash,
                    deployed_at: c.deployed_at,
                })
            }).collect();

            s.initialized = persisted.initialized;
        }

        // Try to load wallet mnemonic from separate key (user previously created/imported)
        if let Some(mnemonic) = store.try_load::<String>(storage::WALLET_MNEMONIC_KEY) {
            if let Ok(wallet) = ExtendedWallet::from_mnemonic(&mnemonic) {
                let addresses = wallet.all_addresses();
                let mut s = self.state.write();
                s.wallet = Some(wallet);
                s.addresses = addresses.into_iter().collect();
                // initialized already set from state above
            }
        }
    }

    fn save_persisted(&self) {
        let Some(store) = &self.store else { return };
        let s = self.state.read();

        let persisted = PersistedState {
            initialized: s.initialized,
            selected_chain: s.selected_chain.to_string(),
            selected_network: s.selected_network.to_string(),
            rights: s.rights.iter().map(|r| PersistedRight {
                id: r.id.clone(),
                chain: r.chain.to_string(),
                value: r.value,
                status: r.status.to_string(),
                owner: r.owner.clone(),
            }).collect(),
            transfers: s.transfers.iter().map(|t| PersistedTransfer {
                id: t.id.clone(),
                from_chain: t.from_chain.to_string(),
                to_chain: t.to_chain.to_string(),
                right_id: t.right_id.clone(),
                dest_owner: t.dest_owner.clone(),
                status: t.status.to_string(),
                created_at: t.created_at,
            }).collect(),
            seals: s.seals.iter().map(|s| PersistedSeal {
                seal_ref: s.seal_ref.clone(),
                chain: s.chain.to_string(),
                value: s.value,
                consumed: s.consumed,
                created_at: s.created_at,
            }).collect(),
            proofs: s.proofs.iter().map(|p| PersistedProof {
                chain: p.chain.to_string(),
                right_id: p.right_id.clone(),
                proof_type: p.proof_type.clone(),
                verified: p.verified,
            }).collect(),
            contracts: s.contracts.iter().map(|c| PersistedContract {
                chain: c.chain.to_string(),
                address: c.address.clone(),
                tx_hash: c.tx_hash.clone(),
                deployed_at: c.deployed_at,
            }).collect(),
        };

        let _ = store.save(storage::WALLET_STATE_KEY, &persisted);
    }

    fn save_mnemonic(&self, mnemonic: &str) {
        let Some(store) = &self.store else { return };
        let _ = store.save(storage::WALLET_MNEMONIC_KEY, &mnemonic.to_string());
    }

    fn clear_mnemonic(&self) {
        let Some(store) = &self.store else { return };
        let _ = store.delete(storage::WALLET_MNEMONIC_KEY);
    }

    // ===== Wallet Management =====
    pub fn create_wallet(&mut self) -> String {
        let wallet = ExtendedWallet::generate();
        let addresses = wallet.all_addresses();
        let mnemonic = wallet.mnemonic.clone();
        let mut s = self.state.write();
        s.wallet = Some(wallet);
        s.addresses = addresses.into_iter().collect();
        s.initialized = true;
        s.pending_secret = Some(mnemonic.clone());
        drop(s);
        self.save_persisted();
        self.save_mnemonic(&mnemonic);
        mnemonic
    }

    pub fn import_wallet(&mut self, mnemonic: &str) -> Result<(), String> {
        let wallet = ExtendedWallet::from_mnemonic(mnemonic)?;
        let addresses = wallet.all_addresses();
        let mut s = self.state.write();
        s.wallet = Some(wallet);
        s.addresses = addresses.into_iter().collect();
        s.initialized = true;
        s.pending_secret = None;
        drop(s);
        self.save_persisted();
        self.save_mnemonic(mnemonic);
        Ok(())
    }

    pub fn import_wallet_from_key(&mut self, private_key: &str) -> Result<(), String> {
        let wallet = ExtendedWallet::from_private_key(private_key)?;
        let addresses = wallet.all_addresses();
        let mut s = self.state.write();
        s.wallet = Some(wallet);
        s.addresses = addresses.into_iter().collect();
        s.initialized = true;
        s.pending_secret = None;
        drop(s);
        self.save_persisted();
        Ok(())
    }

    pub fn lock(&mut self) {
        let mut s = self.state.write();
        s.wallet = None;
        s.addresses.clear();
        s.pending_secret = None;
        drop(s);
        self.clear_mnemonic();
        // We keep other state (rights, seals, etc.) persisted
    }

    pub fn clear_pending_secret(&mut self) {
        self.state.write().pending_secret = None;
    }

    // ===== State Selectors =====
    pub fn wallet(&self) -> Option<ExtendedWallet> {
        self.state.read().wallet.clone()
    }

    pub fn addresses(&self) -> Vec<(Chain, String)> {
        let s = self.state.read();
        s.addresses.iter().map(|(k, v)| (*k, v.clone())).collect()
    }

    pub fn address_for_chain(&self, chain: Chain) -> Option<String> {
        self.state.read().addresses.get(&chain).cloned()
    }

    pub fn is_initialized(&self) -> bool {
        self.state.read().initialized
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

    pub fn pending_secret(&self) -> Option<String> {
        self.state.read().pending_secret.clone()
    }

    // ===== Rights =====
    pub fn rights(&self) -> Vec<TrackedRight> {
        self.state.read().rights.clone()
    }

    pub fn rights_for_chain(&self, chain: Chain) -> Vec<TrackedRight> {
        self.state.read().rights.iter().filter(|r| r.chain == chain).cloned().collect()
    }

    pub fn add_right(&mut self, right: TrackedRight) {
        self.state.write().rights.push(right);
        self.save_persisted();
    }

    pub fn get_right(&self, id: &str) -> Option<TrackedRight> {
        self.state.read().rights.iter().find(|r| r.id == id).cloned()
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
        self.state.read().transfers.iter().find(|t| t.id == id).cloned()
    }

    // ===== Contracts =====
    pub fn contracts(&self) -> Vec<DeployedContract> {
        self.state.read().contracts.clone()
    }

    pub fn contracts_for_chain(&self, chain: Chain) -> Vec<DeployedContract> {
        self.state.read().contracts.iter().filter(|c| c.chain == chain).cloned().collect()
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
        self.state.read().seals.iter().filter(|s| s.chain == chain).cloned().collect()
    }

    pub fn add_seal(&mut self, seal: SealRecord) {
        self.state.write().seals.push(seal);
        self.save_persisted();
    }

    pub fn is_seal_consumed(&self, seal_ref: &str) -> bool {
        self.state.read().seals.iter().any(|s| s.seal_ref == seal_ref && s.consumed)
    }

    // ===== Proofs =====
    pub fn proofs(&self) -> Vec<ProofRecord> {
        self.state.read().proofs.clone()
    }

    pub fn add_proof(&mut self, proof: ProofRecord) {
        self.state.write().proofs.push(proof);
        self.save_persisted();
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
}

/// Wallet provider component.
#[component]
pub fn WalletProvider() -> Element {
    let state = use_signal(AppState::default);
    let ctx = use_hook(|| {
        let store = storage::wallet_storage().ok();
        let mut ctx = WalletContext { state, store };
        ctx.load_persisted();
        ctx
    });
    use_context_provider(|| WalletContext { state: ctx.state, store: ctx.store.clone() });

    rsx! {
        Router::<Route> {}
    }
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
    }
}

pub fn chain_icon_emoji(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "\u{1F7E0}",
        Chain::Ethereum => "\u{1F537}",
        Chain::Sui => "\u{1F30A}",
        Chain::Aptos => "\u{1F7E2}",
    }
}

pub fn chain_name(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "Bitcoin",
        Chain::Ethereum => "Ethereum",
        Chain::Sui => "Sui",
        Chain::Aptos => "Aptos",
    }
}
