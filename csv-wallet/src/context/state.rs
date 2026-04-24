//! Application state definition.

use crate::context::types::*;
use crate::wallet_core::WalletData;
use csv_adapter_core::Chain;

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
