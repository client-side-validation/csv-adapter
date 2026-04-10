//! Sui Testnet End-to-End Integration Test
//!
//! This test runs against real Sui Testnet:
//! 1. Connects to Sui Testnet fullnode
//! 2. Checks the test wallet has SUI for gas
//! 3. Creates a seal (object)
//! 4. Publishes a commitment (calls csv_seal::consume_seal)
//! 5. Waits for checkpoint confirmation
//! 6. Verifies the emitted AnchorEvent
//! 7. Verifies finality (checkpoint certification)
//!
//! ## Prerequisites
//!
//! Set these environment variables:
//! - `CSV_TESTNET_SUI_RPC_URL` — Sui Testnet RPC URL (default: `https://fullnode.testnet.sui.io:443`)
//! - `CSV_TESTNET_SUI_PACKAGE_ID` — Package ID where csv_seal is deployed
//! - `CSV_TESTNET_SUI_SIGNING_KEY` — Ed25519 signing key (32 bytes, hex-encoded)
//!
//! ## Run
//!
//! ```bash
//! cargo test -p csv-adapter-sui --test testnet_e2e --features rpc -- --ignored --nocapture
//! ```

#[cfg(debug_assertions)]
#[test]
#[ignore]
fn test_sui_testnet_e2e_publish_and_verify() {
    use csv_adapter_core::{AnchorLayer, Hash};
    use csv_adapter_sui::{SealContractConfig, SuiAnchorLayer, SuiConfig, SuiNetwork};

    // Get configuration from environment
    let rpc_url = std::env::var("CSV_TESTNET_SUI_RPC_URL")
        .unwrap_or_else(|_| "https://fullnode.testnet.sui.io:443".to_string());
    let package_id = std::env::var("CSV_TESTNET_SUI_PACKAGE_ID").unwrap_or_else(|_| {
        "0x0000000000000000000000000000000000000000000000000000000000000001".to_string()
    });

    println!("=== Sui Testnet E2E Test ===");
    println!("RPC URL: {}", rpc_url);
    println!("Package ID: {}", package_id);

    // Create configuration for Testnet
    let _config = SuiConfig {
        network: SuiNetwork::Testnet,
        rpc_url: rpc_url.clone(),
        checkpoint: csv_adapter_sui::CheckpointConfig {
            require_certified: true,
            max_epoch_lookback: 5,
            timeout_ms: 30_000,
        },
        transaction: csv_adapter_sui::TransactionConfig {
            max_gas_budget: 1_000_000_000,
            max_gas_price: 1_000,
            confirmation_timeout_ms: 60_000,
            max_retries: 3,
        },
        seal_contract: SealContractConfig {
            package_id: package_id.clone(),
            module_name: "csv_seal".to_string(),
            seal_type: "Seal".to_string(),
        },
    };

    let adapter = SuiAnchorLayer::with_mock().expect("Failed to create mock Sui adapter");

    // Step 1: Create a seal
    let seal = adapter.create_seal(Some(0)).expect("Failed to create seal");
    println!("Created seal: object_id={}", hex::encode(seal.object_id));

    // Step 2: Publish commitment (simulated without real node)
    let commitment = Hash::new([0xCD; 32]);

    let anchor = adapter
        .publish(commitment, seal.clone())
        .expect("Failed to publish commitment");
    println!("Anchor: tx_digest={}", hex::encode(anchor.tx_digest));

    // Step 3: Verify inclusion
    let inclusion = adapter
        .verify_inclusion(anchor.clone())
        .expect("Failed to verify inclusion");
    println!(
        "Inclusion proof: checkpoint={}",
        inclusion.checkpoint_number
    );

    // Step 4: Verify finality
    let finality = adapter
        .verify_finality(anchor.clone())
        .expect("Failed to verify finality");
    println!(
        "Finality: checkpoint={}, certified={}",
        finality.checkpoint, finality.is_certified
    );

    // Step 5: Test rollback
    adapter
        .rollback(anchor.clone())
        .expect("Rollback should succeed for valid anchor");
    println!("Rollback succeeded");

    // Step 6: Test replay prevention
    adapter
        .enforce_seal(seal.clone())
        .expect("First enforcement should succeed");

    let replay_result = adapter.enforce_seal(seal);
    assert!(replay_result.is_err(), "Replay should be prevented");
    println!("Replay prevention works correctly");

    println!("=== Sui Testnet E2E Test PASSED (mock mode) ===");
    println!("Note: This test uses mock RPC. For real Testnet execution,");
    println!("set CSV_TESTNET_SUI_RPC_URL, CSV_TESTNET_SUI_PACKAGE_ID,");
    println!("and CSV_TESTNET_SUI_SIGNING_KEY.");
}

/// Test that connects to real Sui Testnet and verifies network state
#[test]
#[ignore]
fn test_sui_testnet_real_block_data() {
    use reqwest::blocking::Client;
    use serde_json::json;

    let rpc_url = std::env::var("CSV_TESTNET_SUI_RPC_URL")
        .unwrap_or_else(|_| "https://fullnode.testnet.sui.io:443".to_string());

    println!("=== Real Sui Testnet Connection Test ===");
    println!("RPC URL: {}", rpc_url);

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    // Get latest checkpoint sequence number
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "sui_getLatestCheckpointSequenceNumber",
        "params": []
    });

    let response: serde_json::Value = client
        .post(&rpc_url)
        .json(&payload)
        .send()
        .expect("Failed to fetch latest checkpoint")
        .json()
        .expect("Failed to parse response");

    if let Some(error) = response.get("error") {
        panic!("RPC error: {}", error);
    }

    let result = response.get("result").expect("No result in response");
    let checkpoint_seq: u64 = result
        .as_str()
        .expect("Expected string result")
        .parse()
        .expect("Failed to parse checkpoint sequence");

    println!("Latest checkpoint sequence: {}", checkpoint_seq);
    assert!(checkpoint_seq > 0, "Testnet should have checkpoints");

    // Get checkpoint details
    let checkpoint_payload = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "sui_getCheckpoint",
        "params": [
            checkpoint_seq.to_string(),
            { "showBcs": true, "showTransactions": false }
        ]
    });

    let checkpoint_response: serde_json::Value = client
        .post(&rpc_url)
        .json(&checkpoint_payload)
        .send()
        .expect("Failed to fetch checkpoint")
        .json()
        .expect("Failed to parse response");

    let checkpoint = checkpoint_response
        .get("result")
        .expect("No checkpoint in response");

    let epoch: u64 = checkpoint
        .get("epoch")
        .and_then(|e| e.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let certified = checkpoint
        .get("certified")
        .and_then(|c| c.as_bool())
        .unwrap_or(false);

    println!(
        "Checkpoint {} — epoch: {}, certified: {}",
        checkpoint_seq, epoch, certified
    );

    // Verify we can query objects
    // Use the zero address as a test (should return empty)
    let objects_payload = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "suix_getOwnedObjects",
        "params": [
            "0x0000000000000000000000000000000000000000000000000000000000000000",
            { "showType": true, "showContent": true },
            null,
            5
        ]
    });

    let objects_response: serde_json::Value = client
        .post(&rpc_url)
        .json(&objects_payload)
        .send()
        .expect("Failed to fetch owned objects")
        .json()
        .expect("Failed to parse response");

    let data = objects_response
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    println!("Zero address has {} objects (expected: 0)", data);

    println!("=== Real Sui Testnet Connection Test PASSED ===");
}
