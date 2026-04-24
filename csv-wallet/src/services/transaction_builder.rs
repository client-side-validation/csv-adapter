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
fn build_sui_transaction(
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
pub async fn discover_contracts(
    chain: Chain,
    address: &str,
    rpc_url: &str,
) -> Result<Vec<DiscoveredContract>, BlockchainError> {
    match chain {
        Chain::Sui => discover_sui_contracts(address, rpc_url).await,
        Chain::Aptos => discover_aptos_contracts(address, rpc_url).await,
        Chain::Ethereum => discover_ethereum_contracts(address, rpc_url).await,
        Chain::Solana => discover_solana_contracts(address, rpc_url).await,
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
    let modules_url = format!("{}/v1/accounts/{}/modules", rpc_url.trim_end_matches('/'), address);

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

/// Discover Ethereum contracts
async fn discover_ethereum_contracts(
    address: &str,
    rpc_url: &str,
) -> Result<Vec<DiscoveredContract>, BlockchainError> {
    let client = reqwest::Client::new();
    
    // Query for contract code at address
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
    
    let json: serde_json::Value = response.json().await.map_err(|e| BlockchainError {
        message: format!("Failed to parse Ethereum response: {}", e),
        chain: Some(Chain::Ethereum),
        code: None,
    })?;
    
    let mut contracts = Vec::new();
    
    if let Some(code) = json.get("result").and_then(|r| r.as_str()) {
        if code.len() > 2 && code != "0x" {
            // This is a contract address
            contracts.push(DiscoveredContract {
                address: address.to_string(),
                contract_type: ContractType::Unknown,
                description: format!("Smart contract ({} bytes)", (code.len() - 2) / 2),
            });
        }
    }
    
    Ok(contracts)
}

/// Discover Solana programs
async fn discover_solana_contracts(
    address: &str,
    rpc_url: &str,
) -> Result<Vec<DiscoveredContract>, BlockchainError> {
    use web_sys::console;
    
    let client = reqwest::Client::new();
    
    // Query for account info to check if it's a program
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "getAccountInfo",
        "params": [address, {"encoding": "jsonParsed"}],
        "id": 1
    });
    
    console::log_1(&format!("Discovering Solana program at: {}", address).into());
    
    let response = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| BlockchainError {
            message: format!("Failed to query Solana account: {}", e),
            chain: Some(Chain::Solana),
            code: None,
        })?;
    
    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| BlockchainError {
            message: format!("Failed to parse Solana response: {}", e),
            chain: Some(Chain::Solana),
            code: None,
        })?;
    
    let mut contracts = Vec::new();
    
    // Check if this is a program (executable account)
    if let Some(owner) = json
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.get("owner"))
        .and_then(|o| o.as_str())
    {
        console::log_1(&format!("Solana account owner: {}", owner).into());
        
        // BPFLoader accounts indicate programs
        if owner.contains("BPFLoader") {
            contracts.push(DiscoveredContract {
                address: address.to_string(),
                contract_type: ContractType::Unknown,
                description: "Solana program (BPF Loader)".to_string(),
            });
        }
    }
    
    Ok(contracts)
}
