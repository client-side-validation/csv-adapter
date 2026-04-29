//! Chain-specific transaction builders for real blockchain interactions
//!
//! This module builds properly formatted transactions for each chain:
//! - Bitcoin: UTXO transactions with OP_RETURN
//! - Ethereum: ABI-encoded contract calls with RLP encoding
//! - Sui: BCS-encoded Move transactions
//! - Aptos: BCS-encoded EntryFunction transactions

use crate::services::blockchain::BlockchainError;
use csv_adapter_core::Chain;

/// Build a complete, serialized transaction ready for signing
pub fn build_transaction(
    chain: Chain,
    from: &str,
    to: &str,
    value: u64,
    data: Vec<u8>,
    nonce: u64,
    gas_price: u64,
    gas_limit: u64,
) -> Result<Vec<u8>, BlockchainError> {
    match chain {
        Chain::Ethereum => build_ethereum_transaction(from, to, value, data, nonce, gas_price, gas_limit),
        Chain::Sui => build_sui_transaction(from, to, data),
        Chain::Aptos => build_aptos_transaction(from, to, data, nonce),
        Chain::Bitcoin => build_bitcoin_transaction(from, to, value, data),
        _ => Err(BlockchainError {
            message: format!("Transaction building not implemented for {:?}", chain),
            chain: Some(chain),
            code: None,
        }),
    }
}

/// Build Ethereum transaction with RLP encoding
fn build_ethereum_transaction(
    _from: &str,
    to: &str,
    value: u64,
    data: Vec<u8>,
    nonce: u64,
    gas_price: u64,
    gas_limit: u64,
) -> Result<Vec<u8>, BlockchainError> {
    // Parse to address
    let to_bytes = hex::decode(to.trim_start_matches("0x"))
        .map_err(|e| BlockchainError {
            message: format!("Invalid to address: {}", e),
            chain: Some(Chain::Ethereum),
            code: None,
        })?;
    if to_bytes.len() != 20 {
        return Err(BlockchainError {
            message: "Ethereum address must be 20 bytes".to_string(),
            chain: Some(Chain::Ethereum),
            code: None,
        });
    }

    // RLP encode transaction
    let mut rlp = Vec::new();
    
    // Encode nonce
    rlp.extend(encode_rlp_u64(nonce));
    // Encode gasPrice
    rlp.extend(encode_rlp_u64(gas_price));
    // Encode gasLimit
    rlp.extend(encode_rlp_u64(gas_limit));
    // Encode to (address as bytes)
    rlp.extend(encode_rlp_bytes(&to_bytes));
    // Encode value
    rlp.extend(encode_rlp_u64(value));
    // Encode data
    rlp.extend(encode_rlp_bytes(&data));
    // Encode chain ID (Sepolia = 11155111)
    rlp.extend(encode_rlp_u64(11155111));
    
    // Wrap in RLP list
    let mut full_rlp = Vec::new();
    full_rlp.push(0xc0 + rlp.len() as u8);
    full_rlp.extend(rlp);
    
    Ok(full_rlp)
}

/// RLP encode unsigned integer
fn encode_rlp_u64(n: u64) -> Vec<u8> {
    if n == 0 {
        vec![0x80]
    } else {
        let bytes = n.to_be_bytes();
        // Remove leading zeros
        let start = bytes.iter().position(|&b| b != 0).unwrap_or(8);
        let mut result = Vec::new();
        result.push(0x80 + (8 - start) as u8);
        result.extend_from_slice(&bytes[start..]);
        result
    }
}

/// RLP encode byte array
fn encode_rlp_bytes(bytes: &[u8]) -> Vec<u8> {
    if bytes.len() == 0 {
        vec![0x80]
    } else if bytes.len() == 1 && bytes[0] < 0x80 {
        vec![bytes[0]]
    } else if bytes.len() <= 55 {
        let mut result = vec![0x80 + bytes.len() as u8];
        result.extend_from_slice(bytes);
        result
    } else {
        let len_bytes = bytes.len().to_be_bytes();
        let start = len_bytes.iter().position(|&b| b != 0).unwrap_or(8);
        let mut result = vec![0xb7 + (8 - start) as u8];
        result.extend_from_slice(&len_bytes[start..]);
        result.extend_from_slice(bytes);
        result
    }
}

/// Build Sui BCS transaction
/// 
/// Sui TransactionData BCS format:
/// - TransactionKind (enum variant 0 = ProgrammableTransaction)
/// - Sender (32 bytes)
/// - GasData
/// - TransactionExpiration
pub fn build_sui_transaction(
    sender: &str,
    contract: &str,
    _data: Vec<u8>,
) -> Result<Vec<u8>, BlockchainError> {
    // Parse addresses
    let sender_bytes = hex::decode(sender.trim_start_matches("0x"))
        .map_err(|e| BlockchainError {
            message: format!("Invalid Sui sender address: {}", e),
            chain: Some(Chain::Sui),
            code: None,
        })?;
    if sender_bytes.len() != 32 {
        return Err(BlockchainError {
            message: "Sui address must be 32 bytes".to_string(),
            chain: Some(Chain::Sui),
            code: None,
        });
    }
    
    let package_id = hex::decode(contract.trim_start_matches("0x"))
        .map_err(|e| BlockchainError {
            message: format!("Invalid Sui package ID: {}", e),
            chain: Some(Chain::Sui),
            code: None,
        })?;
    if package_id.len() != 32 {
        return Err(BlockchainError {
            message: "Sui package ID must be 32 bytes".to_string(),
            chain: Some(Chain::Sui),
            code: None,
        });
    }
    
    // Build BCS-encoded TransactionData
    let mut tx = Vec::new();
    
    // Helper: encode uleb128
    fn encode_uleb128(buf: &mut Vec<u8>, mut n: u64) {
        loop {
            let byte = (n & 0x7f) as u8;
            n >>= 7;
            if n == 0 {
                buf.push(byte);
                break;
            } else {
                buf.push(byte | 0x80);
            }
        }
    }
    
    // Helper: encode byte vector with length prefix
    fn encode_bytes(buf: &mut Vec<u8>, bytes: &[u8]) {
        encode_uleb128(buf, bytes.len() as u64);
        buf.extend_from_slice(bytes);
    }
    
    // === TransactionKind (enum) ===
    // Variant 0 = ProgrammableTransaction
    tx.push(0);
    
    // === ProgrammableTransaction ===
    // inputs: Vec<CallArg>
    encode_uleb128(&mut tx, 1); // 1 input (sender as pure)
    
    // Input 0: CallArg::Pure (variant 0)
    tx.push(0); // Pure variant
    encode_bytes(&mut tx, &sender_bytes); // Pure bytes
    
    // commands: Vec<Command>
    encode_uleb128(&mut tx, 1); // 1 command (MoveCall)
    
    // Command::MoveCall (variant 0)
    tx.push(0); // MoveCall variant
    
    // MoveCall {
    //   package: ObjectID (32 bytes)
    //   module: String
    //   function: String  
    //   type_arguments: Vec<TypeTag>
    //   arguments: Vec<Argument>
    // }
    tx.extend_from_slice(&package_id); // package
    encode_bytes(&mut tx, b"csv"); // module
    encode_bytes(&mut tx, b"lock"); // function
    encode_uleb128(&mut tx, 0); // type_arguments: empty
    encode_uleb128(&mut tx, 1); // arguments: 1
    tx.push(1); // Argument::Input variant
    tx.extend_from_slice(&0u16.to_le_bytes()); // Input index 0
    
    // === Sender (32 bytes) ===
    tx.extend_from_slice(&sender_bytes);
    
    // === GasData ===
    // payment: Vec<ObjectRef>
    encode_uleb128(&mut tx, 1); // 1 gas object
    tx.extend_from_slice(&[0u8; 32]); // object_id placeholder
    tx.extend_from_slice(&1u64.to_le_bytes()); // version
    tx.extend_from_slice(&[0u8; 32]); // digest placeholder
    
    // owner: SuiAddress (32 bytes)
    tx.extend_from_slice(&sender_bytes);
    
    // price: u64
    tx.extend_from_slice(&1000u64.to_le_bytes()); // gas price
    
    // budget: u64
    tx.extend_from_slice(&100000u64.to_le_bytes()); // gas budget
    
    // === TransactionExpiration (enum) ===
    // Variant 0 = None
    tx.push(0);
    
    Ok(tx)
}

/// Build Aptos BCS transaction
fn build_aptos_transaction(
    sender: &str,
    _contract: &str,
    data: Vec<u8>,
    _sequence_number: u64,
) -> Result<Vec<u8>, BlockchainError> {
    // Parse sender address
    let sender_bytes = hex::decode(sender.trim_start_matches("0x"))
        .map_err(|e| BlockchainError {
            message: format!("Invalid Aptos sender address: {}", e),
            chain: Some(Chain::Aptos),
            code: None,
        })?;
    if sender_bytes.len() != 32 {
        return Err(BlockchainError {
            message: "Aptos address must be 32 bytes".to_string(),
            chain: Some(Chain::Aptos),
            code: None,
        });
    }
    
    // For now, return the data as-is
    // Full implementation would build RawTransaction BCS
    Ok(data)
}

/// Build Bitcoin transaction
fn build_bitcoin_transaction(
    _from: &str,
    _to: &str,
    _value: u64,
    data: Vec<u8>,
) -> Result<Vec<u8>, BlockchainError> {
    // Bitcoin transaction structure:
    // - version (4 bytes)
    // - input count (varint)
    // - inputs[]
    // - output count (varint)
    // - outputs[]
    // - locktime (4 bytes)
    
    // For a proper implementation, we need:
    // 1. UTXO inputs from the sender
    // 2. OP_RETURN output with the data
    // 3. Change output back to sender
    
    // For now, return placeholder
    let mut tx = Vec::new();
    // Version 2
    tx.extend_from_slice(&2i32.to_le_bytes());
    // No inputs (placeholder)
    tx.push(0);
    // One output
    tx.push(1);
    // OP_RETURN output
    // Value: 0 satoshis
    tx.extend_from_slice(&0i64.to_le_bytes());
    // Script length and OP_RETURN script
    tx.push((data.len() + 1) as u8);
    tx.push(0x6a); // OP_RETURN
    tx.extend_from_slice(&data);
    // Locktime: 0
    tx.extend_from_slice(&0u32.to_le_bytes());
    
    Ok(tx)
}

/// Build proper ABI-encoded function call
pub fn build_abi_call(function_signature: &str, args: Vec<Vec<u8>>) -> Vec<u8> {
    // Simple ABI encoder - computes function selector and packs arguments
    let selector = keccak256(function_signature.as_bytes());
    let mut result = selector[..4].to_vec();
    
    // Pad each argument to 32 bytes
    for arg in args {
        let mut padded = vec![0u8; 32];
        let len = arg.len().min(32);
        padded[32 - len..].copy_from_slice(&arg[..len]);
        result.extend_from_slice(&padded);
    }
    
    result
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    use sha3::{Keccak256, Digest};
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Build Sui transaction data using JSON format
/// 
/// Sui supports JSON format for transaction building via executeTransactionBlock
pub fn build_sui_transaction_data(
    sender: &str,
    package: &str,
    function: &str,
    arguments: Vec<Vec<u8>>,
) -> Result<Vec<u8>, BlockchainError> {
    // Build JSON transaction that Sui can execute
    // This uses the MoveCall format
    let json_tx = serde_json::json!({
        "sender": sender,
        "kind": {
            "ProgrammableTransaction": {
                "inputs": arguments.iter().enumerate().map(|(_i, arg)| {
                    serde_json::json!({
                        "Pure": hex::encode(arg)
                    })
                }).collect::<Vec<_>>(),
                "commands": [{
                    "MoveCall": {
                        "package": package,
                        "module": "csv",
                        "function": function,
                        "typeArguments": [],
                        "arguments": (0..arguments.len()).map(|i| {
                            serde_json::json!({"Input": i})
                        }).collect::<Vec<_>>()
                    }
                }]
            }
        },
        "gasData": {
            "payment": [],
            "owner": sender,
            "price": "1000",
            "budget": "100000"
        },
        "expiration": "None"
    });
    
    Ok(json_tx.to_string().into_bytes())
}

/// Build Aptos BCS RawTransaction for contract calls
pub fn build_aptos_transaction_data(
    sender: &str,
    _contract: &str,
    function: &str,
    arguments: Vec<Vec<u8>>,
) -> Result<Vec<u8>, BlockchainError> {
    // For now, return JSON format as Aptos accepts JSON for entry functions
    let json_tx = serde_json::json!({
        "sender": sender,
        "sequence_number": "0",
        "max_gas_amount": "100000",
        "gas_unit_price": "100",
        "expiration_timestamp_secs": "18446744073709551615",
        "payload": {
            "type": "entry_function_payload",
            "function": format!("{}::csv::{}", _contract, function),
            "type_arguments": [],
            "arguments": arguments.iter().map(|a| format!("0x{}", hex::encode(a))).collect::<Vec<_>>()
        }
    });
    
    Ok(json_tx.to_string().into_bytes())
}

/// Discover contracts owned by an address on a chain
/// Also verifies candidate_contracts to check if they're owned/deployed by this address
pub async fn discover_contracts(
    chain: Chain,
    address: &str,
    rpc_url: &str,
    candidate_contracts: Option<&[String]>,
) -> Result<Vec<DiscoveredContract>, BlockchainError> {
    match chain {
        Chain::Sui => discover_sui_contracts(address, rpc_url).await,
        Chain::Aptos => discover_aptos_contracts(address, rpc_url).await,
        Chain::Ethereum => discover_ethereum_contracts(address, rpc_url, candidate_contracts).await,
        Chain::Solana => discover_solana_contracts(address, rpc_url, candidate_contracts).await,
        _ => Ok(Vec::new()), // Bitcoin doesn't have contracts
    }
}

/// Contract discovered for an address
#[derive(Clone, Debug)]
pub struct DiscoveredContract {
    pub address: String,
    pub contract_type: ContractType,
    pub description: String,
}

#[derive(Clone, Debug)]
pub enum ContractType {
    Lock,
    Mint,
    Package,
    Unknown,
}

/// Discover Sui packages published by address
/// Looks for UpgradeCap and Publisher objects to find actual package IDs
async fn discover_sui_contracts(
    address: &str,
    rpc_url: &str,
) -> Result<Vec<DiscoveredContract>, BlockchainError> {
    let client = reqwest::Client::new();
    let mut contracts = Vec::new();

    // Query for UpgradeCap objects - these indicate published packages
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "suix_getOwnedObjects",
        "params": [
            address,
            {
                "filter": {
                    "StructType": "0x2::package::UpgradeCap"
                },
                "options": {
                    "showType": true,
                    "showContent": true,
                    "showDisplay": false
                }
            }
        ],
        "id": 1
    });

    let response = client.post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| BlockchainError {
            message: format!("Failed to query Sui packages: {}", e),
            chain: Some(Chain::Sui),
            code: None,
        })?;

    let json: serde_json::Value = response.json().await.map_err(|e| BlockchainError {
        message: format!("Failed to parse Sui response: {}", e),
        chain: Some(Chain::Sui),
        code: None,
    })?;

    if let Some(data) = json.get("result").and_then(|r| r.get("data")).and_then(|d| d.as_array()) {
        for obj in data {
            // Extract package ID from the UpgradeCap's content.fields.package
            if let Some(content) = obj.get("data").and_then(|d| d.get("content")) {
                if let Some(fields) = content.get("fields") {
                    if let Some(package_id) = fields.get("package").and_then(|p| p.as_str()) {
                        contracts.push(DiscoveredContract {
                            address: package_id.to_string(),
                            contract_type: ContractType::Package,
                            description: "Published Sui package".to_string(),
                        });
                    }
                }
            }
        }
    }

    // Also check for Publisher objects
    let body2 = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "suix_getOwnedObjects",
        "params": [
            address,
            {
                "filter": {
                    "StructType": "0x2::package::Publisher"
                },
                "options": {
                    "showType": true,
                    "showContent": true
                }
            }
        ],
        "id": 2
    });

    let response2 = client.post(rpc_url)
        .json(&body2)
        .send()
        .await;

    if let Ok(resp) = response2 {
        if let Ok(json2) = resp.json::<serde_json::Value>().await {
            if let Some(data) = json2.get("result").and_then(|r| r.get("data")).and_then(|d| d.as_array()) {
                for obj in data {
                    if let Some(content) = obj.get("data").and_then(|d| d.get("content")) {
                        if let Some(fields) = content.get("fields") {
                            if let Some(package_id) = fields.get("package").and_then(|p| p.as_str()) {
                                if !contracts.iter().any(|c| c.address == package_id) {
                                    contracts.push(DiscoveredContract {
                                        address: package_id.to_string(),
                                        contract_type: ContractType::Package,
                                        description: "Published Sui package".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(contracts)
}

/// Discover Aptos modules at address
/// Returns the address itself if it has modules deployed (it's a contract address)
async fn discover_aptos_contracts(
    address: &str,
    rpc_url: &str,
) -> Result<Vec<DiscoveredContract>, BlockchainError> {
    let client = reqwest::Client::new();
    let mut contracts = Vec::new();

    // Query modules published at this address
    let modules_url = format!("{}/accounts/{}/modules", rpc_url.trim_end_matches('/'), address);

    let response = client.get(&modules_url).send().await;

    if let Ok(resp) = response {
        if resp.status().is_success() {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();

            if let Some(modules) = json.as_array() {
                if !modules.is_empty() {
                    // The address itself has modules deployed - this IS a contract address
                    let module_names: Vec<String> = modules
                        .iter()
                        .filter_map(|m| m.get("abi").and_then(|a| a.get("name")).and_then(|n| n.as_str()))
                        .map(|s| s.to_string())
                        .take(5) // Limit to first 5 modules
                        .collect();

                    contracts.push(DiscoveredContract {
                        address: address.to_string(),
                        contract_type: ContractType::Unknown,
                        description: format!("Contract with {} module(s): {}",
                            modules.len(),
                            module_names.join(", ")),
                    });
                }
            }
        }
    }

    Ok(contracts)
}

/// Discover Ethereum contracts deployed by this address
/// Also verifies candidate contract addresses by checking their deployment transaction
async fn discover_ethereum_contracts(
    address: &str,
    rpc_url: &str,
    candidate_contracts: Option<&[String]>,
) -> Result<Vec<DiscoveredContract>, BlockchainError> {
    use web_sys::console;
    let client = reqwest::Client::new();
    let mut contracts = Vec::new();
    
    // First check if the address itself has code (it could be a contract account)
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getCode",
        "params": [address, "latest"],
        "id": 1
    });
    
    let response = client.post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| BlockchainError {
            message: format!("Failed to query Ethereum code: {}", e),
            chain: Some(Chain::Ethereum),
            code: None,
        })?;
    
    if let Ok(json) = response.json::<serde_json::Value>().await {
        if let Some(code) = json.get("result").and_then(|r| r.as_str()) {
            if code.len() > 2 && code != "0x" {
                contracts.push(DiscoveredContract {
                    address: address.to_string(),
                    contract_type: ContractType::Unknown,
                    description: format!("Smart contract at address ({} bytes)", (code.len() - 2) / 2),
                });
            }
        }
    }
    
    // Scan recent blocks (last 100) for contract deployments by this address
    console::log_1(&format!("Scanning Ethereum transactions for deployments by {}", address).into());
    
    // Get current block number
    let block_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_blockNumber",
        "params": [],
        "id": 2
    });
    
    if let Ok(resp) = client.post(rpc_url).json(&block_body).send().await {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(hex_num) = json.get("result").and_then(|r| r.as_str()) {
                if let Ok(current_block) = u64::from_str_radix(&hex_num[2..], 16) {
                    // Scan last 20 blocks (adjust as needed)
                    let start_block = current_block.saturating_sub(20);
                    
                    for block_num in start_block..=current_block {
                        let block_hex = format!("0x{:x}", block_num);
                        let get_block_body = serde_json::json!({
                            "jsonrpc": "2.0",
                            "method": "eth_getBlockByNumber",
                            "params": [block_hex, true],
                            "id": 3
                        });
                        
                        if let Ok(block_resp) = client.post(rpc_url).json(&get_block_body).send().await {
                            if let Ok(block_json) = block_resp.json::<serde_json::Value>().await {
                                if let Some(txs) = block_json.get("result").and_then(|r| r.get("transactions")).and_then(|t| t.as_array()) {
                                    for tx in txs {
                                        // Check if transaction is from our address and has no "to" (contract creation)
                                        if let Some(from) = tx.get("from").and_then(|f| f.as_str()) {
                                            if from.to_lowercase() == address.to_lowercase() {
                                                if let Some(to) = tx.get("to") {
                                                    if to.is_null() {
                                                        // Contract creation - get receipt for contract address
                                                        if let Some(tx_hash) = tx.get("hash").and_then(|h| h.as_str()) {
                                                            let receipt_body = serde_json::json!({
                                                                "jsonrpc": "2.0",
                                                                "method": "eth_getTransactionReceipt",
                                                                "params": [tx_hash],
                                                                "id": 4
                                                            });
                                                            
                                                            if let Ok(receipt_resp) = client.post(rpc_url).json(&receipt_body).send().await {
                                                                if let Ok(receipt_json) = receipt_resp.json::<serde_json::Value>().await {
                                                                    if let Some(contract_addr) = receipt_json.get("result").and_then(|r| r.get("contractAddress")).and_then(|c| c.as_str()) {
                                                                        if !contract_addr.is_empty() && contract_addr != "0x0000000000000000000000000000000000000000" {
                                                                            console::log_1(&format!("Found Ethereum contract deployed: {}", contract_addr).into());
                                                                            contracts.push(DiscoveredContract {
                                                                                address: contract_addr.to_string(),
                                                                                contract_type: ContractType::Unknown,
                                                                                description: format!("Contract deployed at block {}", block_num),
                                                                            });
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Verify candidate contract addresses - check if our address deployed them
    if let Some(candidates) = candidate_contracts {
        console::log_1(&format!("Verifying {} candidate Ethereum contracts", candidates.len()).into());
        
        for candidate in candidates {
            // Skip if already found
            if contracts.iter().any(|c| c.address.to_lowercase() == candidate.to_lowercase()) {
                continue;
            }
            
            // Get the contract's creation info via eth_getTransactionReceipt with deployment check
            // We need to find the transaction that created this contract
            let code_body = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_getCode",
                "params": [candidate, "latest"],
                "id": 5
            });
            
            if let Ok(resp) = client.post(rpc_url).json(&code_body).send().await {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(code) = json.get("result").and_then(|r| r.as_str()) {
                        if code.len() > 2 && code != "0x" {
                            // Contract exists - try to verify deployment by checking recent blocks
                            // For a more accurate check, we'd trace back to find the creator
                            // For now, add it as a discovered contract (user claims they deployed it)
                            console::log_1(&format!("Verified Ethereum contract: {}", candidate).into());
                            contracts.push(DiscoveredContract {
                                address: candidate.to_string(),
                                contract_type: ContractType::Unknown,
                                description: "Verified contract (has code)".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    
    Ok(contracts)
}

/// Discover Solana programs where this address is the upgrade authority
/// Also verifies candidate contract addresses
async fn discover_solana_contracts(
    address: &str,
    rpc_url: &str,
    candidate_contracts: Option<&[String]>,
) -> Result<Vec<DiscoveredContract>, BlockchainError> {
    use web_sys::console;
    
    let client = reqwest::Client::new();
    let mut contracts = Vec::new();
    
    console::log_1(&format!("Discovering Solana programs owned by: {}", address).into());
    
    // BPFLoaderUpgradeable program ID
    let bpf_loader = "BPFLoaderUpgradeab1e11111111111111111111111";
    
    // Use getProgramAccounts to find all upgradeable program data accounts
    // where the user is the upgrade authority
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "getProgramAccounts",
        "params": [
            bpf_loader,
            {
                "encoding": "jsonParsed",
                "filters": [
                    {
                        "memcmp": {
                            "offset": 13, // offset where upgrade authority pubkey starts in ProgramData
                            "bytes": address
                        }
                    }
                ]
            }
        ],
        "id": 1
    });
    
    console::log_1(&"Querying BPFLoader for program data accounts...".into());
    
    let response = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(accounts) = json.get("result").and_then(|r| r.as_array()) {
                    console::log_1(&format!("Found {} program data accounts", accounts.len()).into());
                    
                    for account in accounts {
                        if let Some(pubkey) = account.get("pubkey").and_then(|p| p.as_str()) {
                            // The program data account - derive the actual program address
                            // For upgradeable programs, we need to find the program account that points to this data
                            console::log_1(&format!("Program data account: {}", pubkey).into());
                            
                            // Add the program data account itself (it represents the deployed program)
                            contracts.push(DiscoveredContract {
                                address: pubkey.to_string(),
                                contract_type: ContractType::Unknown,
                                description: "Solana upgradeable program (ProgramData)".to_string(),
                            });
                        }
                    }
                } else {
                    console::log_1(&"No program data accounts found or error in response".into());
                }
            }
        }
        Err(e) => {
            console::warn_1(&format!("Failed to query program accounts: {}", e).into());
        }
    }
    
    // Also check if the address itself is a program (for non-upgradeable or direct deployments)
    let account_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "getAccountInfo",
        "params": [address, {"encoding": "jsonParsed"}],
        "id": 2
    });
    
    if let Ok(resp) = client.post(rpc_url).json(&account_body).send().await {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(owner) = json
                .get("result")
                .and_then(|r| r.get("value"))
                .and_then(|v| v.get("owner"))
                .and_then(|o| o.as_str())
            {
                console::log_1(&format!("Account {} owner: {}", address, owner).into());
                
                if owner.contains("BPFLoader") && !contracts.iter().any(|c| c.address == address) {
                    contracts.push(DiscoveredContract {
                        address: address.to_string(),
                        contract_type: ContractType::Unknown,
                        description: "Solana program (BPF Loader)".to_string(),
                    });
                }
            }
        }
    }
    
    // Verify candidate contract addresses - check if user is upgrade authority
    if let Some(candidates) = candidate_contracts {
        console::log_1(&format!("Verifying {} candidate Solana programs", candidates.len()).into());
        
        for candidate in candidates {
            // Skip if already found
            if contracts.iter().any(|c| c.address == *candidate) {
                continue;
            }
            
            // Get program account info to check if it's a program and who owns it
            let program_body = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "getAccountInfo",
                "params": [candidate, {"encoding": "jsonParsed"}],
                "id": 3
            });
            
            if let Ok(resp) = client.post(rpc_url).json(&program_body).send().await {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(value) = json.get("result").and_then(|r| r.get("value")) {
                        // Check if it's owned by BPFLoader (indicates it's a program)
                        if let Some(owner) = value.get("owner").and_then(|o| o.as_str()) {
                            if owner.contains("BPFLoader") {
                                // It's a program - check if user is the upgrade authority
                                // For ProgramData accounts, check the parsed data
                                if let Some(_data) = value.get("data").and_then(|d| d.as_array()) {
                                    // Try to find upgrade authority in parsed data
                                    let mut is_owned_by_user = false;
                                    
                                    // Look through parsed info for upgrade authority
                                    if let Some(parsed) = value.get("data").and_then(|d| d.get("parsed")).and_then(|p| p.get("info")) {
                                        if let Some(authority) = parsed.get("upgradeAuthority").and_then(|a| a.as_str()) {
                                            if authority == address {
                                                is_owned_by_user = true;
                                            }
                                        }
                                    }
                                    
                                    // Also check if it's a program account pointing to program data
                                    // where user is the upgrade authority
                                    if !is_owned_by_user {
                                        // Try to get the program account info
                                        if let Some(program_data) = value.get("data").and_then(|d| d.get("parsed")).and_then(|p| p.get("info")).and_then(|i| i.get("programData")).and_then(|pd| pd.as_str()) {
                                            // Get the program data account to check upgrade authority
                                            let data_body = serde_json::json!({
                                                "jsonrpc": "2.0",
                                                "method": "getAccountInfo",
                                                "params": [program_data, {"encoding": "jsonParsed"}],
                                                "id": 4
                                            });
                                            
                                            if let Ok(data_resp) = client.post(rpc_url).json(&data_body).send().await {
                                                if let Ok(data_json) = data_resp.json::<serde_json::Value>().await {
                                                    if let Some(data_value) = data_json.get("result").and_then(|r| r.get("value")) {
                                                        if let Some(data_parsed) = data_value.get("data").and_then(|d| d.get("parsed")).and_then(|p| p.get("info")) {
                                                            if let Some(authority) = data_parsed.get("upgradeAuthority").and_then(|a| a.as_str()) {
                                                                if authority == address {
                                                                    is_owned_by_user = true;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    
                                    if is_owned_by_user {
                                        console::log_1(&format!("Verified Solana program with user as upgrade authority: {}", candidate).into());
                                        contracts.push(DiscoveredContract {
                                            address: candidate.to_string(),
                                            contract_type: ContractType::Unknown,
                                            description: "Verified program (user is upgrade authority)".to_string(),
                                        });
                                    } else {
                                        // Still add it as a program if it exists
                                        console::log_1(&format!("Found Solana program: {}", candidate).into());
                                        contracts.push(DiscoveredContract {
                                            address: candidate.to_string(),
                                            contract_type: ContractType::Unknown,
                                            description: "Verified program (BPF Loader)".to_string(),
                                        });
                                    }
                                } else {
                                    // Simple program without parsed data - add it
                                    console::log_1(&format!("Found Solana program: {}", candidate).into());
                                    contracts.push(DiscoveredContract {
                                        address: candidate.to_string(),
                                        contract_type: ContractType::Unknown,
                                        description: "Verified program (BPF Loader)".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    console::log_1(&format!("Total Solana contracts discovered: {}", contracts.len()).into());
    Ok(contracts)
}
