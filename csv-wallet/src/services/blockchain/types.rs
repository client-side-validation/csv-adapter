//! Blockchain service types and error definitions.

use csv_adapter_core::Chain;
use serde::{Deserialize, Serialize};

/// Blockchain operation error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainError {
    pub message: String,
    pub chain: Option<Chain>,
    pub code: Option<u32>,
}

impl std::fmt::Display for BlockchainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Blockchain error: {}", self.message)
    }
}

impl std::error::Error for BlockchainError {}

/// Transaction receipt.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionReceipt {
    pub tx_hash: String,
    pub block_number: Option<u64>,
    pub gas_used: Option<u64>,
    pub status: TransactionStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed(String),
}

/// Cross-chain transfer status.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CrossChainStatus {
    Initiated,
    Locked,
    ProofGenerated,
    ProofVerified,
    Minted,
    Completed,
    Failed(String),
}

/// Proof data for cross-chain verification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrossChainProof {
    pub source_chain: Chain,
    pub target_chain: Chain,
    pub right_id: String,
    pub lock_tx_hash: String,
    pub proof_data: ProofData,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProofData {
    Merkle {
        root: String,
        path: Vec<String>,
        leaf: String,
    },
    Mpt {
        account_proof: Vec<String>,
        storage_proof: Vec<String>,
        value: String,
    },
    Checkpoint {
        checkpoint_digest: String,
        transaction_block: u64,
        certificate: String,
    },
    Ledger {
        ledger_version: u64,
        proof: Vec<u8>,
        root_hash: String,
    },
    /// Zero-Knowledge proof (Phase 5)
    Zk {
        /// Proof system used (SP1, Groth16, etc.)
        proof_system: String,
        /// Base64-encoded proof bytes
        proof_bytes: String,
        /// Seal ID this proof is for
        seal_id: String,
        /// Block hash where proof was generated
        block_hash: String,
        /// Block height
        block_height: u64,
        /// Verifier key hash
        verifier_key_hash: String,
    },
}

/// Contract deployment info.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractDeployment {
    pub chain: Chain,
    pub contract_address: String,
    pub tx_hash: String,
    pub deployed_at: u64,
    pub contract_type: ContractType,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum ContractType {
    Registry,
    Bridge,
    Lock,
}

/// Result of a cross-chain transfer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrossChainTransferResult {
    pub transfer_id: String,
    pub lock_tx_hash: String,
    pub mint_tx_hash: String,
    pub proof: Option<CrossChainProof>,
    pub status: CrossChainStatus,
    /// Gas fee for the lock transaction on source chain (in native token units)
    pub source_fee: Option<u64>,
    /// Gas fee for the mint transaction on destination chain (in native token units)
    pub dest_fee: Option<u64>,
}

/// Map of deployed contracts by chain.
pub type ContractDeployments = std::collections::HashMap<Chain, ContractDeployment>;

/// Transaction to be signed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnsignedTransaction {
    pub chain: Chain,
    pub from: String,
    pub to: String,
    pub value: u64,
    pub data: Vec<u8>,
    pub nonce: Option<u64>,
    pub gas_price: Option<u64>,
    pub gas_limit: Option<u64>,
}

/// Signed transaction ready for broadcast.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub chain: Chain,
    pub tx_hash: String,
    pub raw_bytes: Vec<u8>,
}

/// Solana transaction (basic for deleted solana_tx module)
#[derive(Clone, Debug)]
pub struct SolanaTransaction {
    pub message: SolanaMessage,
    pub signatures: Vec<Vec<u8>>,
}

/// Solana message structure
#[derive(Clone, Debug)]
pub struct SolanaMessage {
    pub data: Vec<u8>,
}

impl SolanaMessage {
    pub fn serialize(&self) -> Vec<u8> {
        self.data.clone()
    }
}

/// Solana account metadata
#[derive(Clone, Debug)]
pub struct SolanaAccountMeta {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

/// Bitcoin UTXO (basic for deleted bitcoin_tx module)
#[derive(Clone, Debug)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub value: u64,
    pub script_pubkey: Vec<u8>,
}

/// Bitcoin UTXO from mempool.space API
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct BitcoinUtxo {
    pub txid: String,
    pub vout: u32,
    #[serde(rename = "value")]
    pub value: u64,
    #[serde(skip)]
    pub address: String,
    pub status: Option<BitcoinUtxoStatus>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct BitcoinUtxoStatus {
    pub confirmed: bool,
    pub block_height: Option<u64>,
    pub block_hash: Option<String>,
    pub block_time: Option<u64>,
}
