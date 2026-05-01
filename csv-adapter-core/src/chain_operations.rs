//! Core Chain Operation Traits
//!
//! This module defines the standardized traits that all chain adapters must implement
//! for production use. These traits provide a unified interface for:
//!
//! - **ChainQuery**: Querying balances, transactions, and chain state
//! - **ChainSigner**: Deriving addresses and signing transactions/messages
//! - **ChainBroadcaster**: Submitting and confirming transactions
//! - **ChainDeployer**: Deploying contracts and programs
//! - **ChainProofProvider**: Building and verifying cryptographic proofs
//! - **ChainRightOps**: Managing rights (create, consume, lock, mint, refund)
//!
//! All adapters must implement these traits to be registered in the production registry.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::hash::Hash;
use crate::proof::{FinalityProof, InclusionProof};
use crate::right::RightId;

/// Result type for chain operations
pub type ChainOpResult<T> = Result<T, ChainOpError>;

/// Errors that can occur during chain operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChainOpError {
    /// Chain is not supported or not configured
    UnsupportedChain(String),
    /// RPC connection failed or returned error
    RpcError(String),
    /// Transaction failed validation
    TransactionError(String),
    /// Signing operation failed
    SigningError(String),
    /// Proof verification failed
    ProofVerificationError(String),
    /// Contract deployment failed
    DeploymentError(String),
    /// Capability is not available (not yet implemented or not supported on this chain)
    CapabilityUnavailable(String),
    /// Feature is not enabled (requires configuration or feature flag)
    FeatureNotEnabled(String),
    /// Invalid input parameters
    InvalidInput(String),
    /// Timeout waiting for operation
    Timeout(String),
    /// Unknown error
    Unknown(String),
}

impl core::fmt::Display for ChainOpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ChainOpError::UnsupportedChain(msg) => write!(f, "Unsupported chain: {}", msg),
            ChainOpError::RpcError(msg) => write!(f, "RPC error: {}", msg),
            ChainOpError::TransactionError(msg) => write!(f, "Transaction error: {}", msg),
            ChainOpError::SigningError(msg) => write!(f, "Signing error: {}", msg),
            ChainOpError::ProofVerificationError(msg) => {
                write!(f, "Proof verification error: {}", msg)
            }
            ChainOpError::DeploymentError(msg) => write!(f, "Deployment error: {}", msg),
            ChainOpError::CapabilityUnavailable(msg) => {
                write!(f, "Capability unavailable: {}", msg)
            }
            ChainOpError::FeatureNotEnabled(msg) => write!(f, "Feature not enabled: {}", msg),
            ChainOpError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            ChainOpError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            ChainOpError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for ChainOpError {}

/// Status of a transaction on chain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionStatus {
    /// Transaction is pending/in mempool
    Pending,
    /// Transaction has been confirmed in a block
    Confirmed { block_height: u64, confirmations: u64 },
    /// Transaction failed
    Failed { reason: String },
    /// Transaction was dropped from mempool
    Dropped,
    /// Unknown status
    Unknown,
}

/// Status of contract deployment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeploymentStatus {
    /// Deployment is pending
    Pending,
    /// Deployment succeeded
    Success {
        contract_address: String,
        transaction_hash: String,
        block_height: u64,
    },
    /// Deployment failed
    Failed { reason: String },
}

/// Status of finality for a transaction or anchor
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FinalityStatus {
    /// Not yet finalized
    Pending,
    /// Finalized (safe from reorgs per chain's finality rules)
    Finalized { block_height: u64, finality_block: u64 },
    /// Orphaned due to reorg
    Orphaned,
}

/// Contract status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractStatus {
    /// Contract address/program ID
    pub address: String,
    /// Whether the contract is deployed and active
    pub is_deployed: bool,
    /// Current balance (if applicable)
    pub balance: Option<u64>,
    /// Owner of the contract
    pub owner: Option<String>,
    /// Chain-specific metadata
    pub metadata: serde_json::Value,
}

/// Balance information for an address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceInfo {
    /// Address queried
    pub address: String,
    /// Total balance in smallest unit
    pub total: u64,
    /// Available/spendable balance
    pub available: u64,
    /// Locked/pending balance
    pub locked: u64,
    /// Token-specific balances (for chains with multiple tokens)
    pub tokens: Vec<TokenBalance>,
}

/// Token-specific balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    /// Token contract address or identifier
    pub token_id: String,
    /// Token symbol
    pub symbol: String,
    /// Token balance
    pub balance: u64,
    /// Number of decimals
    pub decimals: u8,
}

/// Transaction information returned from chain query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInfo {
    /// Transaction hash/signature
    pub hash: String,
    /// Sender address
    pub sender: String,
    /// Recipient address (if applicable)
    pub recipient: Option<String>,
    /// Amount transferred (if applicable)
    pub amount: Option<u64>,
    /// Transaction status
    pub status: TransactionStatus,
    /// Block height (if confirmed)
    pub block_height: Option<u64>,
    /// Timestamp (if available)
    pub timestamp: Option<u64>,
    /// Gas/fee paid
    pub fee: Option<u64>,
    /// Raw transaction data (chain-specific)
    pub raw_data: Option<Vec<u8>>,
}

/// Right operation types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RightOperation {
    /// Create a new right
    Create,
    /// Consume a right
    Consume,
    /// Lock a right for cross-chain transfer
    Lock,
    /// Mint a right on destination chain
    Mint,
    /// Refund a locked right
    Refund,
    /// Record right metadata
    RecordMetadata,
}

/// Result of a right operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RightOperationResult {
    /// The right ID
    pub right_id: RightId,
    /// Operation performed
    pub operation: RightOperation,
    /// Transaction hash
    pub transaction_hash: String,
    /// Block height
    pub block_height: u64,
    /// Chain ID
    pub chain_id: String,
    /// Additional chain-specific data
    pub metadata: serde_json::Value,
}

/// Trait for querying chain state
///
/// Implementors must provide real RPC-backed implementations.
/// No test or artificial data is allowed in production.
#[async_trait]
pub trait ChainQuery: Send + Sync {
    /// Get the balance for an address
    async fn get_balance(&self, address: &str) -> ChainOpResult<BalanceInfo>;

    /// Get transaction information by hash
    async fn get_transaction(&self, hash: &str) -> ChainOpResult<TransactionInfo>;

    /// Get finality status for a transaction or anchor
    async fn get_finality(&self, tx_hash: &str) -> ChainOpResult<FinalityStatus>;

    /// Get contract status at an address
    async fn get_contract_status(&self, contract_address: &str) -> ChainOpResult<ContractStatus>;

    /// Get the latest block height
    async fn get_latest_block_height(&self) -> ChainOpResult<u64>;

    /// Get chain-specific information (version, protocol, etc.)
    async fn get_chain_info(&self) -> ChainOpResult<serde_json::Value>;

    /// Check if an address is valid for this chain
    fn validate_address(&self, address: &str) -> bool;
}

/// Trait for chain signing operations
///
/// All signing operations must use secure key storage.
/// Private keys must never be exposed in plaintext.
#[async_trait]
pub trait ChainSigner: Send + Sync {
    /// Derive an address from a public key or key identifier
    fn derive_address(&self, public_key: &[u8]) -> ChainOpResult<String>;

    /// Sign a transaction
    ///
    /// The transaction data must be canonical serialized before signing.
    async fn sign_transaction(&self, tx_data: &[u8], key_id: &str) -> ChainOpResult<Vec<u8>>;

    /// Sign a message (for verification purposes)
    ///
    /// Messages should include domain separation.
    async fn sign_message(&self, message: &[u8], key_id: &str) -> ChainOpResult<Vec<u8>>;

    /// Verify a signature
    fn verify_signature(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> ChainOpResult<bool>;

    /// Get the signature scheme used by this chain
    fn signature_scheme(&self) -> crate::signature::SignatureScheme;
}

/// Trait for broadcasting transactions
///
/// Implementors must submit to real chain networks.
/// No artificial transaction acceptance is allowed.
#[async_trait]
pub trait ChainBroadcaster: Send + Sync {
    /// Submit a signed transaction to the chain
    ///
    /// Returns the transaction hash if successful.
    async fn submit_transaction(&self, signed_tx: &[u8]) -> ChainOpResult<String>;

    /// Confirm a transaction by hash
    ///
    /// Waits for the transaction to reach the specified confirmation level.
    /// Returns the final status of the transaction.
    async fn confirm_transaction(
        &self,
        tx_hash: &str,
        required_confirmations: u64,
        timeout_secs: u64,
    ) -> ChainOpResult<TransactionStatus>;

    /// Get the current recommended fee/gas price
    async fn get_fee_estimate(&self) -> ChainOpResult<u64>;

    /// Validate a transaction before submission (dry-run where supported)
    ///
    /// This should perform a real dry-run on the chain when available,
    /// not a local test validation.
    async fn validate_transaction(&self, tx_data: &[u8]) -> ChainOpResult<()>;
}

/// Trait for deploying contracts and programs
///
/// All deployments must happen on real chains.
/// No test deployments or temporary addresses allowed.
#[async_trait]
pub trait ChainDeployer: Send + Sync {
    /// Deploy a lock contract for cross-chain rights
    ///
    /// This contract locks rights on the source chain during transfers.
    async fn deploy_lock_contract(
        &self,
        admin_address: &str,
        config: serde_json::Value,
    ) -> ChainOpResult<DeploymentStatus>;

    /// Deploy a mint contract for cross-chain rights
    ///
    /// This contract mints rights on the destination chain after proof verification.
    async fn deploy_mint_contract(
        &self,
        admin_address: &str,
        config: serde_json::Value,
    ) -> ChainOpResult<DeploymentStatus>;

    /// Deploy or publish a seal program (chain-specific)
    ///
    /// For Solana: deploy Anchor program
    /// For Sui: publish Move package
    /// For Aptos: publish Move module
    async fn deploy_or_publish_seal_program(
        &self,
        program_bytes: &[u8],
        admin_address: &str,
    ) -> ChainOpResult<DeploymentStatus>;

    /// Verify that a deployment succeeded and is active
    async fn verify_deployment(&self, contract_address: &str) -> ChainOpResult<bool>;

    /// Get the estimated deployment cost
    async fn estimate_deployment_cost(&self, program_bytes: &[u8]) -> ChainOpResult<u64>;
}

/// Trait for building and verifying cryptographic proofs
///
/// All proof operations must use real cryptographic verification.
/// No fake or deterministic proofs allowed in production.
#[async_trait]
pub trait ChainProofProvider: Send + Sync {
    /// Build an inclusion proof for a commitment
    ///
    /// Proves that a commitment was included in a specific block.
    async fn build_inclusion_proof(
        &self,
        commitment: &Hash,
        block_height: u64,
    ) -> ChainOpResult<InclusionProof>;

    /// Verify an inclusion proof
    ///
    /// Returns true if the proof is valid, false otherwise.
    fn verify_inclusion_proof(
        &self,
        proof: &InclusionProof,
        commitment: &Hash,
    ) -> ChainOpResult<bool>;

    /// Build a finality proof for a transaction or anchor
    ///
    /// Proves that a transaction has reached finality on this chain.
    async fn build_finality_proof(&self, tx_hash: &str) -> ChainOpResult<FinalityProof>;

    /// Verify a finality proof
    ///
    /// Returns true if the proof is valid and the transaction is finalized.
    fn verify_finality_proof(
        &self,
        proof: &FinalityProof,
        tx_hash: &str,
    ) -> ChainOpResult<bool>;

    /// Get the domain separator for proof generation
    fn domain_separator(&self) -> [u8; 32];

    /// Verify a proof bundle (inclusion + finality)
    async fn verify_proof_bundle(
        &self,
        inclusion_proof: &InclusionProof,
        finality_proof: &FinalityProof,
        commitment: &Hash,
    ) -> ChainOpResult<bool>;
}

/// Trait for right operations
///
/// All operations must be backed by real chain transactions.
/// No artificial right state changes allowed.
#[async_trait]
pub trait ChainRightOps: Send + Sync {
    /// Create a new right on this chain
    ///
    /// Returns the operation result with transaction details.
    async fn create_right(
        &self,
        owner: &str,
        asset_class: &str,
        asset_id: &str,
        metadata: serde_json::Value,
    ) -> ChainOpResult<RightOperationResult>;

    /// Consume a right on this chain
    ///
    /// Marks the right as consumed and records the nullifier.
    async fn consume_right(
        &self,
        right_id: &RightId,
        owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult>;

    /// Lock a right for cross-chain transfer
    ///
    /// Locks the right on this chain and prepares for cross-chain mint.
    async fn lock_right(
        &self,
        right_id: &RightId,
        destination_chain: &str,
        owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult>;

    /// Mint a right on this chain (destination chain of cross-chain transfer)
    ///
    /// Creates a corresponding right after verifying the lock proof.
    async fn mint_right(
        &self,
        source_chain: &str,
        source_right_id: &RightId,
        lock_proof: &InclusionProof,
        new_owner: &str,
    ) -> ChainOpResult<RightOperationResult>;

    /// Refund a locked right
    ///
    /// Returns the right to the owner if the cross-chain transfer times out.
    async fn refund_right(
        &self,
        right_id: &RightId,
        owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult>;

    /// Record right metadata on-chain
    ///
    /// Updates or adds metadata for a right.
    async fn record_right_metadata(
        &self,
        right_id: &RightId,
        metadata: serde_json::Value,
        owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult>;

    /// Verify that a right exists and is in the expected state
    async fn verify_right_state(
        &self,
        right_id: &RightId,
        expected_state: &str,
    ) -> ChainOpResult<bool>;
}

/// Combined trait for full chain adapter capabilities
///
/// Implementors must provide real implementations for all operations.
/// Use `CapabilityUnavailable` error for operations not supported on a chain.
pub trait FullChainAdapter:
    ChainQuery + ChainSigner + ChainBroadcaster + ChainDeployer + ChainProofProvider + ChainRightOps
{
    /// Get the chain identifier
    fn chain_id(&self) -> &'static str;

    /// Get the chain name
    fn chain_name(&self) -> &'static str;

    /// Check if a specific capability is available
    fn is_capability_available(&self, capability: ChainCapability) -> bool;
}

/// Chain capabilities that may not be available on all chains
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChainCapability {
    /// Can query balances
    QueryBalance,
    /// Can sign transactions
    SignTransactions,
    /// Can broadcast transactions
    BroadcastTransactions,
    /// Can deploy contracts
    DeployContracts,
    /// Can build inclusion proofs
    BuildInclusionProofs,
    /// Can verify inclusion proofs
    VerifyInclusionProofs,
    /// Can build finality proofs
    BuildFinalityProofs,
    /// Can verify finality proofs
    VerifyFinalityProofs,
    /// Can create rights
    CreateRights,
    /// Can consume rights
    ConsumeRights,
    /// Can lock rights for cross-chain
    LockRights,
    /// Can mint rights on destination
    MintRights,
    /// Can refund locked rights
    RefundRights,
    /// Supports smart contracts
    SmartContracts,
    /// Supports NFTs
    Nfts,
    /// Supports cross-chain transfers
    CrossChain,
}

/// Blanket implementation to allow trait objects
impl<T: ChainQuery + ChainSigner + ChainBroadcaster + ChainDeployer + ChainProofProvider + ChainRightOps + Send + Sync>
    FullChainAdapter for T
{
    fn chain_id(&self) -> &'static str {
        "unknown"
    }

    fn chain_name(&self) -> &'static str {
        "Unknown Chain"
    }

    fn is_capability_available(&self, _capability: ChainCapability) -> bool {
        // By default, assume all capabilities are available
        // Individual adapters should override this
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_op_error_display() {
        let err = ChainOpError::CapabilityUnavailable("test".into());
        assert_eq!(
            err.to_string(),
            "Capability unavailable: test"
        );
    }

    #[test]
    fn test_transaction_status_serialization() {
        let status = TransactionStatus::Confirmed {
            block_height: 100,
            confirmations: 6,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("Confirmed"));
    }

    #[test]
    fn test_chain_capability_equality() {
        assert_eq!(ChainCapability::QueryBalance, ChainCapability::QueryBalance);
        assert_ne!(ChainCapability::QueryBalance, ChainCapability::SignTransactions);
    }
}
