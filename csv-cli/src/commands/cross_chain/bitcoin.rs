//! Bitcoin-specific cross-chain functions

use anyhow::Result;
use csv_adapter_core::hash::Hash;

use crate::config::Config;
use crate::output;

/// Encode a value as a variable-length integer (varint) for Bitcoin transactions
fn encode_varint(buf: &mut Vec<u8>, value: u64) {
    if value < 253 {
        buf.push(value as u8);
    } else if value <= u16::MAX as u64 {
        buf.push(0xFD);
        buf.extend_from_slice(&(value as u16).to_le_bytes());
    } else if value <= u32::MAX as u64 {
        buf.push(0xFE);
        buf.extend_from_slice(&(value as u32).to_le_bytes());
    } else {
        buf.push(0xFF);
        buf.extend_from_slice(&value.to_le_bytes());
    }
}

/// UTXO reference for Bitcoin transactions
#[derive(Debug, Clone)]
pub struct UtxoRef {
    pub txid: String,
    pub vout: u32,
    pub value: u64,
    pub script_pubkey: String,
}

/// Publish a Bitcoin lock transaction
pub fn publish_bitcoin_lock(address: &str, lock_data: &[u8], rpc_url: &str, private_key_hex: &str) -> Result<String> {
    // Verify the private key matches the address before attempting to spend
    let derived_address = derive_bitcoin_address_from_key(private_key_hex)?;
    if derived_address != address {
        return Err(anyhow::anyhow!(
            "Key/Address mismatch: private key derives to {}, but trying to spend from {}. \
            The UTXO was funded to a different address than what this key controls.",
            derived_address,
            address
        ));
    }
    
    let utxos = fetch_bitcoin_utxos(address, rpc_url)?;
    let utxo = utxos
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No UTXOs found for {}", address))?;
    let unsigned = build_bitcoin_op_return_tx(&utxo, lock_data)?;
    let signed = sign_bitcoin_tx(&unsigned, private_key_hex, &utxo, address)?;
    broadcast_bitcoin_tx(&signed, rpc_url)
}

/// Derive Bitcoin address (P2TR) from a private key hex string
fn derive_bitcoin_address_from_key(private_key_hex: &str) -> Result<String> {
    use bitcoin::{
        secp256k1::{Secp256k1, SecretKey, Keypair, XOnlyPublicKey},
        key::TapTweak,
        Address, Network,
    };
    
    let cleaned = private_key_hex.trim().trim_start_matches("0x").trim();
    let key_bytes = hex::decode(cleaned)?;
    let key_32: [u8; 32] = key_bytes[..32.min(key_bytes.len())]
        .try_into()
        .map_err(|_| anyhow::anyhow!("Bitcoin private key too short"))?;
    
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&key_32)?;
    let keypair = Keypair::from_secret_key(&secp, &secret_key);
    let (xonly, _parity) = XOnlyPublicKey::from_keypair(&keypair);
    let (tweaked_pubkey, _) = xonly.tap_tweak(&secp, None);
    
    // Use testnet for signet (addresses start with tb1p)
    let address = Address::p2tr_tweaked(tweaked_pubkey, Network::Testnet);
    Ok(address.to_string())
}

fn fetch_bitcoin_utxos(address: &str, rpc_url: &str) -> Result<Vec<UtxoRef>> {
    let url = format!("{}/address/{}/utxo", rpc_url.trim_end_matches('/'), address);
    let body: serde_json::Value = reqwest::blocking::get(&url)?.json()?;
    let mut out = Vec::new();
    if let Some(arr) = body.as_array() {
        for v in arr {
            if let (Some(txid), Some(vout), Some(value)) = (
                v.get("txid").and_then(|x| x.as_str()),
                v.get("vout").and_then(|x| x.as_u64()),
                v.get("value").and_then(|x| x.as_u64()),
            ) {
                out.push(UtxoRef {
                    txid: txid.to_string(),
                    vout: vout as u32,
                    value,
                    script_pubkey: v
                        .get("scriptpubkey")
                        .or_else(|| v.get("scriptPubKey"))
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                });
            }
        }
    }
    Ok(out)
}

fn build_bitcoin_op_return_tx(utxo: &UtxoRef, lock_data: &[u8]) -> Result<Vec<u8>> {
    let mut tx = Vec::new();
    tx.extend_from_slice(&2u32.to_le_bytes());
    tx.push(1);
    let txid_bytes = hex::decode(&utxo.txid)?;
    let rev: Vec<u8> = txid_bytes.into_iter().rev().collect();
    tx.extend_from_slice(&rev);
    tx.extend_from_slice(&utxo.vout.to_le_bytes());
    tx.push(0);
    tx.extend_from_slice(&0xffffffffu32.to_le_bytes());
    tx.push(1);
    tx.extend_from_slice(&0u64.to_le_bytes());
    let data_len = lock_data.len();
    if data_len > 80 {
        return Err(anyhow::anyhow!("Bitcoin OP_RETURN lock data too long (>80 bytes)"));
    }
    if data_len <= 75 {
        let script_len = 1 + 1 + data_len;
        encode_varint(&mut tx, script_len as u64);
        tx.push(0x6a);
        tx.push(data_len as u8);
        tx.extend_from_slice(lock_data);
    } else {
        let script_len = 1 + 1 + 1 + data_len;
        encode_varint(&mut tx, script_len as u64);
        tx.push(0x6a);
        tx.push(0x4c);
        tx.push(data_len as u8);
        tx.extend_from_slice(lock_data);
    }
    tx.extend_from_slice(&0u32.to_le_bytes());
    Ok(tx)
}

fn sign_bitcoin_tx(unsigned_tx: &[u8], private_key_hex: &str, utxo: &UtxoRef, sender_address: &str) -> Result<Vec<u8>> {
    use bitcoin::{
        consensus::serialize,
        key::{Keypair, TapTweak},
        secp256k1::{Message, PublicKey, Secp256k1, SecretKey},
        sighash::{EcdsaSighashType, SighashCache, TapSighashType},
        ScriptBuf, Transaction, TxOut, Witness, Amount,
    };
    let cleaned = private_key_hex.trim().trim_start_matches("0x").trim();
    let key_bytes = hex::decode(cleaned)?;
    let key_32: [u8; 32] = key_bytes[..32.min(key_bytes.len())]
        .try_into()
        .map_err(|_| anyhow::anyhow!("Bitcoin private key too short"))?;
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&key_32)?;
    let mut tx: Transaction = bitcoin::consensus::deserialize(unsigned_tx)?;
    let script_pubkey_bytes = if utxo.script_pubkey.is_empty() {
        derive_script_pubkey_from_address(sender_address)?
    } else {
        hex::decode(&utxo.script_pubkey)?
    };
    let prev_output = TxOut {
        value: Amount::from_sat(utxo.value),
        script_pubkey: ScriptBuf::from_bytes(script_pubkey_bytes),
    };
    let is_taproot = prev_output.script_pubkey.len() == 34
        && prev_output.script_pubkey.as_bytes()[0] == 0x51
        && prev_output.script_pubkey.as_bytes()[1] == 0x20;
    let is_segwit_v0 = prev_output.script_pubkey.len() == 22
        && prev_output.script_pubkey.as_bytes()[0] == 0x00
        && prev_output.script_pubkey.as_bytes()[1] == 0x14;
    if is_taproot {
        let keypair = Keypair::from_secret_key(&secp, &secret_key);
        let tweaked = keypair.tap_tweak(&secp, None);
        let signing_keypair = tweaked.to_keypair();
        let mut cache = SighashCache::new(&mut tx);
        let sighash = cache.taproot_key_spend_signature_hash(
            0,
            &bitcoin::sighash::Prevouts::All(&[prev_output]),
            TapSighashType::Default,
        )?;
        let msg = Message::from_digest_slice(sighash.as_ref())?;
        let sig = secp.sign_schnorr_no_aux_rand(&msg, &signing_keypair);
        tx.input[0].witness = Witness::from_slice(&[sig.as_ref()]);
    } else if is_segwit_v0 {
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let mut cache = SighashCache::new(&mut tx);
        let sighash = cache.p2wpkh_signature_hash(
            0,
            &prev_output.script_pubkey,
            prev_output.value,
            EcdsaSighashType::All,
        )?;
        let msg = Message::from_digest_slice(sighash.as_ref())?;
        let sig = secp.sign_ecdsa(&msg, &secret_key);
        let mut sig_with_type = sig.serialize_der().to_vec();
        sig_with_type.push(EcdsaSighashType::All as u8);
        let pubkey = public_key.serialize();
        tx.input[0].witness = Witness::from_slice(&[sig_with_type.as_slice(), pubkey.as_slice()]);
    } else {
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let cache = SighashCache::new(&tx);
        let sighash = cache.legacy_signature_hash(0, &prev_output.script_pubkey, EcdsaSighashType::All as u32)?;
        let msg = Message::from_digest_slice(sighash.as_ref())?;
        let sig = secp.sign_ecdsa(&msg, &secret_key);
        let mut sig_with_type = sig.serialize_der().to_vec();
        sig_with_type.push(EcdsaSighashType::All as u8);
        tx.input[0].script_sig = ScriptBuf::builder()
            .push_slice(<&bitcoin::script::PushBytes>::try_from(sig_with_type.as_slice()).unwrap())
            .push_slice(<&bitcoin::script::PushBytes>::try_from(public_key.serialize().as_slice()).unwrap())
            .into_script();
    }
    Ok(serialize(&tx))
}

fn derive_script_pubkey_from_address(address: &str) -> Result<Vec<u8>> {
    use bitcoin::{Address, Network};
    use std::str::FromStr;
    let addr = Address::from_str(address)?;
    let script = if address.starts_with("tb1")
        || address.starts_with("m")
        || address.starts_with("n")
        || address.starts_with("2")
    {
        addr.require_network(Network::Testnet)?.script_pubkey()
    } else {
        addr.assume_checked().script_pubkey()
    };
    Ok(script.to_bytes())
}

fn broadcast_bitcoin_tx(raw_tx: &[u8], rpc_url: &str) -> Result<String> {
    let url = format!("{}/tx", rpc_url.trim_end_matches('/'));
    let resp = reqwest::blocking::Client::new()
        .post(&url)
        .body(hex::encode(raw_tx))
        .send()?;
    let status = resp.status();
    let body = resp.text()?;
    if !status.is_success() {
        return Err(anyhow::anyhow!("Bitcoin broadcast failed ({}): {}", status, body));
    }
    Ok(body.trim().to_string())
}

