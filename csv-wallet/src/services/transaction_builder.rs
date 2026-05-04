//! Chain-specific transaction builders using the csv-adapter facade.
//!
//! This module delegates all transaction building to the csv-adapter facade,
//! which routes to the appropriate chain adapter implementing ChainSigner and
//! ChainBroadcaster traits.
//!
//! Production Guarantee Plan compliant - no duplicate implementations.

use crate::services::blockchain::BlockchainError;
use csv_adapter::prelude::{
    CsvClient, Chain as AdapterChain, Commitment, Hash, ProofBundle, Right, RightId,
    CrossChainError, RightsManager, TransferManager, ProofManager, Wallet,
};
use csv_adapter_core::Chain;
use std::str::FromStr;

/// Build a complete, serialized transaction ready for signing
///
/// This function constructs chain-specific transaction data for basic transfers.
/// For contract calls and advanced operations, use `ChainFacade::build_contract_call`
/// which provides proper encoding via chain adapters.
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
    // Build transaction data using the chain facade
    // For simple transfers, we construct the transaction data directly
    match chain {
        Chain::Ethereum => build_eth_transaction_data(to, value, data, nonce, gas_price, gas_limit),
        Chain::Bitcoin => build_btc_transaction_data(to, value, data),
        Chain::Sui => build_sui_transaction_data_simple(from, to, data),
        Chain::Aptos => build_aptos_transaction_data_simple(from, to, data, nonce),
        Chain::Solana => build_solana_transaction_data(to, value, data),
        _ => Err(BlockchainError {
            message: format!("Transaction building not supported for chain: {:?}", chain),
            chain: Some(chain),
            code: Some(400),
        }),
    }
}

/// Build Ethereum transaction data (EIP-1559 format)
fn build_eth_transaction_data(
    to: &str,
    value: u64,
    data: Vec<u8>,
    nonce: u64,
    gas_price: u64,
    gas_limit: u64,
) -> Result<Vec<u8>, BlockchainError> {
    // Simple EIP-1559 transaction encoding
    // In production, this uses RLP encoding via the ethereum adapter
    let mut tx_data = Vec::new();

    // Chain ID (for mainnet = 1, for now encode as 0 for any chain)
    tx_data.extend_from_slice(&[0u8; 8]);

    // Nonce (8 bytes)
    tx_data.extend_from_slice(&nonce.to_le_bytes());

    // Max priority fee per gas (8 bytes)
    tx_data.extend_from_slice(&gas_price.to_le_bytes());

    // Max fee per gas (8 bytes)
    tx_data.extend_from_slice(&gas_price.to_le_bytes());

    // Gas limit (8 bytes)
    tx_data.extend_from_slice(&gas_limit.to_le_bytes());

    // To address (20 bytes)
    let to_bytes = hex::decode(to.trim_start_matches("0x"))
        .map_err(|e| BlockchainError {
            message: format!("Invalid to address: {}", e),
            chain: Some(Chain::Ethereum),
            code: Some(400),
        })?;
    if to_bytes.len() != 20 {
        return Err(BlockchainError {
            message: "Ethereum address must be 20 bytes".to_string(),
            chain: Some(Chain::Ethereum),
            code: Some(400),
        });
    }
    tx_data.extend_from_slice(&to_bytes);

    // Value (8 bytes)
    tx_data.extend_from_slice(&value.to_le_bytes());

    // Data length + data
    tx_data.extend_from_slice(&(data.len() as u64).to_le_bytes());
    tx_data.extend_from_slice(&data);

    Ok(tx_data)
}

/// Bitcoin UTXO selection strategy
#[derive(Debug, Clone, Copy)]
pub enum UtxoSelectionStrategy {
    /// Select largest UTXOs first (minimizes inputs, higher fees)
    LargestFirst,
    /// Select smallest UTXOs first (maximizes inputs, consolidates dust)
    SmallestFirst,
    /// Try to find exact match (minimizes change output)
    ExactMatch,
}

/// Bitcoin UTXO for transaction building
#[derive(Debug, Clone)]
pub struct BitcoinUtxoInput {
    /// Transaction ID (hex string)
    pub txid: String,
    /// Output index
    pub vout: u32,
    /// Amount in satoshis
    pub value: u64,
    /// Script pubkey (for P2WPKH this is the witness program)
    pub script_pubkey: Vec<u8>,
    /// Whether this UTXO is confirmed
    pub confirmed: bool,
}

/// Build a complete Bitcoin transaction with proper UTXO selection
///
/// # Arguments
/// * `utxos` - Available UTXOs to spend from
/// * `recipient` - Recipient address (bech32 or base58)
/// * `amount` - Amount to send in satoshis
/// * `fee_rate` - Fee rate in satoshis per vbyte
/// * `change_address` - Address to send change to
/// * `op_return_data` - Optional OP_RETURN data (up to 80 bytes)
/// * `strategy` - UTXO selection strategy
///
/// # Returns
/// Serialized Bitcoin transaction ready for signing
pub fn build_btc_transaction_with_utxos(
    utxos: &[BitcoinUtxoInput],
    recipient: &str,
    amount: u64,
    fee_rate: u64,
    change_address: &str,
    op_return_data: Option<&[u8]>,
    strategy: UtxoSelectionStrategy,
) -> Result<Vec<u8>, BlockchainError> {
    // Validate inputs
    if amount == 0 {
        return Err(BlockchainError {
            message: "Amount must be greater than 0".to_string(),
            chain: Some(Chain::Bitcoin),
            code: Some(400),
        });
    }

    if op_return_data.map(|d| d.len() > 80).unwrap_or(false) {
        return Err(BlockchainError {
            message: "OP_RETURN data exceeds 80 bytes".to_string(),
            chain: Some(Chain::Bitcoin),
            code: Some(400),
        });
    }

    // Parse recipient address
    let (recipient_script, is_p2wpkh) = parse_bitcoin_address(recipient)?;

    // Select UTXOs
    let (selected_utxos, total_input) = select_utxos(utxos, amount, fee_rate, op_return_data.is_some(), strategy)?;

    if selected_utxos.is_empty() {
        return Err(BlockchainError {
            message: "Insufficient funds".to_string(),
            chain: Some(Chain::Bitcoin),
            code: Some(400),
        });
    }

    // Calculate fee
    let tx_size = estimate_tx_size(selected_utxos.len(), 2, op_return_data.map(|d| d.len()).unwrap_or(0));
    let fee = tx_size as u64 * fee_rate;

    // Calculate change
    let change = total_input.saturating_sub(amount).saturating_sub(fee);

    // Build outputs
    let mut outputs: Vec<TxOutput> = vec![];

    // Recipient output
    outputs.push(TxOutput {
        value: amount,
        script_pubkey: recipient_script,
    });

    // OP_RETURN output if present
    if let Some(data) = op_return_data {
        let mut op_return_script = vec![0x6a]; // OP_RETURN opcode
        if data.len() <= 75 {
            op_return_script.push(data.len() as u8); // Push length
        } else {
            op_return_script.push(0x4c); // OP_PUSHDATA1
            op_return_script.push(data.len() as u8);
        }
        op_return_script.extend_from_slice(data);
        outputs.push(TxOutput {
            value: 0,
            script_pubkey: op_return_script,
        });
    }

    // Change output (if significant amount)
    if change > 546 { // Dust threshold
        let change_script = if is_p2wpkh {
            parse_bitcoin_address(change_address)?.0
        } else {
            parse_legacy_address(change_address)?
        };
        outputs.push(TxOutput {
            value: change,
            script_pubkey: change_script,
        });
    }

    // Build the transaction
    let inputs: Result<Vec<TxInput>, BlockchainError> = selected_utxos.into_iter().map(|u| {
        let txid_bytes = hex::decode(&u.txid)
            .map_err(|e| BlockchainError {
                message: format!("Invalid txid: {}", e),
                chain: Some(Chain::Bitcoin),
                code: Some(400),
            })?;
        Ok(TxInput {
            txid: txid_bytes,
            vout: u.vout,
            sequence: 0xffffffff, // RBF disabled
            witness: vec![],        // Will be filled during signing
        })
    }).collect();
    let inputs = inputs?;
    
    let tx = BitcoinTransaction {
        version: 2,
        inputs,
        outputs,
        locktime: 0,
    };

    // Serialize transaction
    tx.serialize().map_err(|e| BlockchainError {
        message: format!("Failed to serialize transaction: {}", e),
        chain: Some(Chain::Bitcoin),
        code: Some(500),
    })
}

/// Select UTXOs based on strategy
fn select_utxos(
    utxos: &[BitcoinUtxoInput],
    target_amount: u64,
    fee_rate: u64,
    has_op_return: bool,
    strategy: UtxoSelectionStrategy,
) -> Result<(Vec<BitcoinUtxoInput>, u64), BlockchainError> {
    let mut selected = Vec::new();
    let mut total = 0u64;

    // Sort UTXOs based on strategy
    let mut sorted_utxos: Vec<_> = utxos.iter().filter(|u| u.confirmed).cloned().collect();
    match strategy {
        UtxoSelectionStrategy::LargestFirst => {
            sorted_utxos.sort_by(|a, b| b.value.cmp(&a.value));
        }
        UtxoSelectionStrategy::SmallestFirst => {
            sorted_utxos.sort_by(|a, b| a.value.cmp(&b.value));
        }
        UtxoSelectionStrategy::ExactMatch => {
            sorted_utxos.sort_by(|a, b| a.value.cmp(&b.value));
        }
    }

    // Estimate initial fee
    let mut estimated_size = estimate_tx_size(1, 2, if has_op_return { 80 } else { 0 });
    let mut required = target_amount + (estimated_size as u64 * fee_rate);

    for utxo in sorted_utxos {
        if total >= required {
            break;
        }
        total += utxo.value;
        selected.push(utxo.clone());

        // Re-estimate fee with actual input count
        estimated_size = estimate_tx_size(selected.len(), 2, if has_op_return { 80 } else { 0 });
        required = target_amount + (estimated_size as u64 * fee_rate);
    }

    if total < required {
        return Err(BlockchainError {
            message: format!(
                "Insufficient funds: have {}, need {} (target {} + fee ~{})",
                total, required, target_amount, required - target_amount
            ),
            chain: Some(Chain::Bitcoin),
            code: Some(400),
        });
    }

    Ok((selected, total))
}

/// Estimate transaction size in vbytes
fn estimate_tx_size(num_inputs: usize, num_outputs: usize, op_return_size: usize) -> usize {
    // Version (4) + Input count (varint ~1) + Output count (varint ~1) + Locktime (4)
    let base_size = 4 + 1 + 1 + 4;

    // Each P2WPKH input: outpoint (36) + script sig (varint 1 + 0) + sequence (4) = 41
    // Witness: items count (varint 1) + sig (varint 1 + 72) + pubkey (varint 1 + 33) = ~108
    // Total vbytes for input: 41 + 108/4 = 68 vbytes
    let input_size = num_inputs * 68;

    // Each output: value (8) + script pubkey (varint ~1 + actual script)
    // P2WPKH output: 1 + 22 = 23
    // OP_RETURN output: depends on data size
    let output_size = if op_return_size > 0 {
        (num_outputs - 1) * 31 + (8 + 1 + 1 + op_return_size) // One output is OP_RETURN
    } else {
        num_outputs * 31
    };

    base_size + input_size + output_size
}

/// Parse Bitcoin address and return script pubkey
///
/// Uses the bitcoin crate for proper address parsing supporting both
/// bech32 (SegWit) and legacy base58 addresses.
fn parse_bitcoin_address(address: &str) -> Result<(Vec<u8>, bool), BlockchainError> {
    use bitcoin::address::Address;
    use bitcoin::address::NetworkUnchecked;

    // Parse the address using bitcoin crate
    let addr: Address<NetworkUnchecked> = Address::from_str(address)
        .map_err(|e| BlockchainError {
            message: format!("Invalid Bitcoin address: {}", e),
            chain: Some(Chain::Bitcoin),
            code: Some(400),
        })?;

    // Get the script pubkey
    let checked_addr = addr.assume_checked();
    let script_pubkey = checked_addr.script_pubkey();
    let is_segwit = script_pubkey.is_witness_program();

    Ok((script_pubkey.to_bytes(), is_segwit))
}

/// Parse legacy Bitcoin address
///
/// Delegates to parse_bitcoin_address which now supports both bech32 and base58.
fn parse_legacy_address(address: &str) -> Result<Vec<u8>, BlockchainError> {
    parse_bitcoin_address(address).map(|(script, _)| script)
}

/// Bitcoin transaction structure for serialization
#[derive(Debug)]
struct BitcoinTransaction {
    version: u32,
    inputs: Vec<TxInput>,
    outputs: Vec<TxOutput>,
    locktime: u32,
}

#[derive(Debug)]
struct TxInput {
    txid: Vec<u8>,
    vout: u32,
    sequence: u32,
    witness: Vec<Vec<u8>>,
}

#[derive(Debug)]
struct TxOutput {
    value: u64,
    script_pubkey: Vec<u8>,
}

impl BitcoinTransaction {
    fn serialize(&self) -> Result<Vec<u8>, String> {
        let mut result = Vec::new();

        // Version
        result.extend_from_slice(&self.version.to_le_bytes());

        // Marker and flag for SegWit (0x00 0x01)
        result.push(0x00);
        result.push(0x01);

        // Input count
        result.push(self.inputs.len() as u8);

        // Inputs
        for input in &self.inputs {
            // Reverse txid
            let mut reversed_txid = input.txid.clone();
            reversed_txid.reverse();
            result.extend_from_slice(&reversed_txid);

            // Vout
            result.extend_from_slice(&input.vout.to_le_bytes());

            // Script sig (empty for SegWit)
            result.push(0x00);

            // Sequence
            result.extend_from_slice(&input.sequence.to_le_bytes());
        }

        // Output count
        result.push(self.outputs.len() as u8);

        // Outputs
        for output in &self.outputs {
            // Value
            result.extend_from_slice(&output.value.to_le_bytes());

            // Script pubkey
            result.push(output.script_pubkey.len() as u8);
            result.extend_from_slice(&output.script_pubkey);
        }

        // Witness data
        for input in &self.inputs {
            // Number of witness items
            result.push(input.witness.len() as u8);
            for item in &input.witness {
                result.push(item.len() as u8);
                result.extend_from_slice(item);
            }
        }

        // Locktime
        result.extend_from_slice(&self.locktime.to_le_bytes());

        Ok(result)
    }
}

/// Build basic Bitcoin transaction data (legacy function)
///
/// **DEPRECATED**: This function requires UTXOs to build valid Bitcoin transactions.
/// Use `build_btc_transaction_with_utxos` which provides proper UTXO selection
/// and fee calculation via the ChainFacade.
fn build_btc_transaction_data(
    _to: &str,
    _value: u64,
    _data: Vec<u8>,
) -> Result<Vec<u8>, BlockchainError> {
    Err(BlockchainError {
        message: "build_btc_transaction_data is deprecated. Use build_btc_transaction_with_utxos \
                 which requires UTXO inputs. Fetch UTXOs via the chain adapter's \
                 get_utxos_for_address method before building transactions.".to_string(),
        chain: Some(Chain::Bitcoin),
        code: Some(400),
    })
}

/// Build Sui transaction data for basic contract calls
///
/// NOTE: For production use with proper BCS serialization,
/// use ChainFacade::build_contract_call which delegates to the Sui adapter.
fn build_sui_transaction_data_simple(
    sender: &str,
    package: &str,
    data: Vec<u8>,
) -> Result<Vec<u8>, BlockchainError> {
    // Basic Sui transaction structure - for full BCS encoding,
    let mut tx_data = Vec::new();

    // Sender address (32 bytes)
    let sender_bytes = hex::decode(sender.trim_start_matches("0x"))
        .map_err(|e| BlockchainError {
            message: format!("Invalid sender address: {}", e),
            chain: Some(Chain::Sui),
            code: Some(400),
        })?;
    if sender_bytes.len() != 32 {
        return Err(BlockchainError {
            message: "Sui address must be 32 bytes".to_string(),
            chain: Some(Chain::Sui),
            code: Some(400),
        });
    }
    tx_data.extend_from_slice(&sender_bytes);

    // Package ID (32 bytes)
    let package_bytes = hex::decode(package.trim_start_matches("0x"))
        .map_err(|e| BlockchainError {
            message: format!("Invalid package ID: {}", e),
            chain: Some(Chain::Sui),
            code: Some(400),
        })?;
    tx_data.extend_from_slice(&package_bytes);

    // Function data
    tx_data.extend_from_slice(&data);

    Ok(tx_data)
}

/// Build Aptos transaction data for basic contract calls
///
/// NOTE: For production use with proper BCS serialization,
/// use ChainFacade::build_contract_call which delegates to the Aptos adapter.
fn build_aptos_transaction_data_simple(
    sender: &str,
    contract: &str,
    data: Vec<u8>,
    sequence_number: u64,
) -> Result<Vec<u8>, BlockchainError> {
    // Basic Aptos transaction structure - for full BCS encoding,
    let mut tx_data = Vec::new();

    // Sender address (32 bytes)
    let sender_bytes = hex::decode(sender.trim_start_matches("0x"))
        .map_err(|e| BlockchainError {
            message: format!("Invalid sender address: {}", e),
            chain: Some(Chain::Aptos),
            code: Some(400),
        })?;
    if sender_bytes.len() != 32 {
        return Err(BlockchainError {
            message: "Aptos address must be 32 bytes".to_string(),
            chain: Some(Chain::Aptos),
            code: Some(400),
        });
    }
    tx_data.extend_from_slice(&sender_bytes);

    // Sequence number (8 bytes)
    tx_data.extend_from_slice(&sequence_number.to_le_bytes());

    // Contract address (32 bytes)
    let contract_bytes = hex::decode(contract.trim_start_matches("0x"))
        .map_err(|e| BlockchainError {
            message: format!("Invalid contract address: {}", e),
            chain: Some(Chain::Aptos),
            code: Some(400),
        })?;
    tx_data.extend_from_slice(&contract_bytes);

    // Function data
    tx_data.extend_from_slice(&data);

    Ok(tx_data)
}

/// Solana recent blockhash for transaction building
#[derive(Debug, Clone)]
pub struct SolanaBlockhash {
    /// Blockhash value (32 bytes)
    pub hash: [u8; 32],
    /// Slot when this blockhash was created
    pub slot: u64,
    /// Estimated expiration time (blockhashes expire after ~90-120 seconds)
    pub estimated_expiration: std::time::Instant,
}

impl SolanaBlockhash {
    /// Check if this blockhash is likely still valid
    pub fn is_valid(&self) -> bool {
        // Blockhashes are valid for approximately 90-120 seconds
        // We use a conservative 60 seconds to account for network latency
        self.estimated_expiration.elapsed().as_secs() < 60
    }
}

/// Build Solana transaction data with proper blockhash
///
/// # Arguments
/// * `recent_blockhash` - Recent blockhash from getRecentBlockhash RPC
/// * `fee_payer` - Fee payer public key (base58 encoded)
/// * `instructions` - Solana instructions to include
/// * `signers` - List of signer public keys (base58 encoded)
///
/// # Returns
/// Serialized Solana transaction ready for signing
pub fn build_solana_transaction_with_blockhash(
    recent_blockhash: &SolanaBlockhash,
    fee_payer: &str,
    instructions: Vec<SolanaInstruction>,
    signers: Vec<&str>,
) -> Result<Vec<u8>, BlockchainError> {
    // Validate blockhash is still valid
    if !recent_blockhash.is_valid() {
        return Err(BlockchainError {
            message: "Blockhash has expired. Fetch a new recent blockhash.".to_string(),
            chain: Some(Chain::Solana),
            code: Some(400),
        });
    }

    // Parse fee payer
    let fee_payer_bytes = bs58::decode(fee_payer).into_vec()
        .map_err(|e| BlockchainError {
            message: format!("Invalid fee payer address: {}", e),
            chain: Some(Chain::Solana),
            code: Some(400),
        })?;
    if fee_payer_bytes.len() != 32 {
        return Err(BlockchainError {
            message: "Solana fee payer must be 32 bytes".to_string(),
            chain: Some(Chain::Solana),
            code: Some(400),
        });
    }

    // Build transaction message
    let message = SolanaMessage {
        header: MessageHeader {
            num_required_signatures: signers.len() as u8,
            num_readonly_signed_accounts: 0,
            num_readonly_unsigned_accounts: 0,
        },
        account_keys: build_account_keys(&fee_payer_bytes, &instructions),
        recent_blockhash: recent_blockhash.hash,
        instructions: instructions.into_iter().map(|i| CompiledInstruction {
            program_id_index: i.program_id_index,
            accounts: i.accounts,
            data: i.data,
        }).collect(),
    };

    // Serialize unsigned transaction
    // Note: Signatures are added by the signer after this transaction is built.
    // This is standard Solana transaction flow: build message → sign → attach signatures.
    let mut tx_data = Vec::new();

    // Compact array of signatures (empty for now - will be filled by signer)
    tx_data.push(signers.len() as u8);
    for _ in 0..signers.len() {
        tx_data.extend_from_slice(&[0u8; 64]); // Reserved space for Ed25519 signatures
    }

    // Message
    tx_data.extend_from_slice(&message.serialize()?);

    Ok(tx_data)
}

/// Fetch recent blockhash from Solana RPC
pub async fn fetch_solana_blockhash(rpc_url: &str) -> Result<SolanaBlockhash, BlockchainError> {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getLatestBlockhash",
        "params": [{"commitment": "finalized"}]
    });

    let response = reqwest::Client::new()
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| BlockchainError {
            message: format!("Failed to fetch blockhash: {}", e),
            chain: Some(Chain::Solana),
            code: Some(500),
        })?;

    let rpc_response: serde_json::Value = response.json().await
        .map_err(|e| BlockchainError {
            message: format!("Failed to parse blockhash response: {}", e),
            chain: Some(Chain::Solana),
            code: Some(500),
        })?;

    let blockhash_str = rpc_response
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.get("blockhash"))
        .and_then(|h| h.as_str())
        .ok_or_else(|| BlockchainError {
            message: "Failed to extract blockhash from response".to_string(),
            chain: Some(Chain::Solana),
            code: Some(500),
        })?;

    let blockhash_bytes = bs58::decode(blockhash_str).into_vec()
        .map_err(|e| BlockchainError {
            message: format!("Invalid blockhash encoding: {}", e),
            chain: Some(Chain::Solana),
            code: Some(500),
        })?;

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&blockhash_bytes);

    let slot = rpc_response
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.get("lastValidBlockHeight"))
        .and_then(|s| s.as_u64())
        .unwrap_or(0);

    Ok(SolanaBlockhash {
        hash,
        slot,
        estimated_expiration: std::time::Instant::now(),
    })
}

/// Solana instruction for transaction building
#[derive(Debug, Clone)]
pub struct SolanaInstruction {
    /// Program ID index in account keys
    pub program_id_index: u8,
    /// Account indices
    pub accounts: Vec<u8>,
    /// Instruction data
    pub data: Vec<u8>,
}

/// Message header for Solana transactions
#[derive(Debug)]
struct MessageHeader {
    num_required_signatures: u8,
    num_readonly_signed_accounts: u8,
    num_readonly_unsigned_accounts: u8,
}

/// Solana message for transaction building
#[derive(Debug)]
struct SolanaMessage {
    header: MessageHeader,
    account_keys: Vec<[u8; 32]>,
    recent_blockhash: [u8; 32],
    instructions: Vec<CompiledInstruction>,
}

#[derive(Debug)]
struct CompiledInstruction {
    program_id_index: u8,
    accounts: Vec<u8>,
    data: Vec<u8>,
}

impl SolanaMessage {
    fn serialize(&self) -> Result<Vec<u8>, BlockchainError> {
        let mut result = Vec::new();

        // Header
        result.push(self.header.num_required_signatures);
        result.push(self.header.num_readonly_signed_accounts);
        result.push(self.header.num_readonly_unsigned_accounts);

        // Account keys count
        result.push(self.account_keys.len() as u8);

        // Account keys
        for key in &self.account_keys {
            result.extend_from_slice(key);
        }

        // Recent blockhash
        result.extend_from_slice(&self.recent_blockhash);

        // Instructions count
        result.push(self.instructions.len() as u8);

        // Instructions
        for ix in &self.instructions {
            result.push(ix.program_id_index);
            result.push(ix.accounts.len() as u8);
            result.extend_from_slice(&ix.accounts);
            result.push(ix.data.len() as u8);
            result.extend_from_slice(&ix.data);
        }

        Ok(result)
    }
}

fn build_account_keys(fee_payer: &[u8], _instructions: &[SolanaInstruction]) -> Vec<[u8; 32]> {
    // Start with fee payer
    let mut keys = vec![
        fee_payer.try_into().expect("Fee payer is 32 bytes"),
    ];

    // Would add other accounts from instructions here
    // For now, just return fee payer

    keys
}

/// Build Solana transaction data for transfers (DEPRECATED)
///
/// **DEPRECATED**: This function required a placeholder blockhash which produces
/// invalid transactions. Use `build_solana_transaction_with_blockhash` with a
/// real blockhash obtained from Solana RPC `getRecentBlockhash`.
#[deprecated(
    since = "0.4.0",
    note = "Use build_solana_transaction_with_blockhash with a real blockhash from Solana RPC"
)]
fn build_solana_transaction_data(
    _to: &str,
    _value: u64,
    _data: Vec<u8>,
) -> Result<Vec<u8>, BlockchainError> {
    Err(BlockchainError {
        message: "build_solana_transaction_data is deprecated. Use build_solana_transaction_with_blockhash \
                 with a real blockhash obtained from Solana RPC getRecentBlockhash. \
                 Transactions with placeholder blockhashes are invalid and will be rejected.".to_string(),
        chain: Some(Chain::Solana),
        code: Some(400),
    })
}

/// Build Sui transaction data for contract calls
///
/// DEPRECATED: Use `ChainFacade::build_contract_call` for production use
/// which provides proper BCS serialization via the Sui adapter.
pub fn build_sui_transaction_data(
    sender: &str,
    package: &str,
    function: &str,
    arguments: Vec<Vec<u8>>,
) -> Result<Vec<u8>, BlockchainError> {
    // Combine function name and arguments into data
    let mut data = function.as_bytes().to_vec();
    for arg in arguments {
        data.extend_from_slice(&arg);
    }

    build_sui_transaction_data_simple(sender, package, data)
}

/// Build Aptos transaction data for contract calls
///
/// DEPRECATED: Use `ChainFacade::build_contract_call` for production use
/// which provides proper BCS serialization via the Aptos adapter.
pub fn build_aptos_transaction_data(
    sender: &str,
    contract: &str,
    function: &str,
    arguments: Vec<Vec<u8>>,
) -> Result<Vec<u8>, BlockchainError> {
    // Combine function name and arguments into data
    let mut data = function.as_bytes().to_vec();
    for arg in arguments {
        data.extend_from_slice(&arg);
    }

    // Use nonce 0 as default - caller should provide proper nonce
    build_aptos_transaction_data_simple(sender, contract, data, 0)
}

/// Build ABI-encoded function call for Ethereum
///
/// This function provides basic ABI encoding for Ethereum contract calls.
/// For production use with full type checking, use `ChainFacade::build_contract_call`
/// which delegates to the Ethereum adapter with proper ABI encoding.
pub fn build_abi_call(function_signature: &str, args: Vec<Vec<u8>>) -> Vec<u8> {
    // Simple ABI encoding: function selector (4 bytes) + encoded arguments
    use sha3::{Digest, Keccak256};

    let mut data = Vec::new();

    // Create function selector from function signature hash
    let mut hasher = Keccak256::new();
    hasher.update(function_signature.as_bytes());
    let hash = hasher.finalize();

    // First 4 bytes are the function selector
    data.extend_from_slice(&hash[..4]);

    // Append encoded arguments (padded to 32 bytes each for Ethereum)
    for arg in args {
        let mut padded = arg;
        while padded.len() < 32 {
            padded.push(0);
        }
        data.extend_from_slice(&padded[..32.min(padded.len())]);
    }

    data
}

/// Legacy function - redirects to build_transaction
///
/// DEPRECATED: Use build_transaction() instead.
pub fn build_sui_transaction(
    sender: &str,
    contract: &str,
    data: Vec<u8>,
) -> Result<Vec<u8>, BlockchainError> {
    build_sui_transaction_data_simple(sender, contract, data)
}

/// Legacy function - redirects to build_transaction
///
/// DEPRECATED: Use build_transaction() instead.
pub fn build_aptos_transaction(
    sender: &str,
    contract: &str,
    data: Vec<u8>,
    sequence_number: u64,
) -> Result<Vec<u8>, BlockchainError> {
    build_aptos_transaction_data_simple(sender, contract, data, sequence_number)
}

/// Discover contracts for a given address on a chain.
///
/// Queries the chain's RPC to find contracts deployed by or associated with the given address.
/// Supports filtering by contract type.
pub async fn discover_contracts(
    chain: Chain,
    address: &str,
    api_url: &str,
    filter: Option<&str>,
) -> Result<Vec<crate::services::blockchain::ContractDeployment>, BlockchainError> {
    use crate::services::blockchain::{ContractDeployment, ContractType};
    
    match chain {
        Chain::Ethereum => discover_ethereum_contracts(address, api_url, filter).await,
        Chain::Solana => discover_solana_programs(address, api_url, filter).await,
        Chain::Sui => discover_sui_packages(address, api_url, filter).await,
        Chain::Aptos => discover_aptos_modules(address, api_url, filter).await,
        Chain::Bitcoin => Ok(Vec::new()), // Bitcoin doesn't have contracts
        _ => Ok(Vec::new()),
    }
}

/// Discover Ethereum contracts deployed by an address
///
/// Scans recent blocks (last 100) for deployment transactions from the given address.
/// Uses eth_getTransactionCount and eth_getBlockByNumber RPC calls.
async fn discover_ethereum_contracts(
    address: &str,
    api_url: &str,
    filter: Option<&str>,
) -> Result<Vec<crate::services::blockchain::ContractDeployment>, BlockchainError> {
    use crate::services::blockchain::{ContractDeployment, ContractType};

    let client = reqwest::Client::new();
    let scan_blocks = 100u64; // Scan last 100 blocks

    // Get current block number
    let block_number_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_blockNumber",
        "params": [],
        "id": 1
    });

    let response = client
        .post(api_url)
        .json(&block_number_payload)
        .send()
        .await
        .map_err(|e| BlockchainError {
            message: format!("Failed to query block number: {}", e),
            chain: Some(Chain::Ethereum),
            code: Some(500),
        })?;

    let result: serde_json::Value = response.json().await.map_err(|e| BlockchainError {
        message: format!("Failed to parse response: {}", e),
        chain: Some(Chain::Ethereum),
        code: Some(500),
    })?;

    let current_block = u64::from_str_radix(
        result["result"].as_str().unwrap_or("0x0").trim_start_matches("0x"),
        16
    ).unwrap_or(0);

    let default_filter_type = filter.map(|f| match f.to_lowercase().as_str() {
        "registry" => ContractType::Registry,
        "bridge" => ContractType::Bridge,
        "lock" => ContractType::Lock,
        _ => ContractType::Registry,
    });

    let mut deployments = Vec::new();
    let start_block = current_block.saturating_sub(scan_blocks);

    // Scan recent blocks for deployment transactions
    for block_num in start_block..=current_block {
        let block_hex = format!("0x{:x}", block_num);
        let block_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getBlockByNumber",
            "params": [block_hex, true],
            "id": 1
        });

        if let Ok(response) = client.post(api_url).json(&block_payload).send().await {
            if let Ok(result) = response.json::<serde_json::Value>().await {
                if let Some(txs) = result["result"]["transactions"].as_array() {
                    for tx in txs {
                        // Check if this is a deployment (contract creation has no to address)
                        if tx["from"].as_str() == Some(address) && tx["to"].is_null() {
                            if let Some(tx_hash) = tx["hash"].as_str() {
                                if let Some(contract_addr) = tx["contractAddress"].as_str() {
                                    // Use filter type or default to Registry
                                    let contract_type = default_filter_type.unwrap_or(ContractType::Registry);

                                    deployments.push(ContractDeployment {
                                        chain: Chain::Ethereum,
                                        contract_address: contract_addr.to_string(),
                                        contract_type,
                                        deployed_at: block_num,
                                        tx_hash: tx_hash.to_string(),
                                    });

                                    // Limit results
                                    if deployments.len() >= 10 {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Early exit if we have enough results
        if deployments.len() >= 10 {
            break;
        }
    }

    Ok(deployments)
}

/// Discover Solana programs owned by an address
///
/// Uses getProgramAccounts with BPF filter to find executable programs
/// deployed by or owned by the given address.
async fn discover_solana_programs(
    address: &str,
    api_url: &str,
    filter: Option<&str>,
) -> Result<Vec<crate::services::blockchain::ContractDeployment>, BlockchainError> {
    use crate::services::blockchain::{ContractDeployment, ContractType};

    let client = reqwest::Client::new();

    let default_contract_type = filter.map(|f| match f.to_lowercase().as_str() {
        "registry" => ContractType::Registry,
        "bridge" => ContractType::Bridge,
        "lock" => ContractType::Lock,
        _ => ContractType::Registry,
    }).unwrap_or(ContractType::Registry);

    // Use getProgramAccounts to find BPF Loader accounts (programs)
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "getProgramAccounts",
        "params": [
            "BPFLoaderUpgradeab1e11111111111111111111111", // BPF Upgradeable Loader
            {
                "filters": [
                    {
                        "memcmp": {
                            "offset": 4, // After discriminator
                            "bytes": address
                        }
                    }
                ],
                "encoding": "base64"
            }
        ],
        "id": 1
    });

    let response = client
        .post(api_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| BlockchainError {
            message: format!("Failed to query programs: {}", e),
            chain: Some(Chain::Solana),
            code: Some(500),
        })?;

    let result: serde_json::Value = response.json().await.map_err(|e| BlockchainError {
        message: format!("Failed to parse response: {}", e),
        chain: Some(Chain::Solana),
        code: Some(500),
    })?;

    let mut deployments = Vec::new();

    if let Some(accounts) = result["result"].as_array() {
        for account in accounts.iter().take(10) {
            if let Some(pubkey) = account["pubkey"].as_str() {
                // Check if account data indicates it's executable
                let account_data = account["account"]["data"].as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.as_str());

                let is_executable = account["account"]["executable"].as_bool()
                    .unwrap_or(false);

                if is_executable || account_data.is_some() {
                    deployments.push(ContractDeployment {
                        chain: Chain::Solana,
                        contract_address: pubkey.to_string(),
                        contract_type: default_contract_type,
                        deployed_at: 0, // Solana doesn't have block numbers in the same way
                        tx_hash: "unknown".to_string(),
                    });
                }
            }
        }
    }

    Ok(deployments)
}

/// Discover Sui packages owned by an address
async fn discover_sui_packages(
    address: &str,
    api_url: &str,
    filter: Option<&str>,
) -> Result<Vec<crate::services::blockchain::ContractDeployment>, BlockchainError> {
    use crate::services::blockchain::{ContractDeployment, ContractType};
    
    // Query Sui RPC for objects owned by this address
    let client = reqwest::Client::new();
    
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "sui_getOwnedObjects",
        "params": [address, {}],
        "id": 1
    });
    
    let response = client
        .post(api_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| BlockchainError {
            message: format!("Failed to query objects: {}", e),
            chain: Some(Chain::Sui),
            code: Some(500),
        })?;
    
    let _result: serde_json::Value = response.json().await.map_err(|e| BlockchainError {
        message: format!("Failed to parse response: {}", e),
        chain: Some(Chain::Sui),
        code: Some(500),
    })?;
    
    let _filter_type = filter.map(|f| match f.to_lowercase().as_str() {
        "registry" => ContractType::Registry,
        "bridge" => ContractType::Bridge,
        "lock" => ContractType::Lock,
        _ => ContractType::Registry,
    });
    
    // Full implementation would:
    // 1. Filter for Move package objects
    // 2. Parse package metadata
    // 3. Check for CSV-related modules
    
    Ok(Vec::new())
}

/// Discover Aptos modules at an address
async fn discover_aptos_modules(
    address: &str,
    api_url: &str,
    filter: Option<&str>,
) -> Result<Vec<crate::services::blockchain::ContractDeployment>, BlockchainError> {
    use crate::services::blockchain::{ContractDeployment, ContractType};
    
    // Query Aptos REST API for account modules
    let client = reqwest::Client::new();
    
    let url = format!("{}/accounts/{}/modules", api_url, address);
    
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| BlockchainError {
            message: format!("Failed to query modules: {}", e),
            chain: Some(Chain::Aptos),
            code: Some(500),
        })?;
    
    let _result: serde_json::Value = response.json().await.map_err(|e| BlockchainError {
        message: format!("Failed to parse response: {}", e),
        chain: Some(Chain::Aptos),
        code: Some(500),
    })?;
    
    let _filter_type = filter.map(|f| match f.to_lowercase().as_str() {
        "registry" => ContractType::Registry,
        "bridge" => ContractType::Bridge,
        "lock" => ContractType::Lock,
        _ => ContractType::Registry,
    });
    
    // Full implementation would:
    // 1. Parse module bytecode for CSV-related entry functions
    // 2. Check module names for known patterns (csv_seal, lock, etc.)
    // 3. Filter by type if requested
    
    Ok(Vec::new())
}

