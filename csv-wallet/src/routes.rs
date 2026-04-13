//! Application routes.

use dioxus::prelude::*;
use dioxus_router::*;

use crate::{Layout, AuthLayout};
use crate::pages::*;

#[derive(Routable, PartialEq, Clone, Debug)]
pub enum Route {
    // Auth layout: wallet setup pages
    #[layout(AuthLayout)]
        #[route("/")]
        Welcome {},
        #[route("/create")]
        CreateWallet {},
        #[route("/import")]
        ImportWallet {},

    // Main layout: all functional pages
    #[layout(Layout)]
        // Overview
        #[route("/dashboard")]
        Dashboard {},

        // Rights
        #[route("/rights")]
        Rights {},
        #[route("/rights/create")]
        CreateRight {},
        #[route("/rights/:id")]
        ShowRight { id: String },
        #[route("/rights/transfer")]
        TransferRight {},
        #[route("/rights/consume")]
        ConsumeRight {},

        // Proofs
        #[route("/proofs")]
        Proofs {},
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

        // Contracts
        #[route("/contracts")]
        Contracts {},
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
        ConsumeSeal {},
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
        #[route("/validate/proof")]
        ValidateProof {},
        #[route("/validate/seal")]
        ValidateSeal {},
        #[route("/validate/commitment-chain")]
        ValidateCommitmentChain {},

        // Wallet
        #[route("/wallet/generate")]
        GenerateWallet {},
        #[route("/wallet/import")]
        ImportWalletPage {},
        #[route("/wallet/export")]
        ExportWallet {},
        #[route("/wallet/list")]
        ListWallets {},

        // Settings
        #[route("/settings")]
        Settings {},
}
