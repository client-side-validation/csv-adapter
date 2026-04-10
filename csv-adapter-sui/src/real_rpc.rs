//! Real Sui RPC client using JSON-RPC over HTTP
//!
//! Implements the SuiRpc trait using Sui's official JSON-RPC API.
//! Only compiled when the `rpc` feature is enabled.

use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use base64::Engine;

use crate::rpc::{
    SuiRpc, SuiObject, SuiTransactionBlock, SuiEvent, SuiCheckpoint, SuiLedgerInfo,
    SuiObjectChange, SuiExecutionStatus, SuiTransactionEffects,
};

/// Real Sui RPC client using JSON-RPC
pub struct SuiRpcClient {
    client: Client,
    rpc_url: String,
}

impl SuiRpcClient {
    /// Create a new Sui RPC client
    pub fn new(rpc_url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
            rpc_url: rpc_url.to_string(),
        }
    }

    /// Call a Sui JSON-RPC method
    fn rpc_call(&self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let response: Value = self.client.post(&self.rpc_url).json(&payload).send()?.json()?;

        if let Some(error) = response.get("error") {
            return Err(format!("RPC error: {}", error).into());
        }

        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Parse object ID from Sui response format
    fn parse_object_id_static(id_str: &str) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        let bytes = hex::decode(id_str.trim_start_matches("0x"))?;
        let mut result = [0u8; 32];
        result.copy_from_slice(&bytes);
        Ok(result)
    }

    /// Parse digest from Sui response format
    fn parse_digest_static(digest_str: &str) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        let bytes = hex::decode(digest_str)?;
        let mut result = [0u8; 32];
        result.copy_from_slice(&bytes[..32.min(bytes.len())]);
        Ok(result)
    }
}

impl SuiRpc for SuiRpcClient {
    fn get_object(
        &self,
        object_id: [u8; 32],
    ) -> Result<Option<SuiObject>, Box<dyn std::error::Error + Send + Sync>> {
        let id_hex = format!("0x{}", hex::encode(object_id));
        let result = self.rpc_call("sui_getObject", json!([
            id_hex,
            { "showContent": true, "showBcs": true, "showOwner": true }
        ]))?;

        if result.get("data").is_none() || result["data"].is_null() {
            return Ok(None);
        }

        let data = &result["data"];
        let object_id = Self::parse_object_id_static(data["objectId"].as_str().unwrap_or(""))?;
        let version = data["version"].as_str().unwrap_or("0").parse()?;

        Ok(Some(SuiObject {
            object_id,
            version,
            owner: data["owner"].to_string().into_bytes(),
            object_type: data["type"].as_str().unwrap_or("").to_string(),
            has_public_transfer: data["hasPublicTransfer"].as_bool().unwrap_or(false),
        }))
    }

    fn get_transaction_block(
        &self,
        digest: [u8; 32],
    ) -> Result<Option<SuiTransactionBlock>, Box<dyn std::error::Error + Send + Sync>> {
        let digest_hex = format!("0x{}", hex::encode(digest));
        let result = self.rpc_call("sui_getTransactionBlock", json!([
            digest_hex,
            { "showInput": true, "showEffects": true, "showEvents": true }
        ]))?;

        if result.is_null() {
            return Ok(None);
        }

        let effects = &result["effects"];
        let status_str = effects["status"]["status"].as_str().unwrap_or("failure");
        let status = if status_str == "success" {
            SuiExecutionStatus::Success
        } else {
            SuiExecutionStatus::Failure {
                error: effects["status"]["error"].as_str().unwrap_or("").to_string(),
            }
        };

        let checkpoint = result["checkpoint"].as_u64();
        let digest = Self::parse_digest_static(digest_hex.trim_start_matches("0x"))?;

        // Parse modified objects from effects
        let modified_objects: Vec<SuiObjectChange> = effects["modifiedObjects"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|obj| {
                        let id_str = obj["objectId"].as_str()?;
                        let object_id = Self::parse_object_id_static(id_str).ok()?;
                        Some(SuiObjectChange {
                            object_id,
                            change_type: obj["type"].as_str().unwrap_or("unknown").to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(Some(SuiTransactionBlock {
            digest,
            checkpoint,
            effects: SuiTransactionEffects {
                status,
                gas_used: effects["gasUsed"].as_u64().unwrap_or(0),
                modified_objects,
            },
        }))
    }

    fn get_transaction_events(
        &self,
        digest: [u8; 32],
    ) -> Result<Vec<SuiEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let tx_block = self.get_transaction_block(digest)?;
        if let Some(_block) = tx_block {
            // Events are embedded in the transaction block response
            // In real implementation, parse from sui_getTransactionBlock events array
            Ok(vec![])
        } else {
            Ok(vec![])
        }
    }

    fn get_checkpoint(
        &self,
        sequence_number: u64,
    ) -> Result<Option<SuiCheckpoint>, Box<dyn std::error::Error + Send + Sync>> {
        let result = self.rpc_call("sui_getCheckpoint", json!([
            sequence_number.to_string(),
            { "showBcs": true, "showTransactions": false }
        ]))?;

        if result.is_null() {
            return Ok(None);
        }

        let digest = Self::parse_digest_static(result["digest"].as_str().unwrap_or(""))?;

        Ok(Some(SuiCheckpoint {
            sequence_number,
            digest,
            epoch: result["epoch"].as_str().unwrap_or("0").parse()?,
            network_total_transactions: result["networkTotalTransactions"].as_str().unwrap_or("0").parse()?,
            certified: result["certified"].as_bool().unwrap_or(false),
        }))
    }

    fn get_latest_checkpoint_sequence_number(
        &self,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let result = self.rpc_call("sui_getLatestCheckpointSequenceNumber", json!([]))?;
        Ok(result.as_str().unwrap_or("0").parse()?)
    }

    fn sender_address(&self) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        // In production, this would be the address derived from the signer's public key
        // For now, return a placeholder
        Err("sender_address not implemented for SuiRpcClient".into())
    }

    fn get_gas_objects(
        &self,
        owner: [u8; 32],
    ) -> Result<Vec<SuiObject>, Box<dyn std::error::Error + Send + Sync>> {
        let owner_hex = format!("0x{}", hex::encode(owner));
        let result = self.rpc_call("suix_getCoins", json!([
            owner_hex,
            null, // coin type (all coins)
            null, // cursor
            null  // limit
        ]))?;

        if let Some(data) = result.get("data") {
            if let Some(coins) = data.as_array() {
                return Ok(coins.iter()
                    .filter_map(|coin| {
                        let coin_obj = coin.get("coinObjectId")?;
                        let id_str = coin_obj.as_str()?;
                        let object_id = Self::parse_object_id_static(id_str.trim_start_matches("0x")).ok()?;
                        let version = coin.get("version")?.as_str()?.parse().ok()?;
                        Some(SuiObject {
                            object_id,
                            version,
                            owner: owner.to_vec(),
                            object_type: "0x2::coin::Coin<0x2::sui::SUI>".to_string(),
                            has_public_transfer: true,
                        })
                    })
                    .collect());
            }
        }
        Ok(Vec::new())
    }

    fn execute_signed_transaction(
        &self,
        tx_bytes: Vec<u8>,
        signature: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        // Call sui_executeTransactionBlock with signed transaction
        // https://docs.sui.io/sui-jsonrpc#suix_executeTransactionBlock
        let tx_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &tx_bytes);
        let sig_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &signature);
        let pk_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &public_key);

        let result = self.rpc_call("sui_executeTransactionBlock", json!([
            tx_b64,
            [sig_b64],
            [pk_b64],
            {
                "showInput": true,
                "showEffects": true,
                "showEvents": true
            }
        ]))?;

        // Parse the response to get the transaction digest
        if let Some(digest) = result.get("digest").and_then(|d| d.as_str()) {
            Self::parse_digest_static(digest.trim_start_matches("0x"))
        } else {
            Err(format!("Failed to execute transaction: {:?}", result.get("error")).into())
        }
    }

    fn wait_for_transaction(
        &self,
        digest: [u8; 32],
        timeout_ms: u64,
    ) -> Result<Option<SuiTransactionBlock>, Box<dyn std::error::Error + Send + Sync>> {
        let start = Instant::now();
        let poll_interval = Duration::from_millis(2000);

        loop {
            if start.elapsed() > Duration::from_millis(timeout_ms) {
                return Err("Timeout waiting for transaction confirmation".into());
            }

            if let Some(block) = self.get_transaction_block(digest)? {
                if matches!(block.effects.status, SuiExecutionStatus::Success) {
                    return Ok(Some(block));
                }
            }

            std::thread::sleep(poll_interval);
        }
    }

    fn get_ledger_info(&self) -> Result<SuiLedgerInfo, Box<dyn std::error::Error + Send + Sync>> {
        let latest_checkpoint = self.get_latest_checkpoint_sequence_number()?;
        let checkpoint = self.get_checkpoint(latest_checkpoint)?;

        Ok(SuiLedgerInfo {
            latest_version: checkpoint.as_ref().map(|c| c.network_total_transactions).unwrap_or(0),
            latest_epoch: checkpoint.map(|c| c.epoch).unwrap_or(0),
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
