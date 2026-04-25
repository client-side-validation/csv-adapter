//! Context types - data structures for wallet state management.

use csv_adapter_core::Chain;

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
#[derive(Clone, Debug, PartialEq)]
pub struct TrackedTransfer {
    pub id: String,
    pub from_chain: Chain,
    pub to_chain: Chain,
    pub right_id: String,
    pub dest_owner: String,
    pub status: TransferStatus,
    pub created_at: u64,
    // Transaction details
    pub source_tx_hash: Option<String>,
    pub dest_tx_hash: Option<String>,
    pub source_contract: Option<String>,
    pub dest_contract: Option<String>,
    pub source_fee: Option<String>,
    pub dest_fee: Option<String>,
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
#[derive(Clone, Debug, PartialEq)]
pub struct DeployedContract {
    pub chain: Chain,
    pub address: String,
    pub tx_hash: String,
    pub deployed_at: u64,
}

/// A seal record.
#[derive(Clone, Debug, PartialEq)]
pub struct SealRecord {
    pub seal_ref: String,
    pub chain: Chain,
    pub value: u64,
    pub consumed: bool,
    pub created_at: u64,
}

/// A proof record.
#[derive(Clone, Debug, PartialEq)]
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
    pub id: String,
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

/// Notification kind.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NotificationKind {
    Success,
    Error,
    Warning,
    Info,
}

/// A notification.
#[derive(Clone, Debug)]
pub struct Notification {
    pub kind: NotificationKind,
    pub message: String,
}
