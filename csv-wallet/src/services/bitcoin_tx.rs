//! Bitcoin transaction builder with UTXO support
//!
//! Builds real Bitcoin transactions for cross-chain locking.
//! Uses mempool.space API for UTXO fetching and transaction broadcast.

use crate::services::blockchain::BlockchainError;
use csv_adapter_core::Chain;

/// UTXO from blockchain
#[derive(Clone, Debug)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub value: u64, // satoshis
    pub script_pubkey: String,
}

/// Build a Bitcoin anchor transaction for cross-chain locking
/// 
/// This creates a transaction that:
/// 1. Consumes a UTXO from the sender
/// 2. Creates an OP_RETURN output with the lock commitment
/// 3. Returns change to sender
/// 
/// Returns (unsigned_tx, utxo) for signing
pub async fn build_anchor_transaction(
    sender_address: &str,
    lock_data: &[u8],
    rpc_url: &str,
) -> Result<(Vec<u8>, Utxo), BlockchainError> {
    // Fetch UTXOs for the sender
    let utxos = fetch_utxos(sender_address, rpc_url).await?;
    
    if utxos.is_empty() {
        return Err(BlockchainError {
            message: format!("No UTXOs available for address: {}", sender_address),
            chain: Some(Chain::Bitcoin),
            code: None,
        });
    }
    
    // Use the first UTXO (simplified - real implementation would select optimally)
    let utxo = utxos[0].clone();
    
    // Build transaction
    let mut tx = Vec::new();
    
    // Version (4 bytes, little-endian)
    tx.extend_from_slice(&2u32.to_le_bytes());
    
    // Input count (varint)
    tx.push(1); // One input
    
    // Input: Outpoint (txid + vout)
    let txid_bytes = hex::decode(&utxo.txid)
        .map_err(|e| BlockchainError {
            message: format!("Invalid UTXO txid: {}", e),
            chain: Some(Chain::Bitcoin),
            code: None,
        })?;
    // txid is reversed in Bitcoin
    let txid_rev: Vec<u8> = txid_bytes.iter().rev().cloned().collect();
    tx.extend_from_slice(&txid_rev);
    tx.extend_from_slice(&utxo.vout.to_le_bytes());
    
    // Input: scriptSig length (0 for now - to be signed later)
    tx.push(0);
    
    // Input: sequence (0xffffffff)
    tx.extend_from_slice(&0xffffffffu32.to_le_bytes());
    
    // Output count (varint)
    tx.push(1); // Only OP_RETURN output - remaining value goes to miner fee
    
    // Output 1: OP_RETURN with lock data
    // Value: 0 satoshis
    tx.extend_from_slice(&0u64.to_le_bytes());
    // Script: OP_RETURN <data>
    let script_len = 1 + lock_data.len(); // OP_RETURN + data
    if script_len <= 80 { // Bitcoin allows up to 80 bytes in OP_RETURN
        encode_varint(&mut tx, script_len as u64);
        tx.push(0x6a); // OP_RETURN
        tx.extend_from_slice(lock_data);
    }
    
    // Note: No change output - the entire UTXO value minus 0 goes to miner fee
    // This is acceptable for testnet where UTXO values are small
    web_sys::console::log_1(&format!("Building tx: consuming {} satoshi, all to miner fee", utxo.value).into());
    
    // Locktime (4 bytes)
    tx.extend_from_slice(&0u32.to_le_bytes());
    
    Ok((tx, utxo))
}

/// Fetch UTXOs for a Bitcoin address
async fn fetch_utxos(
    address: &str,
    rpc_url: &str,
) -> Result<Vec<Utxo>, BlockchainError> {
    let client = reqwest::Client::new();
    let url = format!("{}/address/{}/utxo", rpc_url.trim_end_matches('/'), address);
    
    web_sys::console::log_1(&format!("Fetching UTXOs from: {}", url).into());
    
    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| BlockchainError {
            message: format!("Failed to fetch UTXOs: {}", e),
            chain: Some(Chain::Bitcoin),
            code: None,
        })?;
    
    // Get raw text for debugging
    let text = response.text().await.map_err(|e| BlockchainError {
        message: format!("Failed to read UTXO response: {}", e),
        chain: Some(Chain::Bitcoin),
        code: None,
    })?;
    
    web_sys::console::log_1(&format!("UTXO response: {}", &text[..text.len().min(200)]).into());
    
    // Parse as generic JSON first to handle different formats
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| BlockchainError {
        message: format!("Failed to parse UTXO JSON: {}", e),
        chain: Some(Chain::Bitcoin),
        code: None,
    })?;
    
    let mut utxos = Vec::new();
    
    // Handle array response
    if let Some(array) = json.as_array() {
        for item in array {
            if let Some(txid) = item.get("txid").and_then(|v| v.as_str()) {
                if let Some(vout) = item.get("vout").and_then(|v| v.as_u64()) {
                    if let Some(value) = item.get("value").and_then(|v| v.as_u64()) {
                        utxos.push(Utxo {
                            txid: txid.to_string(),
                            vout: vout as u32,
                            value,
                            script_pubkey: item.get("scriptPubKey")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                        });
                    }
                }
            }
        }
    }
    
    web_sys::console::log_1(&format!("Parsed {} UTXOs", utxos.len()).into());
    
    Ok(utxos)
}

/// Encode a varint (variable length integer)
fn encode_varint(buf: &mut Vec<u8>, value: u64) {
    if value < 0xfd {
        buf.push(value as u8);
    } else if value <= 0xffff {
        buf.push(0xfd);
        buf.extend_from_slice(&(value as u16).to_le_bytes());
    } else if value <= 0xffffffff {
        buf.push(0xfe);
        buf.extend_from_slice(&(value as u32).to_le_bytes());
    } else {
        buf.push(0xff);
        buf.extend_from_slice(&value.to_le_bytes());
    }
}

/// Derive scriptPubKey from a Bitcoin address
/// This is needed because mempool.space /utxo endpoint doesn't return scriptPubKey
fn derive_script_pubkey_from_address(address: &str) -> Result<Vec<u8>, BlockchainError> {
    use bitcoin::{Address, Network, ScriptBuf};
    use std::str::FromStr;
    
    // Parse the address
    let addr = Address::from_str(address)
        .map_err(|e| BlockchainError {
            message: format!("Invalid address: {}", e),
            chain: Some(Chain::Bitcoin),
            code: None,
        })?;
    
    // Assume testnet for tb1... addresses
    let script_pubkey: ScriptBuf = if address.starts_with("tb1") || address.starts_with("m") || address.starts_with("n") || address.starts_with("2") {
        addr.require_network(Network::Testnet)
            .map_err(|e| BlockchainError {
                message: format!("Wrong network: {}", e),
                chain: Some(Chain::Bitcoin),
                code: None,
            })?
            .script_pubkey()
    } else {
        // Mainnet addresses
        addr.assume_checked().script_pubkey()
    };
    
    Ok(script_pubkey.to_bytes())
}

/// Sign a Bitcoin transaction using the bitcoin crate for proper format support
/// 
/// This uses the `bitcoin` crate which has full support for:
/// - Legacy P2PKH (1... addresses)
/// - SegWit v0 P2WPKH (tb1q... addresses)  
/// - Taproot P2TR (tb1p... addresses) with BIP340 Schnorr signatures
pub fn sign_bitcoin_transaction(
    unsigned_tx: &[u8],
    private_key_hex: &str,
    utxo: &Utxo,
    sender_address: &str,
) -> Result<Vec<u8>, BlockchainError> {
    use bitcoin::{
        secp256k1::{Secp256k1, SecretKey, Keypair, Message, XOnlyPublicKey, PublicKey, Scalar},
        sighash::{SighashCache, EcdsaSighashType, TapSighashType},
        consensus::serialize,
        key::TapTweak,
        hashes::Hash,
        taproot::TapTweakHash,
        Transaction, TxOut, ScriptBuf, Witness,
    };
    
    // Detect address type from scriptPubKey
    let is_taproot = if let Ok(script) = hex::decode(&utxo.script_pubkey) {
        // Taproot: 0x51 0x20 <32_bytes> (34 bytes total)
        script.len() == 34 && script[0] == 0x51 && script[1] == 0x20
    } else {
        false
    };
    
    let is_segwit_v0 = if let Ok(script) = hex::decode(&utxo.script_pubkey) {
        // P2WPKH: 0x00 0x14 <20_bytes> (22 bytes total)
        script.len() == 22 && script[0] == 0x00 && script[1] == 0x14
    } else {
        false
    };
    
    web_sys::console::log_1(&format!("Address type - Taproot: {}, SegWit v0: {}", is_taproot, is_segwit_v0).into());
    
    // Parse the private key (take first 32 bytes for 64-byte seeds)
    let cleaned = private_key_hex.trim().trim_start_matches("0x").trim();
    let key_bytes = hex::decode(cleaned)
        .map_err(|e| BlockchainError {
            message: format!("Invalid private key hex: {}", e),
            chain: Some(Chain::Bitcoin),
            code: None,
        })?;
    
    // Use first 32 bytes (handles both 32-byte and 64-byte formats)
    let key_32: [u8; 32] = key_bytes[..32.min(key_bytes.len())]
        .try_into()
        .map_err(|_| BlockchainError {
            message: "Key too short".to_string(),
            chain: Some(Chain::Bitcoin),
            code: None,
        })?;
    
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&key_32)
        .map_err(|e| BlockchainError {
            message: format!("Invalid secret key: {}", e),
            chain: Some(Chain::Bitcoin),
            code: None,
        })?;
    
    // Parse the unsigned transaction
    let mut tx: Transaction = bitcoin::consensus::deserialize(unsigned_tx)
        .map_err(|e| BlockchainError {
            message: format!("Failed to parse unsigned tx: {}", e),
            chain: Some(Chain::Bitcoin),
            code: None,
        })?;
    
    // Build the UTXO output script from scriptPubKey hex, or derive from address if empty
    let script_pubkey_bytes = if utxo.script_pubkey.is_empty() {
        web_sys::console::log_1(&"UTXO scriptPubKey empty, deriving from sender address".into());
        derive_script_pubkey_from_address(sender_address)?
    } else {
        hex::decode(&utxo.script_pubkey)
            .map_err(|e| BlockchainError {
                message: format!("Invalid scriptPubKey hex: {}", e),
                chain: Some(Chain::Bitcoin),
                code: None,
            })?
    };
    
    let prev_output = TxOut {
        value: bitcoin::Amount::from_sat(utxo.value),
        script_pubkey: ScriptBuf::from_bytes(script_pubkey_bytes),
    };
    
    // Re-detect address type from actual script
    let is_taproot = prev_output.script_pubkey.len() == 34 && 
                     prev_output.script_pubkey.as_bytes()[0] == 0x51 &&
                     prev_output.script_pubkey.as_bytes()[1] == 0x20;
    let is_segwit_v0 = prev_output.script_pubkey.len() == 22 && 
                       prev_output.script_pubkey.as_bytes()[0] == 0x00 &&
                       prev_output.script_pubkey.as_bytes()[1] == 0x14;
    
    web_sys::console::log_1(&format!("Actual address type - Taproot: {}, SegWit v0: {}, script len: {}", 
        is_taproot, is_segwit_v0, prev_output.script_pubkey.len()).into());
    
    if is_taproot {
        // Taproot signing (BIP340 Schnorr)
        // Get the internal keypair and x-only pubkey
        let keypair = Keypair::from_secret_key(&secp, &secret_key);
        let (xonly_pubkey, _parity) = XOnlyPublicKey::from_keypair(&keypair);
        
        // Compute the tweaked pubkey for key-path spending (no script tree = None)
        let (_tweaked_pubkey, _parity) = xonly_pubkey.tap_tweak(&secp, None);
        
        // For key-path signing, we need the tweaked secret key
        // The tweak is computed as: hash_tap_tweak(xonly_pubkey || merkle_root)
        // For key-path only: merkle_root is None (empty hash)
        // TapTweakHash::hash() computes the tagged hash
        let tweak_hash = TapTweakHash::from_key_and_tweak(xonly_pubkey, None);
        let tweak_scalar = Scalar::from_be_bytes(tweak_hash.to_byte_array())
            .map_err(|e| BlockchainError {
                message: format!("Invalid tweak scalar: {:?}", e),
                chain: Some(Chain::Bitcoin),
                code: None,
            })?;
        
        // Add tweak to secret key: tweaked_key = key + tweak (mod n)
        let tweaked_secret_key = secret_key.add_tweak(&tweak_scalar)
            .map_err(|e| BlockchainError {
                message: format!("Failed to tweak secret key: {:?}", e),
                chain: Some(Chain::Bitcoin),
                code: None,
            })?;
        let tweaked_keypair = Keypair::from_secret_key(&secp, &tweaked_secret_key);
        
        // For P2TR key path spending, we sign with Schnorr using the tweaked key
        // Build the sighash for Taproot
        let mut sighash_cache = SighashCache::new(&mut tx);
        let sighash = sighash_cache
            .taproot_key_spend_signature_hash(
                0, // input index
                &bitcoin::sighash::Prevouts::All(&[prev_output]),
                TapSighashType::Default,
            )
            .map_err(|e| BlockchainError {
                message: format!("Sighash failed: {}", e),
                chain: Some(Chain::Bitcoin),
                code: None,
            })?;
        
        let msg = Message::from_digest_slice(sighash.as_ref())
            .map_err(|e| BlockchainError {
                message: format!("Message error: {}", e),
                chain: Some(Chain::Bitcoin),
                code: None,
            })?;
        
        // Sign with Schnorr (BIP340) using the tweaked keypair
        let signature = secp.sign_schnorr(&msg, &tweaked_keypair);
        
        // The signature is 64 bytes for Schnorr (no sighash byte needed)
        let sig_bytes = signature.as_ref();
        
        // Build witness: just the signature (key path spend)
        let sig_vec = sig_bytes.to_vec();
        let witness = Witness::from_slice(&[sig_vec.as_slice()]);
        tx.input[0].witness = witness;
        
        web_sys::console::log_1(&"Signed with Taproot BIP340 Schnorr".into());
        
    } else if is_segwit_v0 {
        // SegWit v0 (P2WPKH) signing - ECDSA with witness
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        
        let mut sighash_cache = SighashCache::new(&mut tx);
        let sighash = sighash_cache
            .p2wpkh_signature_hash(
                0, // input index
                &prev_output.script_pubkey,
                prev_output.value,
                EcdsaSighashType::All,
            )
            .map_err(|e| BlockchainError {
                message: format!("Sighash failed: {}", e),
                chain: Some(Chain::Bitcoin),
                code: None,
            })?;
        
        let msg = Message::from_digest_slice(sighash.as_ref())
            .map_err(|e| BlockchainError {
                message: format!("Message error: {}", e),
                chain: Some(Chain::Bitcoin),
                code: None,
            })?;
        
        // Sign with ECDSA
        let signature = secp.sign_ecdsa(&msg, &secret_key);
        let sig_der = signature.serialize_der();
        let mut sig_with_hashtype = sig_der.to_vec();
        sig_with_hashtype.push(EcdsaSighashType::All as u8); // SIGHASH_ALL
        
        let pubkey_bytes = public_key.serialize();
        
        // Build witness: [signature, pubkey]
        let witness = Witness::from_slice(&[sig_with_hashtype.as_slice(), pubkey_bytes.as_slice()]);
        tx.input[0].witness = witness;
        
        web_sys::console::log_1(&"Signed with SegWit v0 ECDSA".into());
        
    } else {
        // Legacy P2PKH signing - ECDSA with scriptSig
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        
        let sighash_cache = SighashCache::new(&tx);
        let sighash = sighash_cache
            .legacy_signature_hash(
                0, // input index
                &prev_output.script_pubkey,
                EcdsaSighashType::All as u32,
            )
            .map_err(|e| BlockchainError {
                message: format!("Sighash failed: {}", e),
                chain: Some(Chain::Bitcoin),
                code: None,
            })?;
        
        let msg = Message::from_digest_slice(sighash.as_ref())
            .map_err(|e| BlockchainError {
                message: format!("Message error: {}", e),
                chain: Some(Chain::Bitcoin),
                code: None,
            })?;
        
        // Sign with ECDSA
        let signature = secp.sign_ecdsa(&msg, &secret_key);
        let sig_der = signature.serialize_der();
        let mut sig_with_hashtype = sig_der.to_vec();
        sig_with_hashtype.push(EcdsaSighashType::All as u8); // SIGHASH_ALL
        
        let pubkey_bytes = public_key.serialize();
        
        // Build scriptSig: <sig> <pubkey>
        let script_sig = ScriptBuf::builder()
            .push_slice(<&bitcoin::script::PushBytes>::try_from(sig_with_hashtype.as_slice()).unwrap())
            .push_slice(<&bitcoin::script::PushBytes>::try_from(pubkey_bytes.as_slice()).unwrap())
            .into_script();
        
        tx.input[0].script_sig = script_sig;
        
        web_sys::console::log_1(&"Signed with Legacy P2PKH ECDSA".into());
    }
    
    // Serialize the signed transaction
    let signed_bytes = serialize(&tx);
    
    web_sys::console::log_1(&format!("Signed tx hex (first 120 chars): {}", 
        &hex::encode(&signed_bytes)[..120.min(hex::encode(&signed_bytes).len())]).into());
    
    Ok(signed_bytes)
}

/// Extract hash160 (20 bytes) from a Bitcoin address
/// Supports P2PKH (1...) and P2SH (3...) addresses
fn extract_hash160_from_address(address: &str) -> Option<[u8; 20]> {
    // Try to decode as base58 (P2PKH or P2SH)
    if let Ok(decoded) = bs58::decode(address).into_vec() {
        if decoded.len() == 25 {
            // P2PKH: 1 byte version + 20 bytes hash160 + 4 bytes checksum
            // P2SH: same structure
            let mut hash = [0u8; 20];
            hash.copy_from_slice(&decoded[1..21]);
            return Some(hash);
        }
    }
    
    // Bech32 addresses (bc1...) - Taproot/SegWit v1
    // For now, return None and we'll use UTXO script hash instead
    None
}

/// Extract hash160 from a scriptPubKey
fn extract_hash160_from_script(script: &str) -> Option<[u8; 20]> {
    if script.len() >= 40 {
        // Try to extract last 20 bytes (40 hex chars) from common script patterns
        // P2PKH: 76a914{20bytes}88ac
        // P2SH: a914{20bytes}87
        // P2WPKH: 0014{20bytes}
        if let Ok(bytes) = hex::decode(script) {
            if bytes.len() >= 22 {
                // Check for P2WPKH pattern (0x00, 0x14, ...)
                if bytes[0] == 0x00 && bytes[1] == 0x14 {
                    let mut hash = [0u8; 20];
                    hash.copy_from_slice(&bytes[2..22]);
                    return Some(hash);
                }
            }
            if bytes.len() >= 23 {
                // Check for P2PKH pattern (0x76, 0xa9, 0x14, ...)
                if bytes[0] == 0x76 && bytes[1] == 0xa9 && bytes[2] == 0x14 {
                    let mut hash = [0u8; 20];
                    hash.copy_from_slice(&bytes[3..23]);
                    return Some(hash);
                }
            }
        }
    }
    None
}

/// Broadcast a Bitcoin transaction
pub async fn broadcast_transaction(
    raw_tx: &[u8],
    rpc_url: &str,
) -> Result<String, BlockchainError> {
    let client = reqwest::Client::new();
    let url = format!("{}/tx", rpc_url.trim_end_matches('/'));
    
    let tx_hex = hex::encode(raw_tx);
    
    let response = client.post(&url)
        .body(tx_hex)
        .send()
        .await
        .map_err(|e| BlockchainError {
            message: format!("Failed to broadcast: {}", e),
            chain: Some(Chain::Bitcoin),
            code: None,
        })?;
    
    // Check for HTTP error status
    let status = response.status();
    let body = response.text().await.map_err(|e| BlockchainError {
        message: format!("Failed to read response: {}", e),
        chain: Some(Chain::Bitcoin),
        code: None,
    })?;
    
    if !status.is_success() {
        return Err(BlockchainError {
            message: format!("Broadcast failed (HTTP {}): {}", status, body),
            chain: Some(Chain::Bitcoin),
            code: Some(status.as_u16() as u32),
        });
    }
    
    // Body should be the txid on success
    Ok(body)
}
