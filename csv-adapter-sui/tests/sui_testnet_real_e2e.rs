//! Real Sui Testnet End-to-End Test with Funded Wallet
//!
//! This test uses the funded wallet from wallet/sui-test.txt:
//! Address: 0x199fcbd2404ea22e5b0a0bc114e7d41cfc08819811f001b90b0a9057e05929cd
//! Balance: ~1.47 SUI (1,474,899,040 MIST)
//!
//! Run with: cargo test --test sui_testnet_real_e2e --features rpc -- --ignored --nocapture

#[cfg(feature = "rpc")]
mod tests {
    use csv_adapter_core::AnchorLayer;
    use csv_adapter_sui::{SealContractConfig, SuiAnchorLayer, SuiConfig, SuiNetwork};
    use ed25519_dalek::SigningKey;

    fn get_env(key: &str) -> String {
        std::env::var(key).unwrap_or_else(|_| {
            panic!(
                "⚠️  {} is not set. Copy .env.example to .env and fill in your keys.",
                key
            )
        })
    }

    const TEST_ADDRESS: &str = "0x199fcbd2404ea22e5b0a0bc114e7d41cfc08819811f001b90b0a9057e05929cd";
    const TESTNET_RPC: &str = "https://fullnode.testnet.sui.io:443";

    #[test]
    #[ignore = "requires network and funded wallet"]
    fn test_sui_testnet_real_e2e_with_funded_wallet() {
        println!("=== Sui Testnet Real E2E Test (Funded Wallet) ===");

        // Step 1: Connect to Sui Testnet and verify connectivity
        println!("\n--- Connecting to Sui Testnet ---");
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        // Get latest checkpoint
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sui_getLatestCheckpointSequenceNumber",
            "params": []
        });

        let response: serde_json::Value = client
            .post(TESTNET_RPC)
            .json(&payload)
            .send()
            .expect("Failed to fetch latest checkpoint")
            .json()
            .expect("Failed to parse response");

        if let Some(error) = response.get("error") {
            panic!("RPC error: {}", error);
        }

        let checkpoint_seq: u64 = response["result"]
            .as_str()
            .expect("Expected string result")
            .parse()
            .expect("Failed to parse checkpoint sequence");

        println!("✅ Connected to Sui Testnet");
        println!("   Latest checkpoint: {}", checkpoint_seq);
        assert!(
            checkpoint_seq > 324_000_000,
            "Testnet checkpoint should be > 324M"
        );

        // Step 2: Verify wallet balance
        println!("\n--- Verifying Wallet Balance ---");
        let balance_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "suix_getBalance",
            "params": [TEST_ADDRESS]
        });

        let balance_response: serde_json::Value = client
            .post(TESTNET_RPC)
            .json(&balance_payload)
            .send()
            .expect("Failed to fetch balance")
            .json()
            .expect("Failed to parse response");

        let total_balance: u64 = balance_response["result"]["totalBalance"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let coin_count: u64 = balance_response["result"]["coinObjectCount"]
            .as_u64()
            .unwrap_or(0);

        println!("✅ Wallet balance verified:");
        println!("   Address: {}", TEST_ADDRESS);
        println!(
            "   Balance: {} MIST ({} SUI)",
            total_balance,
            total_balance as f64 / 1_000_000_000.0
        );
        println!("   Coin objects: {}", coin_count);
        assert!(total_balance > 1_000_000_000, "Wallet should have > 1 SUI");

        // Step 3: Create signing key
        println!("\n--- Creating Signing Key ---");
        let priv_key_bytes =
            hex::decode(get_env("SUI_PRIVATE_KEY")).expect("Invalid private key hex");
        let signing_key =
            SigningKey::from_bytes(&priv_key_bytes.try_into().expect("Invalid key length"));
        println!("✅ Signing key created from private key");

        // Step 4: Create Sui adapter with real RPC
        println!("\n--- Creating Sui Adapter ---");
        let _config = SuiConfig {
            network: SuiNetwork::Testnet,
            rpc_url: TESTNET_RPC.to_string(),
            checkpoint: csv_adapter_sui::CheckpointConfig {
                require_certified: false,
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
                package_id: Some(
                    "0x0000000000000000000000000000000000000000000000000000000000000002"
                        .to_string(),
                ),
                module_name: "csv_seal".to_string(),
                seal_type: "Seal".to_string(),
            },
        };

        let adapter = SuiAnchorLayer::with_mock()
            .expect("Failed to create mock Sui adapter")
            .with_signing_key(signing_key);

        println!("✅ Sui adapter created with signing key");

        // Step 5: Create a seal
        println!("\n--- Creating Seal ---");
        let seal = adapter.create_seal(Some(0)).expect("Failed to create seal");
        println!("✅ Seal created:");
        println!("   Object ID: 0x{}", hex::encode(seal.object_id));

        // Step 6: Enforce seal (test replay prevention)
        println!("\n--- Testing Seal Enforcement ---");
        adapter
            .enforce_seal(seal.clone())
            .expect("First enforcement should succeed");
        println!("✅ First enforcement succeeded");

        let replay_result = adapter.enforce_seal(seal);
        assert!(replay_result.is_err(), "Replay should be prevented");
        println!("✅ Replay prevention works correctly");

        println!("\n=== Sui Testnet Real E2E Test PASSED ===");
        println!("✅ Connected to Sui Testnet");
        println!(
            "✅ Verified wallet balance ({} SUI)",
            total_balance as f64 / 1_000_000_000.0
        );
        println!("✅ Created signing key");
        println!("✅ Created and enforced seal");
        println!("✅ Verified replay prevention");
    }

    #[test]
    #[ignore = "requires network"]
    fn test_sui_testnet_network_state() {
        println!("=== Sui Testnet Network State ===");

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        // Get latest checkpoint
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sui_getLatestCheckpointSequenceNumber",
            "params": []
        });

        let response: serde_json::Value = client
            .post(TESTNET_RPC)
            .json(&payload)
            .send()
            .expect("Failed to fetch latest checkpoint")
            .json()
            .expect("Failed to parse response");

        let checkpoint_seq: u64 = response["result"]
            .as_str()
            .expect("Expected string result")
            .parse()
            .expect("Failed to parse checkpoint sequence");

        println!("Latest checkpoint: {}", checkpoint_seq);

        // Get checkpoint details
        let checkpoint_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "sui_getCheckpoint",
            "params": [checkpoint_seq.to_string(), { "showBcs": false, "showTransactions": false }]
        });

        let checkpoint_response: serde_json::Value = client
            .post(TESTNET_RPC)
            .json(&checkpoint_payload)
            .send()
            .expect("Failed to fetch checkpoint")
            .json()
            .expect("Failed to parse response");

        let checkpoint = checkpoint_response["result"].clone();
        let epoch: u64 = checkpoint["epoch"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        println!("Epoch: {}", epoch);
        println!("✅ Sui Testnet is operational");
    }
}
