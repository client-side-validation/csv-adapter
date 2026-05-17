//! Blockchain service for wallet operations.
//!
//! Provides blockchain operations by delegating to csv-sdk.
//! Supports both native and browser wallet contexts.

use csv_sdk::{client::NetworkType, CsvClient};
use csv_core::{ChainId, SanadId};

/// Blockchain error type.
#[derive(Debug, Clone)]
pub struct BlockchainError {
    pub message: String,
    pub chain: Option<ChainId>,
    pub code: Option<u32>,
}

impl std::fmt::Display for BlockchainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlockchainError: {}", self.message)
    }
}

impl std::error::Error for BlockchainError {}

impl From<csv_sdk::CsvError> for BlockchainError {
    fn from(err: csv_sdk::CsvError) -> Self {
        Self {
            message: err.to_string(),
            chain: None,
            code: None,
        }
    }
}

/// Wallet type enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalletType {
    MetaMask,
    Phantom,
    Petra,
    Leather,
    Native,
    Custom,
    SuiWallet,
    AptosWallet,
    SolanaWallet,
}

/// Native wallet.
#[derive(Debug, Clone)]
pub struct NativeWallet {
    pub address: String,
}

impl NativeWallet {
    /// Create a new native wallet.
    pub fn new(address: String) -> Self {
        Self { address }
    }

    /// Get the wallet address.
    pub fn address(&self) -> &str {
        &self.address
    }
}

/// Browser wallet.
#[derive(Debug, Clone, PartialEq)]
pub struct BrowserWallet {
    pub address: String,
    pub chain: Option<ChainId>,
    pub wallet_type: WalletType,
}

/// Contract type enum.
#[derive(Debug, Clone, Copy)]
pub enum ContractType {
    Registry,
    Bridge,
    Lock,
}

/// Contract deployment info.
#[derive(Debug, Clone)]
pub struct ContractDeployment {
    pub address: String,
    pub tx_hash: String,
    pub chain: Option<ChainId>,
    pub contract_address: String,
    pub contract_type: ContractType,
    pub deployed_at: u64,
}

/// Blockchain service using csv-sdk.
pub struct BlockchainService {
    client: CsvClient,
}

impl std::fmt::Debug for BlockchainService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockchainService")
            .field("client", &"<CsvClient>")
            .finish()
    }
}

impl BlockchainService {
    /// Create a new blockchain service.
    pub fn new(_config: BlockchainConfig) -> Self {
        // Initialize csv-sdk client with default configuration
        let client = CsvClient::builder()
            .with_store_backend(csv_sdk::builder::StoreBackend::InMemory)
            .build()
            .expect("Failed to create CSV client");

        Self { client }
    }

}

impl Clone for BlockchainService {
    fn clone(&self) -> Self {
        // Clone the client reference by re-creating a client with the same enabled chains.
        // The runtime is stateless and the client builder is cheap for wallet contexts.
        let client = CsvClient::builder()
            .with_store_backend(csv_sdk::builder::StoreBackend::InMemory)
            .build()
            .expect("Failed to create CSV client");

        Self { client }
    }
}

/// Blockchain configuration.
#[derive(Debug, Clone, Default)]
pub struct BlockchainConfig {
    _private: (),
}

/// Transfer result returned by wallet transfer helpers.
#[derive(Debug, Clone)]
pub struct TransferResult {
    pub transfer_id: String,
    pub source_fee: Option<u64>,
    pub dest_fee: Option<u64>,
    pub lock_tx_hash: Option<String>,
    pub mint_tx_hash: Option<String>,
}

impl BlockchainService {
    /// Execute a cross-chain transfer through the CSV SDK transfer manager.
    pub async fn execute_cross_chain_transfer(
        &self,
        sanad_id: SanadId,
        from_chain: ChainId,
        to_chain: ChainId,
        destination_address: String,
    ) -> Result<TransferResult, BlockchainError> {
        let mut client = self.client.clone();

        client
            .init_adapters(NetworkType::Testnet)
            .await
            .map_err(BlockchainError::from)?;

        let transfer_id = client
            .transfers()
            .cross_chain(sanad_id, to_chain.clone())
            .from_chain(from_chain.clone())
            .to_address(destination_address)
            .execute()
            .await
            .map_err(BlockchainError::from)?;

        let details = client
            .transfers()
            .details(&transfer_id)
            .map_err(BlockchainError::from)?;

        Ok(TransferResult {
            transfer_id,
            source_fee: None,
            dest_fee: None,
            lock_tx_hash: details.lock_tx_hash,
            mint_tx_hash: None,
        })
    }

    /// Execute a local transfer on the same source and destination chain.
    pub async fn transfer_sanad_local(
        &self,
        sanad_id: SanadId,
        chain: ChainId,
        destination_address: String,
    ) -> Result<TransferResult, BlockchainError> {
        self.execute_cross_chain_transfer(sanad_id, chain.clone(), chain, destination_address)
            .await
    }
}

/// Wallet connection utilities stub.
pub mod wallet_connection {
    use super::{ChainId, NativeWallet, WalletType};

    /// Get recommended wallet type for a chain.
    pub fn recommended_wallet(_chain: ChainId) -> WalletType {
        WalletType::MetaMask
    }

    /// Check if MetaMask is installed.
    pub fn is_metamask_installed() -> bool {
        false
    }

    /// Check if Phantom is installed.
    pub fn is_phantom_installed() -> bool {
        false
    }

    /// Connect to MetaMask (stub).
    pub async fn connect_metamask() -> Result<NativeWallet, String> {
        Err("MetaMask not available".to_string())
    }

    /// Create a native wallet from address.
    pub fn native_wallet(address: &str) -> NativeWallet {
        NativeWallet::new(address.to_string())
    }

    /// Check if wallet is installed.
    pub fn is_wallet_installed(_wallet_type: &WalletType) -> bool {
        false
    }
}
