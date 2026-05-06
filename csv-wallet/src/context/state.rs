//! Application state definition.

use crate::context::types::*;
use crate::wallet_core::WalletData;
use csv_core::Chain;

/// Application state.
#[derive(Clone)]
pub struct AppState {
    pub wallet: WalletData,
    pub selected_chain: Chain,
    pub selected_network: Network,
    pub sanads: Vec<TrackedSanad>,
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

impl Default for AppState {
    fn default() -> Self {
        Self {
            wallet: WalletData::default(),
            selected_chain: Chain::Bitcoin,
            selected_network: Network::Test,
            sanads: Vec::new(),
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
            && self.sanads.len() == other.sanads.len()
            && self.seals.len() == other.seals.len()
            && self.proofs.len() == other.proofs.len()
            && self.transfers.len() == other.transfers.len()
            && self.contracts.len() == other.contracts.len()
            && self.transactions.len() == other.transactions.len()
            && self.nfts.len() == other.nfts.len()
            && self.nft_collections.len() == other.nft_collections.len()
            && self.notification.is_some() == other.notification.is_some()
    }
}
