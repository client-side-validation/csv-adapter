//! Solana transaction builder
//!
//! Solana uses a custom binary format (not BCS). Transactions consist of:
//! - Header (3 bytes)
//! - Account keys (compact array of 32-byte pubkeys)
//! - Recent blockhash (32 bytes)
//! - Instructions (compact array)
//! - Signatures (64 bytes each)

use crate::services::blockchain_service::BlockchainError;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use csv_adapter_core::Chain;

/// Solana Pubkey (32 bytes)
pub type Pubkey = [u8; 32];

/// Solana Hash (32 bytes)
pub type Hash = [u8; 32];

/// Solana Instruction
#[derive(Clone, Debug)]
pub struct Instruction {
    /// Program ID index
    pub program_id_index: u8,
    /// Account indices
    pub accounts: Vec<u8>,
    /// Instruction data
    pub data: Vec<u8>,
}

/// Solana Message (the part that gets signed)
#[derive(Clone, Debug)]
pub struct Message {
    /// Header
    pub num_required_signatures: u8,
    pub num_readonly_signed_accounts: u8,
    pub num_readonly_unsigned_accounts: u8,
    /// Account keys
    pub account_keys: Vec<Pubkey>,
    /// Recent blockhash
    pub recent_blockhash: Hash,
    /// Instructions
    pub instructions: Vec<Instruction>,
}

/// Solana Transaction
#[derive(Clone, Debug)]
pub struct Transaction {
    /// Signatures
    pub signatures: Vec<Vec<u8>>, // 64-byte signatures
    /// Message
    pub message: Message,
}

/// Encode a compact array (length + items)
fn encode_compact_array(data: &[u8]) -> Vec<u8> {
    let mut result = encode_compact_u16(data.len() as u16);
    result.extend_from_slice(data);
    result
}

/// Encode compact u16 (used by Solana for array lengths)
fn encode_compact_u16(value: u16) -> Vec<u8> {
    let mut result = Vec::new();
    let mut val = value;
    loop {
        let byte = (val & 0x7f) as u8;
        val >>= 7;
        if val == 0 {
            result.push(byte);
            break;
        } else {
            result.push(byte | 0x80);
        }
    }
    result
}

impl Message {
    /// Serialize the message to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();
        
        // Header (3 bytes)
        result.push(self.num_required_signatures);
        result.push(self.num_readonly_signed_accounts);
        result.push(self.num_readonly_unsigned_accounts);
        
        // Account keys (compact array)
        let mut keys_data = Vec::new();
        for key in &self.account_keys {
            keys_data.extend_from_slice(key);
        }
        result.extend_from_slice(&encode_compact_array(&keys_data));
        
        // Recent blockhash (32 bytes)
        result.extend_from_slice(&self.recent_blockhash);
        
        // Instructions (compact array of instructions)
        let mut instructions_data = Vec::new();
        for ix in &self.instructions {
            instructions_data.push(ix.program_id_index);
            instructions_data.extend_from_slice(&encode_compact_array(&ix.accounts));
            instructions_data.extend_from_slice(&encode_compact_array(&ix.data));
        }
        result.extend_from_slice(&encode_compact_array(&instructions_data));
        
        result
    }
}

impl Transaction {
    /// Serialize the full transaction (signatures + message)
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();
        
        // Signatures (compact array of 64-byte signatures)
        let mut sigs_data = Vec::new();
        for sig in &self.signatures {
            sigs_data.extend_from_slice(sig);
        }
        result.extend_from_slice(&encode_compact_array(&sigs_data));
        
        // Message
        let message_bytes = self.message.serialize();
        result.extend_from_slice(&message_bytes);
        
        result
    }
}

/// Fetch recent blockhash from Solana RPC
pub async fn fetch_recent_blockhash(rpc_url: &str) -> Result<Hash, BlockchainError> {
    let client = reqwest::Client::new();
    
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getLatestBlockhash",
        "params": [{"commitment": "finalized"}]
    });
    
    let response = client.post(rpc_url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| BlockchainError {
            message: format!("Failed to fetch blockhash: {}", e),
            chain: Some(Chain::Solana),
            code: None,
        })?;
    
    let json: serde_json::Value = response.json().await
        .map_err(|e| BlockchainError {
            message: format!("Failed to parse blockhash: {}", e),
            chain: Some(Chain::Solana),
            code: None,
        })?;
    
    let blockhash_str = json
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.get("blockhash"))
        .and_then(|b| b.as_str())
        .ok_or_else(|| BlockchainError {
            message: "Blockhash not found".to_string(),
            chain: Some(Chain::Solana),
            code: None,
        })?;
    
    // Parse base58-encoded blockhash
    parse_pubkey(blockhash_str)
}

/// Build a Solana instruction for invoking a program
pub fn build_instruction(
    program_id: &str,
    accounts: Vec<(Pubkey, bool, bool)>, // (pubkey, is_signer, is_writable)
    data: Vec<u8>,
) -> Result<(Instruction, Pubkey, Vec<Pubkey>), BlockchainError> {
    let program_id_bytes = parse_pubkey(program_id)?;
    
    // Collect unique account keys
    let mut account_keys: Vec<Pubkey> = Vec::new();
    let mut account_indices: Vec<u8> = Vec::new();
    
    // Add program ID first if not already in accounts
    let program_index = if let Some(pos) = account_keys.iter().position(|k| k == &program_id_bytes) {
        pos as u8
    } else {
        account_keys.push(program_id_bytes);
        (account_keys.len() - 1) as u8
    };
    
    // Add other accounts
    for (pubkey, _is_signer, _is_writable) in accounts {
        let index = if let Some(pos) = account_keys.iter().position(|k| k == &pubkey) {
            pos as u8
        } else {
            account_keys.push(pubkey);
            (account_keys.len() - 1) as u8
        };
        account_indices.push(index);
    }
    
    let instruction = Instruction {
        program_id_index: program_index,
        accounts: account_indices,
        data,
    };
    
    Ok((instruction, program_id_bytes, account_keys))
}

/// Build a complete Solana transaction
pub async fn build_solana_transaction(
    payer: &str,
    program_id: &str,
    accounts: Vec<(Pubkey, bool, bool)>, // (pubkey, is_signer, is_writable)
    instruction_data: Vec<u8>,
    rpc_url: &str,
) -> Result<Transaction, BlockchainError> {
    let payer_bytes = parse_pubkey(payer)?;
    
    // Fetch recent blockhash
    let recent_blockhash = fetch_recent_blockhash(rpc_url).await?;
    
    // Build instruction
    let (instruction, _program_id_bytes, mut account_keys) = 
        build_instruction(program_id, accounts, instruction_data)?;
    
    // Ensure payer is first and marked as signer
    if let Some(pos) = account_keys.iter().position(|k| k == &payer_bytes) {
        if pos != 0 {
            account_keys.swap(0, pos);
        }
    } else {
        account_keys.insert(0, payer_bytes);
    }
    
    // Calculate header values
    let num_required_signatures = 1; // Just the payer for now
    let num_readonly_signed_accounts = 0;
    let num_readonly_unsigned_accounts = 0; // All accounts are writable for now
    
    // Build message
    let message = Message {
        num_required_signatures,
        num_readonly_signed_accounts,
        num_readonly_unsigned_accounts,
        account_keys,
        recent_blockhash,
        instructions: vec![instruction],
    };
    
    Ok(Transaction {
        signatures: Vec::new(), // To be filled after signing
        message,
    })
}

/// Parse a base58-encoded Solana pubkey/address
pub fn parse_pubkey(addr: &str) -> Result<Pubkey, BlockchainError> {
    // Try base58 decoding
    match bs58::decode(addr).into_vec() {
        Ok(bytes) => {
            if bytes.len() == 32 {
                let mut result = [0u8; 32];
                result.copy_from_slice(&bytes);
                Ok(result)
            } else {
                Err(BlockchainError {
                    message: format!("Invalid Solana address length: {} bytes, expected 32", bytes.len()),
                    chain: Some(Chain::Solana),
                    code: None,
                })
            }
        }
        Err(e) => {
            // Try hex as fallback
            if addr.starts_with("0x") {
                let bytes = hex::decode(&addr[2..]).map_err(|_| BlockchainError {
                    message: format!("Invalid Solana address: {}", e),
                    chain: Some(Chain::Solana),
                    code: None,
                })?;
                if bytes.len() == 32 {
                    let mut result = [0u8; 32];
                    result.copy_from_slice(&bytes);
                    Ok(result)
                } else {
                    Err(BlockchainError {
                        message: format!("Invalid Solana hex address length: {} bytes", bytes.len()),
                        chain: Some(Chain::Solana),
                        code: None,
                    })
                }
            } else {
                Err(BlockchainError {
                    message: format!("Invalid Solana address: {}", e),
                    chain: Some(Chain::Solana),
                    code: None,
                })
            }
        }
    }
}

/// Broadcast a signed Solana transaction
pub async fn broadcast_solana_transaction(
    signed_tx: &Transaction,
    rpc_url: &str,
) -> Result<String, BlockchainError> {
    let client = reqwest::Client::new();
    
    // Serialize transaction
    let serialized = signed_tx.serialize();
    let encoded = STANDARD.encode(&serialized);
    
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "sendTransaction",
        "params": [
            encoded,
            {"encoding": "base64", "skipPreflight": false, "preflightCommitment": "confirmed"}
        ]
    });
    
    let response = client.post(rpc_url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| BlockchainError {
            message: format!("Failed to broadcast: {}", e),
            chain: Some(Chain::Solana),
            code: None,
        })?;
    
    let json: serde_json::Value = response.json().await
        .map_err(|e| BlockchainError {
            message: format!("Failed to parse response: {}", e),
            chain: Some(Chain::Solana),
            code: None,
        })?;
    
    // Check for error
    if let Some(error) = json.get("error") {
        return Err(BlockchainError {
            message: format!("RPC error: {}", error),
            chain: Some(Chain::Solana),
            code: None,
        });
    }
    
    // Extract signature
    json.get("result")
        .and_then(|r| r.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| BlockchainError {
            message: "Transaction signature not found".to_string(),
            chain: Some(Chain::Solana),
            code: None,
        })
}
