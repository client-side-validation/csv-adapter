//! Application routes.

use dioxus::prelude::*;

use crate::layout::Layout;
use crate::pages::*;
use crate::routes::cross_chain::TransferDetail;

#[derive(Routable, PartialEq, Clone, Debug)]
pub enum Route {
    #[layout(Layout)]
    // Main entry — Dashboard (shows wallet or create/import modal)
    #[route("/")]
    Dashboard {},

    // Rights
    #[route("/rights")]
    Rights {},
    #[route("/rights/create")]
    CreateRight {},
    #[route("/rights/:id")]
    ShowRight { id: String },
    #[route("/rights/:id/journey")]
    RightJourney { id: String },
    #[route("/rights/transfer")]
    TransferRight {},
    #[route("/rights/consume")]
    ConsumeRight {},

    // Proofs
    #[route("/proofs")]
    Proofs {},
    #[route("/proofs/:id/bundle")]
    ProofBundlePage { id: String },
    #[route("/proofs/generate")]
    GenerateProof {},
    #[route("/proofs/verify")]
    VerifyProof {},
    #[route("/proofs/verify-cross-chain")]
    VerifyCrossChainProof {},

    // Cross-Chain
    #[route("/cross-chain")]
    CrossChain {},
    #[route("/cross-chain/transfer")]
    CrossChainTransfer {},
    #[route("/cross-chain/status")]
    CrossChainStatus {},
    #[route("/cross-chain/retry")]
    CrossChainRetry {},
    #[route("/cross-chain/transfer/:id")]
    TransferDetail { id: String },

    // Contracts
    #[route("/contracts")]
    Contracts {},
    #[route("/contracts/add")]
    AddContract {},
    #[route("/contracts/deploy")]
    DeployContract {},
    #[route("/contracts/status")]
    ContractStatus {},

    // Seals
    #[route("/seals")]
    Seals {},
    #[route("/seals/create")]
    CreateSeal {},
    #[route("/seals/consume")]
    ConsumeSeal { seal_ref: Option<String> },
    #[route("/seals/verify")]
    VerifySeal {},

    // Test
    #[route("/test")]
    Test {},
    #[route("/test/run")]
    RunTests {},
    #[route("/test/scenario")]
    RunScenario {},

    // Validate
    #[route("/validate")]
    Validate {},
    #[route("/validate/consignment")]
    ValidateConsignment {},
    #[route("/validate/offline")]
    OfflineVerify {},
    #[route("/validate/proof")]
    ValidateProof {},
    #[route("/validate/seal")]
    ValidateSeal {},
    #[route("/validate/commitment-chain")]
    ValidateCommitmentChain {},

    // NFT Gallery
    #[route("/nfts")]
    NftGallery {},
    #[route("/nfts/collections")]
    NftCollections {},
    #[route("/nfts/:id")]
    NftDetail { id: String },

    // Wallet management sub-page
    #[route("/wallet")]
    WalletPage {},

    // Account-specific views
    #[route("/account/:id/transactions")]
    AccountTransactions { id: String },

    // Transactions
    #[route("/transactions")]
    Transactions {},
    #[route("/transactions/:id")]
    TransactionDetail { id: String },

    // Settings
    #[route("/settings")]
    Settings {},
}
