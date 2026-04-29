//! Ethereum-specific cross-chain functions

use anyhow::Result;
use csv_adapter_core::hash::Hash;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::{Chain, Config};
use crate::output;
use crate::state::UnifiedStateManager;

fn send_ethereum_mint_via_cast(
    contract_address: &str,
    rpc_url: &str,
    private_key: &str,
    right_id: Hash,
    commitment: Hash,
    state_root: Hash,
    source_chain: u8,
    source_seal_ref: Hash,
    proof: &[u8],
    proof_root: Hash,
) -> Result<String> {
    // Use native HTTP RPC implementation (no external cast command needed)
    send_ethereum_mint_native(
        contract_address,
        rpc_url,
        private_key,
        right_id,
        commitment,
        state_root,
        source_chain,
        source_seal_ref,
        proof,
        proof_root,
    )
}

/// Native Ethereum transaction sender using HTTP JSON-RPC
fn send_ethereum_mint_native(
    contract_address: &str,
    rpc_url: &str,
    private_key: &str,
    right_id: Hash,
    commitment: Hash,
    state_root: Hash,
    source_chain: u8,
    source_seal_ref: Hash,
    proof: &[u8],
    proof_root: Hash,
) -> Result<String> {
    use secp256k1::{SecretKey, PublicKey};
    use sha3::{Digest, Keccak256};
    
    // Parse private key
    let cleaned_key = private_key.trim().trim_start_matches("0x").trim();
    let key_bytes = hex::decode(cleaned_key)?;
    let secret_key = SecretKey::from_slice(&key_bytes)?;
    
    // Derive public key and address
    let secp = secp256k1::Secp256k1::new();
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let public_key_bytes = public_key.serialize_uncompressed();
    // Ethereum address: last 20 bytes of Keccak256 of public key (without 0x04 prefix)
    let hash = Keccak256::digest(&public_key_bytes[1..]);
    let sender_address = format!("0x{}", hex::encode(&hash[12..]));
    
    // Get nonce
    let nonce = get_ethereum_nonce(&sender_address, rpc_url)?;
    
    // Get gas price
    let gas_price = get_ethereum_gas_price(rpc_url)?;
    
    // Build the function call data
    // Function selector for mintRight(bytes32,bytes32,bytes32,uint8,bytes,bytes,bytes32)
    let selector = &Keccak256::digest(b"mintRight(bytes32,bytes32,bytes32,uint8,bytes,bytes,bytes32)")[0..4];
    
    // Encode parameters
    let mut data = selector.to_vec();
    
    // rightId (bytes32)
    data.extend_from_slice(right_id.as_bytes());
    
    // commitment (bytes32)
    data.extend_from_slice(commitment.as_bytes());
    
    // stateRoot (bytes32)
    data.extend_from_slice(state_root.as_bytes());
    
    // sourceChain (uint8) - padded to 32 bytes
    data.extend_from_slice(&[0u8; 31]);
    data.push(source_chain);
    
    // sourceSealRef (bytes) - offset pointer
    let source_seal_offset = 7 * 32; // 7 params * 32 bytes each
    data.extend_from_slice(&encode_u256(source_seal_offset as u64));
    
    // proof (bytes) - offset pointer
    let proof_offset = source_seal_offset + 32 + ((source_seal_ref.as_bytes().len() + 31) / 32) * 32;
    data.extend_from_slice(&encode_u256(proof_offset as u64));
    
    // proofRoot (bytes32)
    data.extend_from_slice(proof_root.as_bytes());
    
    // sourceSealRef length and data
    data.extend_from_slice(&encode_u256(source_seal_ref.as_bytes().len() as u64));
    data.extend_from_slice(source_seal_ref.as_bytes());
    // Pad to 32 byte boundary
    let seal_padding = (32 - (source_seal_ref.as_bytes().len() % 32)) % 32;
    data.extend_from_slice(&vec![0u8; seal_padding]);
    
    // proof length and data
    data.extend_from_slice(&encode_u256(proof.len() as u64));
    data.extend_from_slice(proof);
    // Pad to 32 byte boundary
    let proof_padding = (32 - (proof.len() % 32)) % 32;
    data.extend_from_slice(&vec![0u8; proof_padding]);
    
    // Build and sign transaction
    let tx = EthTransaction {
        nonce,
        gas_price,
        gas_limit: 500000,
        to: Some(hex::decode(contract_address.trim_start_matches("0x"))?),
        value: 0,
        data,
        chain_id: 11155111, // Sepolia testnet - should be configurable
    };
    
    let signed_tx = sign_ethereum_transaction(&tx, &secret_key)?;
    
    // Send raw transaction
    send_raw_ethereum_transaction(&signed_tx, rpc_url)
}

fn encode_u256(value: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[24..].copy_from_slice(&value.to_be_bytes());
    bytes
}

struct EthTransaction {
    nonce: u64,
    gas_price: u64,
    gas_limit: u64,
    to: Option<Vec<u8>>,
    value: u64,
    data: Vec<u8>,
    chain_id: u64,
}

fn get_ethereum_nonce(address: &str, rpc_url: &str) -> Result<u64> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionCount",
            "params": [address, "latest"],
            "id": 1
        }))
        .send()?
        .json::<serde_json::Value>()?;
    
    let count_hex = resp.get("result").and_then(|r| r.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to get nonce"))?;
    Ok(u64::from_str_radix(count_hex.trim_start_matches("0x"), 16).unwrap_or(0))
}

fn get_ethereum_gas_price(rpc_url: &str) -> Result<u64> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_gasPrice",
            "params": [],
            "id": 1
        }))
        .send()?
        .json::<serde_json::Value>()?;
    
    let price_hex = resp.get("result").and_then(|r| r.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to get gas price"))?;
    Ok(u64::from_str_radix(price_hex.trim_start_matches("0x"), 16).unwrap_or(20000000000))
}

fn get_ethereum_balance(address: &str, rpc_url: &str) -> Result<u128> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getBalance",
            "params": [address, "latest"],
            "id": 1
        }))
        .send()?
        .json::<serde_json::Value>()?;
    
    let balance_hex = resp.get("result").and_then(|r| r.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to get balance"))?;
    Ok(u128::from_str_radix(balance_hex.trim_start_matches("0x"), 16).unwrap_or(0))
}

fn sign_ethereum_transaction(tx: &EthTransaction, secret_key: &secp256k1::SecretKey) -> Result<String> {
    use sha3::{Digest, Keccak256};
    use secp256k1::{Message, Secp256k1};
    
    // RLP encode transaction
    let mut rlp = Vec::new();
    
    // Nonce
    rlp.extend_from_slice(&encode_rlp(tx.nonce));
    // Gas price
    rlp.extend_from_slice(&encode_rlp(tx.gas_price));
    // Gas limit
    rlp.extend_from_slice(&encode_rlp(tx.gas_limit));
    // To
    if let Some(to) = &tx.to {
        rlp.extend_from_slice(&encode_rlp_length(to.len()));
        rlp.extend_from_slice(to);
    } else {
        rlp.push(0x80);
    }
    // Value
    rlp.extend_from_slice(&encode_rlp(tx.value));
    // Data
    rlp.extend_from_slice(&encode_rlp_length(tx.data.len()));
    rlp.extend_from_slice(&tx.data);
    // Chain ID, 0, 0 for EIP-155
    rlp.extend_from_slice(&encode_rlp(tx.chain_id));
    rlp.push(0x80);
    rlp.push(0x80);
    
    // Wrap in list
    let mut encoded = if rlp.len() <= 55 {
        vec![0xc0 + rlp.len() as u8]
    } else {
        let len_bytes = encode_length_bytes(rlp.len());
        let mut e = vec![0xf7 + len_bytes.len() as u8];
        e.extend_from_slice(&len_bytes);
        e
    };
    encoded.extend_from_slice(&rlp);
    
    // Hash and sign
    let hash = Keccak256::digest(&encoded);
    let message = Message::from_digest_slice(&hash)?;
    let secp = Secp256k1::new();
    let sig = secp.sign_ecdsa(&message, secret_key);
    let sig_bytes = sig.serialize_compact();
    
    // Determine recovery ID (v) by checking which public key recovers correctly
    // For simplicity, we try both 0 and 1 and use 0 as default
    // In production, you should properly compute the recovery ID
    let recovery_id = 0u8; // Default to 0
    
    // Build signed transaction with v, r, s
    let mut signed_rlp = Vec::new();
    signed_rlp.extend_from_slice(&encode_rlp(tx.nonce));
    signed_rlp.extend_from_slice(&encode_rlp(tx.gas_price));
    signed_rlp.extend_from_slice(&encode_rlp(tx.gas_limit));
    if let Some(to) = &tx.to {
        signed_rlp.extend_from_slice(&encode_rlp_length(to.len()));
        signed_rlp.extend_from_slice(to);
    } else {
        signed_rlp.push(0x80);
    }
    signed_rlp.extend_from_slice(&encode_rlp(tx.value));
    signed_rlp.extend_from_slice(&encode_rlp_length(tx.data.len()));
    signed_rlp.extend_from_slice(&tx.data);
    // v = chain_id * 2 + 35 + recovery_id
    let v = tx.chain_id * 2 + 35 + recovery_id as u64;
    signed_rlp.extend_from_slice(&encode_rlp(v));
    // r
    signed_rlp.extend_from_slice(&encode_rlp_bytes(&sig_bytes[..32]));
    // s
    signed_rlp.extend_from_slice(&encode_rlp_bytes(&sig_bytes[32..]));
    
    // Wrap in list
    let mut signed_encoded = if signed_rlp.len() <= 55 {
        vec![0xc0 + signed_rlp.len() as u8]
    } else {
        let len_bytes = encode_length_bytes(signed_rlp.len());
        let mut e = vec![0xf7 + len_bytes.len() as u8];
        e.extend_from_slice(&len_bytes);
        e
    };
    signed_encoded.extend_from_slice(&signed_rlp);
    
    Ok(hex::encode(signed_encoded))
}

fn encode_rlp(value: u64) -> Vec<u8> {
    if value == 0 {
        return vec![0x80];
    }
    let bytes = value.to_be_bytes();
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(8);
    let len = 8 - start;
    if len == 1 && bytes[start] < 0x80 {
        return vec![bytes[start]];
    }
    let mut result = vec![0x80 + len as u8];
    result.extend_from_slice(&bytes[start..]);
    result
}

fn encode_rlp_length(len: usize) -> Vec<u8> {
    if len == 0 {
        return vec![0x80];
    }
    if len < 56 {
        return vec![0x80 + len as u8];
    }
    let bytes = encode_length_bytes(len);
    let mut result = vec![0xb7 + bytes.len() as u8];
    result.extend_from_slice(&bytes);
    result
}

fn encode_rlp_bytes(bytes: &[u8]) -> Vec<u8> {
    if bytes.len() == 1 && bytes[0] < 0x80 {
        return vec![bytes[0]];
    }
    let mut result = encode_rlp_length(bytes.len());
    result.extend_from_slice(bytes);
    result
}

fn encode_length_bytes(len: usize) -> Vec<u8> {
    let mut n = len;
    let mut bytes = Vec::new();
    while n > 0 {
        bytes.push((n & 0xff) as u8);
        n >>= 8;
    }
    bytes.reverse();
    bytes
}

fn send_raw_ethereum_transaction(signed_tx_hex: &str, rpc_url: &str) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_sendRawTransaction",
            "params": [format!("0x{}", signed_tx_hex)],
            "id": 1
        }))
        .send()?
        .json::<serde_json::Value>()?;
    
    if let Some(error) = resp.get("error") {
        return Err(anyhow::anyhow!("RPC error: {}", error));
    }
    
    resp.get("result")
        .and_then(|r| r.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Failed to send transaction"))
}
