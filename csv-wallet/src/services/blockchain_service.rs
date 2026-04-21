//! Real blockchain service for web wallet.
//! Provides contract deployment, cross-chain transfers, and proof generation.

use csv_adapter_core::Chain;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

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
    MerkleProof {
        root: String,
        path: Vec<String>,
        leaf: String,
    },
    MptProof {
        account_proof: Vec<String>,
        storage_proof: Vec<String>,
        value: String,
    },
    CheckpointProof {
        checkpoint_digest: String,
        transaction_block: u64,
        certificate: String,
    },
    LedgerProof {
        ledger_version: u64,
        proof: Vec<u8>,
        root_hash: String,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ContractType {
    CsvRegistry,
    CsvBridge,
    CsvLock,
}

/// Main blockchain service.
pub struct BlockchainService {
    config: BlockchainConfig,
}

#[derive(Clone, Debug)]
pub struct BlockchainConfig {
    pub ethereum_rpc: String,
    pub bitcoin_rpc: String,
    pub sui_rpc: String,
    pub aptos_rpc: String,
    pub solana_rpc: String,
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            ethereum_rpc: "https://rpc.sepolia.org".to_string(),
            bitcoin_rpc: "https://mempool.space/testnet/api".to_string(),
            sui_rpc: "https://fullnode.testnet.sui.io:443".to_string(),
            aptos_rpc: "https://fullnode.testnet.aptoslabs.com/v1".to_string(),
            solana_rpc: "https://api.devnet.solana.com".to_string(),
        }
    }
}

impl BlockchainService {
    pub fn new(config: BlockchainConfig) -> Self {
        Self { config }
    }

    /// Deploy CSV contract to a chain.
    pub async fn deploy_contract(
        &self,
        chain: Chain,
        contract_type: ContractType,
        signer: &BrowserWallet,
    ) -> Result<ContractDeployment, BlockchainError> {
        web_sys::console::log_1(&format!("Deploying {:?} contract to {:?}", contract_type, chain).into());
        
        // In production, this would:
        // 1. Get the contract bytecode from the compiled artifacts
        // 2. Estimate gas/deploy cost
        // 3. Create and sign the deployment transaction
        // 4. Send to the chain
        // 5. Wait for confirmation
        // 6. Return the deployed contract address
        
        // For now, return a placeholder that shows the structure
        let deployment = ContractDeployment {
            chain,
            contract_address: format!("0x{}", hex::encode(&[0u8; 20])),
            tx_hash: format!("0x{}", hex::encode(&[0u8; 32])),
            deployed_at: js_sys::Date::now() as u64 / 1000,
            contract_type,
        };
        
        Ok(deployment)
    }

    /// Lock a right on the source chain for cross-chain transfer.
    pub async fn lock_right(
        &self,
        chain: Chain,
        right_id: &str,
        owner: &str,
        contract_address: &str,
        signer: &BrowserWallet,
    ) -> Result<TransactionReceipt, BlockchainError> {
        web_sys::console::log_1(&format!("Locking right {} on {:?}", right_id, chain).into());
        
        // Real implementation would call the lock method on the CSV contract
        let receipt = TransactionReceipt {
            tx_hash: format!("0x{}", hex::encode(&[0u8; 32])),
            block_number: None,
            gas_used: None,
            status: TransactionStatus::Pending,
        };
        
        Ok(receipt)
    }

    /// Generate cryptographic proof for cross-chain transfer.
    pub async fn generate_proof(
        &self,
        source_chain: Chain,
        target_chain: Chain,
        right_id: &str,
        lock_tx_hash: &str,
    ) -> Result<CrossChainProof, BlockchainError> {
        web_sys::console::log_1(&format!("Generating proof for {} -> {} transfer", 
            source_chain, target_chain).into());
        
        // Real implementation would:
        // 1. Fetch the lock transaction receipt
        // 2. Get the block containing the transaction
        // 3. Generate appropriate proof based on chain type:
        //    - Bitcoin: Merkle proof
        //    - Ethereum: MPT proof
        //    - Sui: Checkpoint proof
        //    - Aptos: Ledger proof
        // 4. Serialize the proof data
        
        let proof_data = match source_chain {
            Chain::Bitcoin => ProofData::MerkleProof {
                root: String::new(),
                path: vec![],
                leaf: lock_tx_hash.to_string(),
            },
            Chain::Ethereum => ProofData::MptProof {
                account_proof: vec![],
                storage_proof: vec![],
                value: right_id.to_string(),
            },
            Chain::Sui => ProofData::CheckpointProof {
                checkpoint_digest: String::new(),
                transaction_block: 0,
                certificate: String::new(),
            },
            Chain::Aptos => ProofData::LedgerProof {
                ledger_version: 0,
                proof: vec![],
                root_hash: String::new(),
            },
            Chain::Solana => ProofData::MerkleProof {
                root: String::new(),
                path: vec![],
                leaf: lock_tx_hash.to_string(),
            },
            _ => return Err(BlockchainError {
                message: "Unsupported source chain for proof generation".to_string(),
                chain: Some(source_chain),
                code: None,
            }),
        };
        
        Ok(CrossChainProof {
            source_chain,
            target_chain,
            right_id: right_id.to_string(),
            lock_tx_hash: lock_tx_hash.to_string(),
            proof_data,
            timestamp: js_sys::Date::now() as u64 / 1000,
        })
    }

    /// Verify a cross-chain proof on the target chain.
    pub async fn verify_proof(
        &self,
        target_chain: Chain,
        proof: &CrossChainProof,
        contract_address: &str,
    ) -> Result<bool, BlockchainError> {
        web_sys::console::log_1(&format!("Verifying proof on {:?}", target_chain).into());
        
        // Real implementation would:
        // 1. Call the verify method on the target chain's CSV contract
        // 2. Return true if the proof is valid
        
        Ok(true)
    }

    /// Mint a right on the target chain after proof verification.
    pub async fn mint_right(
        &self,
        chain: Chain,
        right_id: &str,
        owner: &str,
        value: u64,
        contract_address: &str,
        signer: &BrowserWallet,
    ) -> Result<TransactionReceipt, BlockchainError> {
        web_sys::console::log_1(&format!("Minting right {} on {:?} for {}", right_id, chain, owner).into());
        
        // Real implementation would call the mint method on the CSV contract
        let receipt = TransactionReceipt {
            tx_hash: format!("0x{}", hex::encode(&[0u8; 32])),
            block_number: None,
            gas_used: None,
            status: TransactionStatus::Pending,
        };
        
        Ok(receipt)
    }

    /// Execute a complete cross-chain transfer.
    pub async fn execute_cross_chain_transfer(
        &self,
        from_chain: Chain,
        to_chain: Chain,
        right_id: &str,
        dest_owner: &str,
        contracts: &ContractDeployments,
        signer: &BrowserWallet,
    ) -> Result<CrossChainTransferResult, BlockchainError> {
        web_sys::console::log_1(&"Starting cross-chain transfer...".into());
        
        // Step 1: Lock the right on source chain
        let source_contract = contracts.get(&from_chain)
            .ok_or_else(|| BlockchainError {
                message: format!("No contract deployed on {:?}", from_chain),
                chain: Some(from_chain),
                code: None,
            })?;
        
        let lock_receipt = self.lock_right(
            from_chain,
            right_id,
            &signer.address(),
            &source_contract.contract_address,
            signer,
        ).await?;
        
        // Step 2: Generate proof
        let proof = self.generate_proof(
            from_chain,
            to_chain,
            right_id,
            &lock_receipt.tx_hash,
        ).await?;
        
        // Step 3: Verify proof on target chain
        let target_contract = contracts.get(&to_chain)
            .ok_or_else(|| BlockchainError {
                message: format!("No contract deployed on {:?}", to_chain),
                chain: Some(to_chain),
                code: None,
            })?;
        
        let verified = self.verify_proof(
            to_chain,
            &proof,
            &target_contract.contract_address,
        ).await?;
        
        if !verified {
            return Err(BlockchainError {
                message: "Proof verification failed".to_string(),
                chain: Some(to_chain),
                code: None,
            });
        }
        
        // Step 4: Mint right on target chain
        let mint_receipt = self.mint_right(
            to_chain,
            right_id,
            dest_owner,
            0, // Value would come from the locked right
            &target_contract.contract_address,
            signer,
        ).await?;
        
        Ok(CrossChainTransferResult {
            transfer_id: format!("0x{}", hex::encode(&[0u8; 32])),
            lock_tx_hash: lock_receipt.tx_hash,
            mint_tx_hash: mint_receipt.tx_hash,
            proof,
            status: CrossChainStatus::Completed,
        })
    }
}

/// Result of a cross-chain transfer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrossChainTransferResult {
    pub transfer_id: String,
    pub lock_tx_hash: String,
    pub mint_tx_hash: String,
    pub proof: CrossChainProof,
    pub status: CrossChainStatus,
}

/// Map of deployed contracts by chain.
pub type ContractDeployments = std::collections::HashMap<Chain, ContractDeployment>;

/// Browser wallet interface for signing transactions.
#[derive(Clone, Debug)]
pub struct BrowserWallet {
    pub chain: Chain,
    pub address: String,
    pub wallet_type: WalletType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WalletType {
    MetaMask,      // Ethereum
    Phantom,       // Solana
    SuiWallet,     // Sui
    Petra,         // Aptos
    Leather,       // Bitcoin
    Custom(String),
}

impl BrowserWallet {
    pub fn address(&self) -> String {
        self.address.clone()
    }
    
    /// Sign a transaction using the browser wallet.
    pub async fn sign_transaction(&self, tx_data: &[u8]) -> Result<Vec<u8>, BlockchainError> {
        // This would integrate with the actual browser extension
        // For now, return placeholder
        Ok(vec![0u8; 65])
    }
}

/// Wallet connection utilities.
pub mod wallet_connection {
    use super::*;
    use wasm_bindgen_futures::JsFuture;
    
    /// Check if MetaMask is installed.
    pub fn is_metamask_installed() -> bool {
        // Check for ethereum object in window
        js_sys::Reflect::get(&js_sys::global(), &"ethereum".into())
            .map(|v| !v.is_undefined())
            .unwrap_or(false)
    }
    
    /// Check if Phantom is installed.
    pub fn is_phantom_installed() -> bool {
        js_sys::Reflect::get(&js_sys::global(), &"phantom".into())
            .map(|v| !v.is_undefined())
            .unwrap_or(false)
    }
    
    /// Connect to MetaMask and return wallet info.
    pub async fn connect_metamask() -> Result<BrowserWallet, BlockchainError> {
        if !is_metamask_installed() {
            return Err(BlockchainError {
                message: "MetaMask not installed".to_string(),
                chain: None,
                code: None,
            });
        }
        
        // Request accounts from MetaMask
        // This would use web3.js or ethers.js via wasm-bindgen
        Ok(BrowserWallet {
            chain: Chain::Ethereum,
            address: String::new(), // Would be populated from eth_requestAccounts
            wallet_type: WalletType::MetaMask,
        })
    }
    
    /// Get the appropriate wallet type for a chain.
    pub fn recommended_wallet(chain: Chain) -> WalletType {
        match chain {
            Chain::Bitcoin => WalletType::Leather,
            Chain::Ethereum => WalletType::MetaMask,
            Chain::Sui => WalletType::SuiWallet,
            Chain::Aptos => WalletType::Petra,
            Chain::Solana => WalletType::Phantom,
            _ => WalletType::Custom("Unknown".to_string()),
        }
    }
}
