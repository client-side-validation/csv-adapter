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

/// Seal status - shows lifecycle state
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SealStatus {
    /// Seal created, protecting a Right
    Active,
    /// Right locked, seal holding the value
    Locked,
    /// Seal consumed, value released
    Consumed,
    /// Seal was used in a cross-chain transfer
    Transferred,
}

impl std::fmt::Display for SealStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SealStatus::Active => write!(f, "Active"),
            SealStatus::Locked => write!(f, "Locked"),
            SealStatus::Consumed => write!(f, "Consumed"),
            SealStatus::Transferred => write!(f, "Transferred"),
        }
    }
}

/// The cryptographic content sealed for verification
#[derive(Clone, Debug, PartialEq)]
pub struct SealContent {
    /// Hash of the sealed right data
    pub content_hash: String,
    /// Owner address who created the seal
    pub owner: String,
    /// Block height/number when sealed
    pub block_number: Option<u64>,
    /// Transaction hash that created the seal
    pub lock_tx_hash: Option<String>,
}

/// A seal record - cryptographically protects a Right.
/// In the CSV protocol, a Seal is created when a Right is locked for
/// cross-chain transfer or secure storage.
#[derive(Clone, Debug, PartialEq)]
pub struct SealRecord {
    pub seal_ref: String,
    pub chain: Chain,
    pub value: u64,
    /// Which Right this seal is protecting
    pub right_id: String,
    /// Current status in the seal lifecycle
    pub status: SealStatus,
    /// When the seal was created
    pub created_at: u64,
    /// Cryptographic content of the seal
    pub content: Option<SealContent>,
    /// Reference to any proof generated from this seal
    pub proof_ref: Option<String>,
}

/// Proof status - shows verification state
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProofStatus {
    /// Proof generated but not verified
    Generated,
    /// Proof submitted for verification
    Pending,
    /// Proof verified successfully
    Verified,
    /// Proof verification failed
    Invalid,
}

impl std::fmt::Display for ProofStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofStatus::Generated => write!(f, "Generated"),
            ProofStatus::Pending => write!(f, "Pending"),
            ProofStatus::Verified => write!(f, "Verified"),
            ProofStatus::Invalid => write!(f, "Invalid"),
        }
    }
}

/// Specific proof data based on chain type
#[derive(Clone, Debug, PartialEq)]
pub enum ProofData {
    /// Bitcoin-style Merkle proof
    Merkle {
        /// Merkle root hash
        root: String,
        /// Proof path (sibling hashes)
        path: Vec<String>,
        /// Leaf index
        leaf_index: u64,
    },
    /// Ethereum MPT (Merkle Patricia Trie) proof
    Mpt {
        /// State root
        root: String,
        /// Account proof path
        account_proof: Vec<String>,
        /// Storage proof path
        storage_proof: Vec<String>,
    },
    /// Sui checkpoint proof
    Checkpoint {
        /// Checkpoint sequence number
        sequence: u64,
        /// Checkpoint digest
        digest: String,
        /// Validator signatures
        signatures: Vec<String>,
    },
    /// Aptos ledger proof
    Ledger {
        /// Ledger version
        version: u64,
        /// Proof data
        proof: String,
    },
    /// Solana proof
    Solana {
        /// Slot number
        slot: u64,
        /// Bank hash
        bank_hash: String,
        /// Merkle proof
        merkle_proof: Vec<String>,
    },
    /// Zero-Knowledge proof (Phase 5)
    /// Stores a ZkSealProof for trustless verification without RPC
    Zk {
        /// Proof system used (SP1, Groth16, PlonK, etc.)
        proof_system: String,
        /// Serialized proof bytes (base64 encoded)
        proof_bytes: String,
        /// Public inputs from the proof
        seal_id: String,
        block_hash: String,
        block_height: u64,
        /// Verifier key hash (for identifying the circuit)
        verifier_key_hash: String,
    },
}

impl ProofData {
    /// Check if this is a ZK proof
    pub fn is_zk(&self) -> bool {
        matches!(self, ProofData::Zk { .. })
    }

    /// Get the proof system if this is a ZK proof
    pub fn zk_proof_system(&self) -> Option<&str> {
        match self {
            ProofData::Zk { proof_system, .. } => Some(proof_system),
            _ => None,
        }
    }
}

/// A proof record - cryptographic proof that validates a Seal.
/// In the CSV protocol, a Proof is generated from a Seal and submitted
/// to the destination chain to verify the locked value exists.
#[derive(Clone, Debug, PartialEq)]
pub struct ProofRecord {
    /// Which chain the proof was generated on (source chain)
    pub chain: Chain,
    /// Which Right this proof validates
    pub right_id: String,
    /// Which Seal this proof was generated from
    pub seal_ref: String,
    /// Type of proof (merkle, mpt, checkpoint, etc.)
    pub proof_type: String,
    /// Current status in the proof lifecycle
    pub status: ProofStatus,
    /// When the proof was generated
    pub generated_at: u64,
    /// When the proof was verified (if applicable)
    pub verified_at: Option<u64>,
    /// The actual cryptographic proof data
    pub data: Option<ProofData>,
    /// Target chain for cross-chain verification
    pub target_chain: Option<Chain>,
    /// Verification transaction hash on target chain
    pub verification_tx_hash: Option<String>,
}

impl ProofRecord {
    /// Create a new ZK proof record from a ZkSealProof (Phase 5).
    pub fn from_zk_proof(
        right_id: String,
        seal_ref: String,
        zk_proof: &csv_adapter_core::zk_proof::ZkSealProof,
    ) -> Self {
        use base64::{Engine as _, engine::general_purpose::STANDARD};

        let proof_system = zk_proof.verifier_key.proof_system.to_string();
        let proof_bytes = STANDARD.encode(&zk_proof.proof_bytes);
        let seal_id = hex::encode(&zk_proof.public_inputs.seal_ref.seal_id);
        let block_hash = hex::encode(zk_proof.public_inputs.block_hash.as_bytes());
        let verifier_key_hash = hex::encode(&zk_proof.verifier_key.hash().as_bytes()[..16]);

        Self {
            chain: zk_proof.public_inputs.source_chain,
            right_id,
            seal_ref,
            proof_type: "zk_seal".to_string(),
            status: ProofStatus::Generated,
            generated_at: zk_proof.public_inputs.timestamp,
            verified_at: None,
            data: Some(ProofData::Zk {
                proof_system,
                proof_bytes,
                seal_id,
                block_hash,
                block_height: zk_proof.public_inputs.block_height,
                verifier_key_hash,
            }),
            target_chain: None,
            verification_tx_hash: None,
        }
    }

    /// Check if this proof record is a ZK proof.
    pub fn is_zk_proof(&self) -> bool {
        self.data.as_ref().map_or(false, |d| d.is_zk())
    }

    /// Get the ZK proof data if this is a ZK proof.
    pub fn zk_data(&self) -> Option<&ProofData> {
        self.data.as_ref().filter(|d| d.is_zk())
    }
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
