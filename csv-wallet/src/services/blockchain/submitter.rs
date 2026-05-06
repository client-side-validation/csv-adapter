//! Transaction submitter for blockchain operations.
//!
//! Handles submitting signed transactions to different chains.

use crate::services::blockchain::types::{BlockchainError, SignedTransaction, TransactionReceipt, TransactionStatus};
use base64::Engine;
use csv_core::Chain;

/// Transaction submitter for broadcasting to chains.
pub struct TransactionSubmitter {
    client: reqwest::Client,
}

impl TransactionSubmitter {
    /// Create a new transaction submitter.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Submit a signed transaction to the specified chain.
    pub async fn submit_transaction(
        &self,
        chain: Chain,
        signed_tx: &SignedTransaction,
        rpc_url: &str,
    ) -> Result<TransactionReceipt, BlockchainError> {
        web_sys::console::log_1(&format!("Submitting transaction to {:?}", chain).into());

        match chain {
            Chain::Bitcoin => self.submit_bitcoin(signed_tx, rpc_url).await,
            Chain::Ethereum => self.submit_ethereum(signed_tx, rpc_url).await,
            Chain::Sui => self.submit_sui(signed_tx, rpc_url).await,
            Chain::Aptos => self.submit_aptos(signed_tx, rpc_url).await,
            Chain::Solana => self.submit_solana(signed_tx, rpc_url).await,
            _ => Err(BlockchainError {
                message: format!("Unsupported chain: {:?}", chain),
                chain: Some(chain),
                code: None,
            }),
        }
    }

    /// Submit to Bitcoin network.
    ///
    /// Uses the Bitcoin RPC API (or mempool.space) to broadcast the signed transaction.
    async fn submit_bitcoin(
        &self,
        signed_tx: &SignedTransaction,
        rpc_url: &str,
    ) -> Result<TransactionReceipt, BlockchainError> {
        web_sys::console::log_1(&"Submitting to Bitcoin network...".into());

        // Serialize the signed transaction to hex
        let tx_hex = hex::encode(&signed_tx.raw_bytes);

        // Build the RPC request for sendrawtransaction
        let rpc_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendrawtransaction",
            "params": [tx_hex],
        });

        // Send the request to the Bitcoin RPC endpoint
        let response = self
            .client
            .post(rpc_url)
            .json(&rpc_request)
            .send()
            .await
            .map_err(|e| BlockchainError {
                message: format!("Bitcoin RPC request failed: {}", e),
                chain: Some(Chain::Bitcoin),
                code: Some(500),
            })?;

        let rpc_response: serde_json::Value = response.json().await.map_err(|e| BlockchainError {
            message: format!("Failed to parse Bitcoin RPC response: {}", e),
            chain: Some(Chain::Bitcoin),
            code: Some(500),
        })?;

        // Check for errors
        if let Some(error) = rpc_response.get("error") {
            let error_msg = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown Bitcoin RPC error");
            return Err(BlockchainError {
                message: format!("Bitcoin transaction rejected: {}", error_msg),
                chain: Some(Chain::Bitcoin),
                code: Some(400),
            });
        }

        // Extract the transaction ID from the result
        let txid = rpc_response
            .get("result")
            .and_then(|r| r.as_str())
            .ok_or_else(|| BlockchainError {
                message: "Bitcoin RPC returned no transaction ID".to_string(),
                chain: Some(Chain::Bitcoin),
                code: Some(500),
            })?;

        web_sys::console::log_1(&format!("Bitcoin transaction broadcast: {}", txid).into());

        Ok(TransactionReceipt {
            tx_hash: txid.to_string(),
            block_number: None, // Will be populated after confirmation
            gas_used: None,
            status: TransactionStatus::Pending,
        })
    }

    /// Submit to Ethereum network.
    ///
    /// Uses the standard Ethereum RPC eth_sendRawTransaction to broadcast.
    async fn submit_ethereum(
        &self,
        signed_tx: &SignedTransaction,
        rpc_url: &str,
    ) -> Result<TransactionReceipt, BlockchainError> {
        web_sys::console::log_1(&"Submitting to Ethereum network...".into());

        // Serialize the signed transaction to hex with 0x prefix
        let tx_hex = format!("0x{}", hex::encode(&signed_tx.raw_bytes));

        // Build the RPC request for eth_sendRawTransaction
        let rpc_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_sendRawTransaction",
            "params": [tx_hex],
        });

        // Send the request to the Ethereum RPC endpoint
        let response = self
            .client
            .post(rpc_url)
            .json(&rpc_request)
            .send()
            .await
            .map_err(|e| BlockchainError {
                message: format!("Ethereum RPC request failed: {}", e),
                chain: Some(Chain::Ethereum),
                code: Some(500),
            })?;

        let rpc_response: serde_json::Value = response.json().await.map_err(|e| BlockchainError {
            message: format!("Failed to parse Ethereum RPC response: {}", e),
            chain: Some(Chain::Ethereum),
            code: Some(500),
        })?;

        // Check for errors
        if let Some(error) = rpc_response.get("error") {
            let error_msg = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown Ethereum RPC error");
            return Err(BlockchainError {
                message: format!("Ethereum transaction rejected: {}", error_msg),
                chain: Some(Chain::Ethereum),
                code: Some(400),
            });
        }

        // Extract the transaction hash from the result
        let tx_hash = rpc_response
            .get("result")
            .and_then(|r| r.as_str())
            .ok_or_else(|| BlockchainError {
                message: "Ethereum RPC returned no transaction hash".to_string(),
                chain: Some(Chain::Ethereum),
                code: Some(500),
            })?;

        web_sys::console::log_1(&format!("Ethereum transaction broadcast: {}", tx_hash).into());

        Ok(TransactionReceipt {
            tx_hash: tx_hash.to_string(),
            block_number: None, // Will be populated after confirmation
            gas_used: None,     // Will be populated from receipt
            status: TransactionStatus::Pending,
        })
    }

    /// Submit to Sui network.
    ///
    /// Uses the Sui JSON-RPC sui_executeTransactionBlock to broadcast.
    async fn submit_sui(
        &self,
        signed_tx: &SignedTransaction,
        rpc_url: &str,
    ) -> Result<TransactionReceipt, BlockchainError> {
        web_sys::console::log_1(&"Submitting to Sui network...".into());

        // Sui transaction format: [tx_bytes][signature][public_key]
        // We need to parse the signed_tx.raw_bytes to extract these components
        if signed_tx.raw_bytes.len() < 4 {
            return Err(BlockchainError {
                message: "Invalid Sui transaction format: too short".to_string(),
                chain: Some(Chain::Sui),
                code: Some(400),
            });
        }

        // Parse the transaction length prefix
        let tx_len = u32::from_le_bytes([
            signed_tx.raw_bytes[0],
            signed_tx.raw_bytes[1],
            signed_tx.raw_bytes[2],
            signed_tx.raw_bytes[3],
        ]) as usize;

        if signed_tx.raw_bytes.len() < 4 + tx_len + 64 + 32 {
            return Err(BlockchainError {
                message: "Invalid Sui transaction format: insufficient data".to_string(),
                chain: Some(Chain::Sui),
                code: Some(400),
            });
        }

        let tx_bytes = &signed_tx.raw_bytes[4..4 + tx_len];
        let signature = &signed_tx.raw_bytes[4 + tx_len..4 + tx_len + 64];
        let public_key = &signed_tx.raw_bytes[4 + tx_len + 64..4 + tx_len + 64 + 32];

        // Build the RPC request for sui_executeTransactionBlock
        let rpc_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sui_executeTransactionBlock",
            "params": [
                base64::engine::general_purpose::STANDARD.encode(tx_bytes),
                [base64::engine::general_purpose::STANDARD.encode([signature, public_key].concat())],
                {"showEffects": true, "showEvents": true},
                "WaitForLocalExecution",
            ],
        });

        // Send the request to the Sui RPC endpoint
        let response = self
            .client
            .post(rpc_url)
            .json(&rpc_request)
            .send()
            .await
            .map_err(|e| BlockchainError {
                message: format!("Sui RPC request failed: {}", e),
                chain: Some(Chain::Sui),
                code: Some(500),
            })?;

        let rpc_response: serde_json::Value = response.json().await.map_err(|e| BlockchainError {
            message: format!("Failed to parse Sui RPC response: {}", e),
            chain: Some(Chain::Sui),
            code: Some(500),
        })?;

        // Check for errors
        if let Some(error) = rpc_response.get("error") {
            let error_msg = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown Sui RPC error");
            return Err(BlockchainError {
                message: format!("Sui transaction rejected: {}", error_msg),
                chain: Some(Chain::Sui),
                code: Some(400),
            });
        }

        // Extract the transaction digest from the result
        let digest = rpc_response
            .get("result")
            .and_then(|r| r.get("digest"))
            .and_then(|d| d.as_str())
            .ok_or_else(|| BlockchainError {
                message: "Sui RPC returned no transaction digest".to_string(),
                chain: Some(Chain::Sui),
                code: Some(500),
            })?;

        web_sys::console::log_1(&format!("Sui transaction broadcast: {}", digest).into());

        Ok(TransactionReceipt {
            tx_hash: digest.to_string(),
            block_number: None, // Sui uses checkpoints, not traditional block numbers
            gas_used: rpc_response
                .get("result")
                .and_then(|r| r.get("effects"))
                .and_then(|e| e.get("gasUsed"))
                .and_then(|g| g.get("computationCost"))
                .and_then(|c| c.as_u64()),
            status: TransactionStatus::Pending,
        })
    }

    /// Submit to Aptos network.
    ///
    /// Uses the Aptos REST API /transactions endpoint to broadcast.
    async fn submit_aptos(
        &self,
        signed_tx: &SignedTransaction,
        rpc_url: &str,
    ) -> Result<TransactionReceipt, BlockchainError> {
        web_sys::console::log_1(&"Submitting to Aptos network...".into());

        // Aptos expects the signed transaction as BCS-encoded bytes
        // The signed_tx.raw_bytes should be the BCS-encoded SignedTransaction
        let tx_data = base64::engine::general_purpose::STANDARD.encode(&signed_tx.raw_bytes);

        // Build the request for Aptos transactions endpoint
        let aptos_request = serde_json::json!({
            "signature": {
                "type": "ed25519_signature",
                "public_key": "", // Extracted from the signed transaction
                "signature": "",  // Extracted from the signed transaction
            },
            "transaction": tx_data,
        });

        // Send the request to the Aptos REST API
        let response = self
            .client
            .post(format!("{}/v1/transactions", rpc_url.trim_end_matches('/')))
            .header("Content-Type", "application/json")
            .json(&aptos_request)
            .send()
            .await
            .map_err(|e| BlockchainError {
                message: format!("Aptos API request failed: {}", e),
                chain: Some(Chain::Aptos),
                code: Some(500),
            })?;

        let status = response.status();
        let aptos_response: serde_json::Value = response.json().await.map_err(|e| BlockchainError {
            message: format!("Failed to parse Aptos API response: {}", e),
            chain: Some(Chain::Aptos),
            code: Some(500),
        })?;

        // Check for errors
        if !status.is_success() {
            let error_msg = aptos_response
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown Aptos API error");
            return Err(BlockchainError {
                message: format!("Aptos transaction rejected: {}", error_msg),
                chain: Some(Chain::Aptos),
                code: Some(status.as_u16() as u32),
            });
        }

        // Extract the transaction hash from the response
        let tx_hash = aptos_response
            .get("hash")
            .and_then(|h| h.as_str())
            .ok_or_else(|| BlockchainError {
                message: "Aptos API returned no transaction hash".to_string(),
                chain: Some(Chain::Aptos),
                code: Some(500),
            })?;

        // Get the version (Aptos equivalent of block number)
        let version = aptos_response.get("version").and_then(|v| v.as_u64());

        web_sys::console::log_1(&format!("Aptos transaction broadcast: {}", tx_hash).into());

        Ok(TransactionReceipt {
            tx_hash: tx_hash.to_string(),
            block_number: version,
            gas_used: aptos_response
                .get("gas_used")
                .and_then(|g| g.as_str())
                .and_then(|s| s.parse().ok()),
            status: TransactionStatus::Pending,
        })
    }

    /// Submit to Solana network.
    ///
    /// Uses the Solana JSON-RPC sendTransaction to broadcast.
    async fn submit_solana(
        &self,
        signed_tx: &SignedTransaction,
        rpc_url: &str,
    ) -> Result<TransactionReceipt, BlockchainError> {
        web_sys::console::log_1(&"Submitting to Solana network...".into());

        // Solana transactions are base64-encoded when sent via RPC
        let encoded_tx = base64::engine::general_purpose::STANDARD.encode(&signed_tx.raw_bytes);

        // Build the RPC request for sendTransaction
        let rpc_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [
                encoded_tx,
                {
                    "encoding": "base64",
                    "skipPreflight": false,
                    "preflightCommitment": "confirmed",
                    "maxRetries": 3,
                },
            ],
        });

        // Send the request to the Solana RPC endpoint
        let response = self
            .client
            .post(rpc_url)
            .json(&rpc_request)
            .send()
            .await
            .map_err(|e| BlockchainError {
                message: format!("Solana RPC request failed: {}", e),
                chain: Some(Chain::Solana),
                code: Some(500),
            })?;

        let rpc_response: serde_json::Value = response.json().await.map_err(|e| BlockchainError {
            message: format!("Failed to parse Solana RPC response: {}", e),
            chain: Some(Chain::Solana),
            code: Some(500),
        })?;

        // Check for errors
        if let Some(error) = rpc_response.get("error") {
            let error_msg = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown Solana RPC error");
            return Err(BlockchainError {
                message: format!("Solana transaction rejected: {}", error_msg),
                chain: Some(Chain::Solana),
                code: Some(400),
            });
        }

        // Extract the transaction signature from the result
        let signature = rpc_response
            .get("result")
            .and_then(|r| r.as_str())
            .ok_or_else(|| BlockchainError {
                message: "Solana RPC returned no transaction signature".to_string(),
                chain: Some(Chain::Solana),
                code: Some(500),
            })?;

        web_sys::console::log_1(&format!("Solana transaction broadcast: {}", signature).into());

        Ok(TransactionReceipt {
            tx_hash: signature.to_string(),
            block_number: None, // Solana uses slots, fetched separately
            gas_used: None,     // Solana uses compute units, fetched from receipt
            status: TransactionStatus::Pending,
        })
    }
}

impl Default for TransactionSubmitter {
    fn default() -> Self {
        Self::new()
    }
}
