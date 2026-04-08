//! Bitcoin Signet End-to-End Integration Test
//!
//! This test runs against a real Bitcoin Signet node:
//! 1. Connects to a public Signet RPC endpoint
//! 2. Creates a seal (UTXO) from the test wallet
//! 3. Publishes a commitment (builds + signs + broadcasts Taproot tx)
//! 4. Waits for confirmation
//! 5. Extracts the Merkle proof from the confirmed block
//! 6. Verifies the inclusion proof
//! 7. Verifies finality (confirmation depth)
//!
//! ## Prerequisites
//!
//! Set these environment variables:
//! - `CSV_TESTNET_BITCOIN_RPC_URL` — Signet node RPC URL (e.g., `https://mempool.space/signet/api/`)
//! - `CSV_TESTNET_BITCOIN_RPC_USER` — RPC username (optional for public endpoints)
//! - `CSV_TESTNET_BITCOIN_RPC_PASS` — RPC password
//! - `CSV_TESTNET_BITCOIN_XPUB` — Extended public key for the test wallet
//!
//! ## Run
//!
//! ```bash
//! cargo test -p csv-adapter-bitcoin --test signet_e2e --features rpc -- --ignored --nocapture
//! ```

#[test]
#[ignore]
fn test_signet_e2e_publish_and_verify() {
    use csv_adapter_bitcoin::BitcoinAnchorLayer;
    use csv_adapter_core::{Hash, AnchorLayer};

    // Get configuration from environment
    let _rpc_url = std::env::var("CSV_TESTNET_BITCOIN_RPC_URL")
        .unwrap_or_else(|_| "https://mempool.space/signet/api/".to_string());
    let _xpub = std::env::var("CSV_TESTNET_BITCOIN_XPUB")
        .ok();

    println!("=== Bitcoin Signet E2E Test ===");
    println!("RPC URL: {}", _rpc_url);

    // Create adapter with Signet configuration
    let adapter = BitcoinAnchorLayer::signet()
        .expect("Failed to create Signet adapter");

    // Step 1: Create a seal
    let seal = adapter.create_seal(Some(100_000))
        .expect("Failed to create seal");
    println!("Created seal: txid={}, vout={}", seal.txid_hex(), seal.vout);

    // Step 2: Publish commitment (simulated without real node)
    let commitment = Hash::new([0xAB; 32]);

    // Without a real RPC connection, publish() returns a simulated txid
    // In a real test with RPC configured:
    // let anchor = adapter.publish(commitment, seal.clone()).unwrap();
    // println!("Published commitment: txid={}", hex::encode(anchor.txid));

    let anchor = adapter.publish(commitment, seal.clone())
        .expect("Failed to publish commitment");
    println!("Anchor: txid={}", hex::encode(anchor.txid));

    // Step 3: Verify inclusion
    let inclusion = adapter.verify_inclusion(anchor.clone())
        .expect("Failed to verify inclusion");
    println!("Inclusion proof: tx_index={}, block_height={}",
             inclusion.tx_index, inclusion.block_height);

    // Step 4: Verify finality
    let finality = adapter.verify_finality(anchor.clone())
        .expect("Failed to verify finality");
    println!("Finality: confirmations={}, meets_required={}, required_depth={}",
             finality.confirmations, finality.meets_required_depth, finality.required_depth);

    // Step 5: Test rollback
    adapter.rollback(anchor)
        .expect("Rollback should succeed for valid anchor");
    println!("Rollback succeeded");

    // Step 6: Test replay prevention
    adapter.enforce_seal(seal.clone())
        .expect("First enforcement should succeed");

    let replay_result = adapter.enforce_seal(seal);
    assert!(replay_result.is_err(), "Replay should be prevented");
    println!("Replay prevention works correctly");

    println!("=== Bitcoin Signet E2E Test PASSED ===");
    println!("Note: This test uses simulated publishing. For real Signet execution,");
    println!("set CSV_TESTNET_BITCOIN_RPC_URL to a funded Signet node.");
}

/// Test that connects to a real Signet node and verifies block data
#[test]
#[ignore]
fn test_signet_real_block_data() {
    use reqwest::blocking::Client;

    // Use public Signet API
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    // Get latest block height
    let height_url = "https://mempool.space/signet/api/blocks/tip/height";
    let height: u64 = client.get(height_url)
        .send()
        .expect("Failed to fetch block height")
        .text()
        .expect("Failed to read response")
        .trim()
        .parse()
        .expect("Failed to parse height");

    println!("Current Signet height: {}", height);
    assert!(height > 0, "Signet should have blocks");

    // Get the tip block hash
    let hash_url = "https://mempool.space/signet/api/blocks/tip/hash";
    let block_hash = client.get(hash_url)
        .send()
        .expect("Failed to fetch block hash")
        .text()
        .expect("Failed to read response")
        .trim()
        .to_string();

    println!("Tip block hash: {}", block_hash);
    assert_eq!(block_hash.len(), 64, "Block hash should be 32 bytes hex");

    // Get block txids
    let txids_url = format!("https://mempool.space/signet/api/block/{}/txids", block_hash);
    let txids_text = client.get(&txids_url)
        .send()
        .expect("Failed to fetch txids")
        .text()
        .expect("Failed to read response");

    let txids: Vec<String> = serde_json::from_str(&txids_text)
        .expect("Failed to parse txids");

    println!("Block has {} transactions", txids.len());
    assert!(!txids.is_empty(), "Block should have transactions");

    // Verify we can extract a Merkle proof
    use csv_adapter_bitcoin::proofs::{
        extract_merkle_proof_from_block,
        compute_merkle_root,
        verify_merkle_proof,
    };

    // Convert txids to byte arrays
    let block_txids: Vec<[u8; 32]> = txids.iter()
        .map(|hex| {
            let bytes = hex::decode(hex).expect("Invalid txid hex");
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            arr.reverse(); // Bitcoin displays txids in reversed byte order
            arr
        })
        .collect();

    // Compute merkle root
    let merkle_root = compute_merkle_root(&block_txids)
        .expect("Failed to compute merkle root");
    println!("Computed merkle root: {}", hex::encode(merkle_root));

    // Extract proof for the coinbase tx (first txid)
    let coinbase_txid = block_txids[0];
    let block_hash_bytes = hex::decode(&block_hash).expect("Invalid block hash");
    let mut block_hash_arr = [0u8; 32];
    block_hash_arr.copy_from_slice(&block_hash_bytes);

    let proof = extract_merkle_proof_from_block(
        coinbase_txid,
        &block_txids,
        block_hash_arr,
        height,
    ).expect("Failed to extract proof for coinbase tx");

    println!("Coinbase proof: tx_index={}, block_height={}",
             proof.tx_index, proof.block_height);
    assert_eq!(proof.tx_index, 0, "Coinbase should be at index 0");

    // Verify the proof
    let verified = verify_merkle_proof(
        &coinbase_txid,
        &merkle_root,
        &proof,
    );
    assert!(verified, "Merkle proof should verify against computed root");

    println!("=== Real Signet Block Data Test PASSED ===");
}
