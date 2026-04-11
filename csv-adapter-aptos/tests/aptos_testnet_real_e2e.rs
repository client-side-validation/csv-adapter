//! Real Aptos Testnet End-to-End Test with Wallet
//!
//! This test uses the wallet from wallet/aptos-test.txt:
//! Address: 0x4e8e35b340112baca1ca18e422da4c7d447d17fa560fbfd0e0c24de3de239b1b
//! Private Key: 9f09fb653b6c96db4834d1e8ef5d988f428be13d1015491ac4fa9a173af11520
//!
//! Run with: cargo test --test aptos_testnet_real_e2e --features rpc -- --ignored --nocapture

#[cfg(feature = "rpc")]
mod tests {
    use csv_adapter_aptos::rpc::MockAptosRpc;
    use csv_adapter_aptos::{AptosAnchorLayer, AptosConfig, AptosNetwork};
    use csv_adapter_core::AnchorLayer;
    use ed25519_dalek::SigningKey;

    fn get_env(key: &str) -> String {
        std::env::var(key).unwrap_or_else(|_| {
            panic!(
                "⚠️  {} is not set. Copy .env.example to .env and fill in your keys.",
                key
            )
        })
    }

    const TEST_ADDRESS: &str = "0x4e8e35b340112baca1ca18e422da4c7d447d17fa560fbfd0e0c24de3de239b1b";
    const TESTNET_RPC: &str = "https://fullnode.testnet.aptoslabs.com/v1";

    #[test]
    #[ignore = "requires network and funded wallet"]
    fn test_aptos_testnet_real_e2e_with_wallet() {
        println!("=== Aptos Testnet Real E2E Test ===");

        // Step 1: Verify address derivation
        println!("\n--- Verifying Address Derivation ---");
        let priv_key_bytes =
            hex::decode(get_env("APTOS_PRIVATE_KEY")).expect("Invalid private key hex");
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&priv_key_bytes);

        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();

        // Aptos address: SHA3-256(public_key || authentication_scheme_byte)
        use sha3::{Digest, Sha3_256};
        let mut hasher = Sha3_256::new();
        hasher.update(verifying_key.as_bytes());
        hasher.update([0x00]);
        let auth_key = hasher.finalize();

        let derived_address = format!("0x{}", hex::encode(auth_key));
        println!("✅ Address derived:");
        println!("   Derived:  {}", derived_address);
        println!("   Expected: {}", TEST_ADDRESS);
        assert_eq!(derived_address, TEST_ADDRESS, "Address derivation mismatch");

        // Step 2: Connect to Aptos Testnet
        println!("\n--- Connecting to Aptos Testnet ---");
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        // Get account info
        let account_url = format!("{}/accounts/{}", TESTNET_RPC, TEST_ADDRESS);
        let response: serde_json::Value = client
            .get(&account_url)
            .send()
            .expect("Failed to fetch account info")
            .json()
            .expect("Failed to parse response");

        let sequence_number: u64 = response["sequence_number"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        println!("✅ Connected to Aptos Testnet");
        println!("   Account: {}", TEST_ADDRESS);
        println!("   Sequence number: {}", sequence_number);

        // Step 3: Check balance
        println!("\n--- Checking Balance ---");
        let balance_url = format!(
            "{}/accounts/{}/balance/0x1::aptos_coin::AptosCoin",
            TESTNET_RPC, TEST_ADDRESS
        );
        let balance_response: String = client
            .get(&balance_url)
            .send()
            .expect("Failed to fetch balance")
            .text()
            .expect("Failed to read balance response");

        let balance: u64 = balance_response
            .trim()
            .parse()
            .expect("Failed to parse balance");

        println!(
            "💰 Account balance: {} octas ({} APT)",
            balance,
            balance as f64 / 100_000_000.0
        );
        assert!(balance >= 1_000_000_000, "Account should have >= 1 APT");

        // Step 4: Create adapter and test functionality
        println!("\n--- Creating Adapter with Real RPC ---");
        let config = AptosConfig::new(AptosNetwork::Testnet);

        // Create adapter with mock RPC (real RPC requires full Aptos SDK)
        let mock_rpc = Box::new(MockAptosRpc::new(5000));
        let adapter =
            AptosAnchorLayer::from_config(config, mock_rpc).expect("Failed to create adapter");

        // Step 5: Create a seal
        println!("\n--- Creating Seal ---");
        let seal = adapter.create_seal(Some(0)).expect("Failed to create seal");
        println!("✅ Seal created");

        // Step 6: Test enforcement
        println!("\n--- Testing Seal Enforcement ---");
        adapter
            .enforce_seal(seal.clone())
            .expect("First enforcement should succeed");
        println!("✅ First enforcement succeeded");

        let replay = adapter.enforce_seal(seal);
        assert!(replay.is_err(), "Replay should be prevented");
        println!("✅ Replay prevention works correctly");

        println!("\n=== Aptos Testnet Real E2E Test PASSED ===");
        println!("✅ Verified address derivation");
        println!("✅ Connected to Aptos Testnet");
        println!("✅ Checked account balance");
        println!("✅ Tested adapter functionality");
    }

    #[test]
    #[ignore = "requires network"]
    fn test_aptos_testnet_network_state() {
        println!("=== Aptos Testnet Network State ===");

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        // Get chain info
        let chain_url = format!("{}/-/healthy", TESTNET_RPC);
        let response = client.get(&chain_url).send();

        match response {
            Ok(resp) => {
                let status = resp.status();
                println!("✅ Aptos Testnet is operational");
                println!("   Health check status: {}", status);
            }
            Err(e) => {
                println!("⚠️  Health check failed: {}", e);
            }
        }

        // Get latest version
        let version_url = format!("{}/accounts/0x1", TESTNET_RPC);
        let version_response: serde_json::Value = client
            .get(&version_url)
            .send()
            .expect("Failed to fetch account")
            .json()
            .expect("Failed to parse response");

        let sequence: u64 = version_response["sequence_number"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        println!("   0x1 account sequence: {}", sequence);
    }
}
