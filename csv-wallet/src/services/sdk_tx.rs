//! SDK-style transaction builders for Sui and Aptos using BCS encoding
//!
//! Uses the `bcs` crate for proper BCS serialization matching the official SDKs.
//! No wallet adapter required - uses imported private keys directly.

use crate::services::blockchain_service::BlockchainError;
use csv_adapter_core::Chain;
use serde::{Serialize, Deserialize};

// ============ Sui BCS Types ============

/// Sui ObjectID (32 bytes)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ObjectID([u8; 32]);

/// Sui SuiAddress (32 bytes)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SuiAddress([u8; 32]);

/// Sui TransactionDigest (32 bytes)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionDigest([u8; 32]);

/// Sui Move function identifier
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Identifier(String);

/// Sui TypeTag (simplified - just unit for now)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TypeTag {
    Bool,
    U8,
    U64,
    U128,
    Address,
    Signer,
    Vector(Box<TypeTag>),
    Struct(StructTag),
    U16,
    U32,
    U256,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StructTag {
    address: AccountAddress,
    module: Identifier,
    name: Identifier,
    type_params: Vec<TypeTag>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AccountAddress([u8; 32]);

/// Sui CallArg - argument to a move call
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CallArg {
    /// Object argument
    Object(ObjectArg),
    /// Pure argument (BCS encoded bytes)
    Pure(Vec<u8>),
    /// Object vector argument
    ObjVec(Vec<ObjectArg>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ObjectArg {
    /// A Move object that's either from the historic object pool or generated
    ImmOrOwnedObject(ObjectRef),
    /// A Move object from the shared object pool
    SharedObject {
        id: ObjectID,
        initial_shared_version: u64,
        mutable: bool,
    },
    /// A Move object from the Conensus layer
    Receiving(ObjectRef),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ObjectRef(ObjectID, u64, TransactionDigest);

/// Sui Argument - reference to inputs or results
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Argument {
    /// Gas coin argument
    GasCoin,
    /// One of the input objects
    Input(u16),
    /// Reference to a result of a command
    Result(u16),
    /// Reference to a nested result
    NestedResult(u16, u16),
}

/// Sui Command - individual operation in a programmable transaction
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Command {
    /// Move call
    MoveCall(Box<ProgrammableMoveCall>),
    /// Transfer objects
    TransferObjects(Vec<Argument>, Argument),
    /// Split coin
    SplitCoins(Argument, Vec<Argument>),
    /// Merge coins
    MergeCoins(Argument, Vec<Argument>),
    /// Publish
    Publish(Vec<Vec<u8>>, Vec<ObjectID>),
    /// Make move vec
    MakeMoveVec(Option<TypeTag>, Vec<Argument>),
    /// Upgrade
    Upgrade(Vec<Vec<u8>>, Vec<ObjectID>, ObjectID, Argument),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProgrammableMoveCall {
    /// Package ID
    package: ObjectID,
    /// Module name
    module: Identifier,
    /// Function name
    function: Identifier,
    /// Type arguments
    type_arguments: Vec<TypeTag>,
    /// Arguments
    arguments: Vec<Argument>,
}

/// Sui ProgrammableTransaction
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProgrammableTransaction {
    inputs: Vec<CallArg>,
    commands: Vec<Command>,
}

/// Sui TransactionKind
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TransactionKind {
    /// Change epoch transaction
    ChangeEpoch,
    /// Genesis transaction
    Genesis,
    /// Consensus commit prologue
    ConsensusCommitPrologue,
    /// Programmable transaction
    ProgrammableTransaction(ProgrammableTransaction),
    /// Authenticator state update
    AuthenticatorStateUpdate,
    /// Randomness state update
    RandomnessStateUpdate,
    /// End of epoch transaction
    EndOfEpochTransaction,
}

/// Sui GasData
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GasData {
    payment: Vec<ObjectRef>,
    owner: SuiAddress,
    price: u64,
    budget: u64,
}

/// Sui TransactionExpiration
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TransactionExpiration {
    None,
    Epoch(u64),
}

/// Sui TransactionData (the full transaction data structure)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionData {
    kind: TransactionKind,
    sender: SuiAddress,
    gas_data: GasData,
    expiration: TransactionExpiration,
}

/// Build Sui transaction with proper BCS encoding
pub fn build_sui_transaction(
    sender: &str,
    package: &str,
    module: &str,
    function: &str,
    arguments: Vec<Vec<u8>>,
    gas_object_id: &str,
    gas_object_version: u64,
    gas_object_digest: &str,
    gas_budget: u64,
) -> Result<Vec<u8>, BlockchainError> {
    // Parse addresses
    let sender_bytes = parse_sui_address(sender)?;
    let package_bytes = parse_object_id(package)?;
    let gas_id_bytes = parse_object_id(gas_object_id)?;
    let gas_digest_bytes = parse_digest(gas_object_digest)?;
    
    // Build inputs - Pure arguments for each parameter
    let inputs: Vec<CallArg> = arguments.into_iter()
        .map(|arg| CallArg::Pure(arg))
        .collect();
    
    // Build command - MoveCall
    let move_call = ProgrammableMoveCall {
        package: ObjectID(package_bytes),
        module: Identifier(module.to_string()),
        function: Identifier(function.to_string()),
        type_arguments: Vec::new(),
        arguments: (0..inputs.len()).map(|i| Argument::Input(i as u16)).collect(),
    };
    
    let commands = vec![Command::MoveCall(Box::new(move_call))];
    
    // Build ProgrammableTransaction
    let programmable_tx = ProgrammableTransaction { inputs, commands };
    
    // Build TransactionKind
    let kind = TransactionKind::ProgrammableTransaction(programmable_tx);
    
    // Build GasData
    let gas_data = GasData {
        payment: vec![ObjectRef(
            ObjectID(gas_id_bytes),
            gas_object_version,
            TransactionDigest(gas_digest_bytes)
        )],
        owner: SuiAddress(sender_bytes),
        price: 1000,
        budget: gas_budget,
    };
    
    // Build full TransactionData
    let tx_data = TransactionData {
        kind,
        sender: SuiAddress(sender_bytes),
        gas_data,
        expiration: TransactionExpiration::None,
    };
    
    // BCS encode
    bcs::to_bytes(&tx_data).map_err(|e| BlockchainError {
        message: format!("BCS encoding failed: {}", e),
        chain: Some(Chain::Sui),
        code: None,
    })
}

fn parse_sui_address(addr: &str) -> Result<[u8; 32], BlockchainError> {
    let bytes = hex::decode(addr.trim_start_matches("0x"))
        .map_err(|e| BlockchainError {
            message: format!("Invalid Sui address: {}", e),
            chain: Some(Chain::Sui),
            code: None,
        })?;
    if bytes.len() != 32 {
        return Err(BlockchainError {
            message: format!("Sui address must be 32 bytes, got {}", bytes.len()),
            chain: Some(Chain::Sui),
            code: None,
        });
    }
    let mut result = [0u8; 32];
    result.copy_from_slice(&bytes);
    Ok(result)
}

fn parse_object_id(id: &str) -> Result<[u8; 32], BlockchainError> {
    parse_sui_address(id) // Same format
}

fn parse_digest(digest: &str) -> Result<[u8; 32], BlockchainError> {
    let bytes = hex::decode(digest.trim_start_matches("0x"))
        .map_err(|e| BlockchainError {
            message: format!("Invalid digest: {}", e),
            chain: Some(Chain::Sui),
            code: None,
        })?;
    if bytes.len() != 32 {
        return Err(BlockchainError {
            message: format!("Digest must be 32 bytes, got {}", bytes.len()),
            chain: Some(Chain::Sui),
            code: None,
        });
    }
    let mut result = [0u8; 32];
    result.copy_from_slice(&bytes);
    Ok(result)
}

// ============ Aptos BCS Types ============

/// Aptos AccountAddress (32 bytes)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AptosAddress([u8; 32]);

/// Aptos ModuleId
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModuleId {
    address: AptosAddress,
    name: Identifier,
}

/// Aptos EntryFunction
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EntryFunction {
    module: ModuleId,
    function: Identifier,
    ty_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
}

/// Aptos TransactionPayload
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TransactionPayload {
    Script, // Not implemented - placeholder
    EntryFunction(EntryFunction),
    Multisig, // Not implemented - placeholder
}

/// Aptos RawTransaction
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RawTransaction {
    sender: AptosAddress,
    sequence_number: u64,
    payload: TransactionPayload,
    max_gas_amount: u64,
    gas_unit_price: u64,
    expiration_timestamp_secs: u64,
    chain_id: u8,
}

/// Build Aptos transaction with proper BCS encoding
pub fn build_aptos_transaction(
    sender: &str,
    contract: &str,
    module: &str,
    function: &str,
    arguments: Vec<Vec<u8>>,
    sequence_number: u64,
    max_gas_amount: u64,
    gas_unit_price: u64,
) -> Result<Vec<u8>, BlockchainError> {
    // Parse addresses
    let sender_bytes = parse_aptos_address(sender)?;
    let contract_bytes = parse_aptos_address(contract)?;
    
    // Build EntryFunction payload
    let entry_fn = EntryFunction {
        module: ModuleId {
            address: AptosAddress(contract_bytes),
            name: Identifier(module.to_string()),
        },
        function: Identifier(function.to_string()),
        ty_args: Vec::new(),
        args: arguments,
    };
    
    let payload = TransactionPayload::EntryFunction(entry_fn);
    
    // Build RawTransaction
    // Chain ID: 2 = Testnet, 1 = Mainnet
    let raw_tx = RawTransaction {
        sender: AptosAddress(sender_bytes),
        sequence_number,
        payload,
        max_gas_amount,
        gas_unit_price,
        expiration_timestamp_secs: 0, // No expiration
        chain_id: 2, // Testnet
    };
    
    // BCS encode
    bcs::to_bytes(&raw_tx).map_err(|e| BlockchainError {
        message: format!("BCS encoding failed: {}", e),
        chain: Some(Chain::Aptos),
        code: None,
    })
}

fn parse_aptos_address(addr: &str) -> Result<[u8; 32], BlockchainError> {
    let bytes = hex::decode(addr.trim_start_matches("0x"))
        .map_err(|e| BlockchainError {
            message: format!("Invalid Aptos address: {}", e),
            chain: Some(Chain::Aptos),
            code: None,
        })?;
    if bytes.len() != 32 {
        return Err(BlockchainError {
            message: format!("Aptos address must be 32 bytes, got {}", bytes.len()),
            chain: Some(Chain::Aptos),
            code: None,
        });
    }
    let mut result = [0u8; 32];
    result.copy_from_slice(&bytes);
    Ok(result)
}

/// Fetch Sui gas objects for an address
/// 
/// Required before building transactions
pub async fn fetch_sui_gas_objects(
    address: &str,
    rpc_url: &str,
) -> Result<Vec<(String, u64, String)>, BlockchainError> {
    // Query suix_getCoins to find SUI coins for gas
    let client = reqwest::Client::new();
    
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "suix_getCoins",
        "params": [address, null, null, null]
    });
    
    let response = client.post(rpc_url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| BlockchainError {
            message: format!("Failed to fetch gas objects: {}", e),
            chain: Some(Chain::Sui),
            code: None,
        })?;
    
    let json: serde_json::Value = response.json().await
        .map_err(|e| BlockchainError {
            message: format!("Failed to parse gas objects: {}", e),
            chain: Some(Chain::Sui),
            code: None,
        })?;
    
    let mut gas_objects = Vec::new();
    
    if let Some(data) = json.get("result").and_then(|r| r.get("data")) {
        if let Some(array) = data.as_array() {
            for coin in array {
                if let (Some(coin_type), Some(obj_id), Some(_version), Some(digest), Some(balance)) = (
                    coin.get("coinType").and_then(|v| v.as_str()),
                    coin.get("coinObjectId").and_then(|v| v.as_str()),
                    coin.get("version").and_then(|v| v.as_str()),
                    coin.get("digest").and_then(|v| v.as_str()),
                    coin.get("balance").and_then(|v| v.as_str()).and_then(|s| s.parse::<u64>().ok()),
                ) {
                    // Only include SUI coins
                    if coin_type.contains("0x2::sui::SUI") {
                        gas_objects.push((obj_id.to_string(), balance, digest.to_string()));
                    }
                }
            }
        }
    }
    
    Ok(gas_objects)
}

/// Fetch Aptos account sequence number
pub async fn fetch_aptos_sequence(
    address: &str,
    rpc_url: &str,
) -> Result<u64, BlockchainError> {
    let client = reqwest::Client::new();
    let url = format!("{}/accounts/{}", rpc_url.trim_end_matches('/'), address);
    
    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| BlockchainError {
            message: format!("Failed to fetch account: {}", e),
            chain: Some(Chain::Aptos),
            code: None,
        })?;
    
    let json: serde_json::Value = response.json().await
        .map_err(|e| BlockchainError {
            message: format!("Failed to parse account: {}", e),
            chain: Some(Chain::Aptos),
            code: None,
        })?;
    
    json.get("sequence_number")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or_else(|| BlockchainError {
            message: "Sequence number not found".to_string(),
            chain: Some(Chain::Aptos),
            code: None,
        })
}

/// Sign transaction data with Ed25519 (for Sui/Aptos)
pub fn sign_ed25519(
    tx_data: &[u8],
    private_key_hex: &str,
) -> Result<Vec<u8>, BlockchainError> {
    use ed25519_dalek::{Signer, SigningKey};
    
    let key_bytes = hex::decode(private_key_hex.trim_start_matches("0x"))
        .map_err(|e| BlockchainError {
            message: format!("Invalid private key: {}", e),
            chain: None,
            code: None,
        })?;
    
    if key_bytes.len() < 32 {
        return Err(BlockchainError {
            message: format!("Key too short: {} bytes", key_bytes.len()),
            chain: None,
            code: None,
        });
    }
    
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&key_bytes[..32]);
    
    let signing_key = SigningKey::from_bytes(&seed);
    let signature = signing_key.sign(tx_data);
    
    Ok(signature.to_bytes().to_vec())
}
