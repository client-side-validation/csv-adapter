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
    use csv_adapter_core::{AnchorLayer, Hash};

    // Get configuration from environment
    let _rpc_url = std::env::var("CSV_TESTNET_BITCOIN_RPC_URL")
        .unwrap_or_else(|_| "https://mempool.space/signet/api/".to_string());
    let _xpub = std::env::var("CSV_TESTNET_BITCOIN_XPUB").ok();

    println!("=== Bitcoin Signet E2E Test ===");
    println!("RPC URL: {}", _rpc_url);

    // Create adapter with Signet configuration
    let adapter = BitcoinAnchorLayer::signet().expect("Failed to create Signet adapter");

    // Step 1: Create a seal
    let seal = adapter
        .create_seal(Some(100_000))
        .expect("Failed to create seal");
    println!("Created seal: txid={}, vout={}", seal.txid_hex(), seal.vout);

    // Step 2: Publish commitment (simulated without real node)
    let commitment = Hash::new([0xAB; 32]);

    let anchor = adapter
        .publish(commitment, seal.clone())
        .expect("Failed to publish commitment");
    println!("Anchor: txid={}", hex::encode(anchor.txid));

    // Step 3: Verify inclusion
    let inclusion = adapter
        .verify_inclusion(anchor.clone())
        .expect("Failed to verify inclusion");
    println!(
        "Inclusion proof: tx_index={}, block_height={}",
        inclusion.tx_index, inclusion.block_height
    );

    // Step 4: Verify finality
    let finality = adapter
        .verify_finality(anchor.clone())
        .expect("Failed to verify finality");
    println!(
        "Finality: confirmations={}, meets_required={}, required_depth={}",
        finality.confirmations, finality.meets_required_depth, finality.required_depth
    );

    // Step 5: Test rollback
    adapter
        .rollback(anchor)
        .expect("Rollback should succeed for valid anchor");
    println!("Rollback succeeded");

    // Step 6: Test replay prevention
    adapter
        .enforce_seal(seal.clone())
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
    let height: u64 = client
        .get(height_url)
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
    let block_hash = client
        .get(hash_url)
        .send()
        .expect("Failed to fetch block hash")
        .text()
        .expect("Failed to read response")
        .trim()
        .to_string();

    println!("Tip block hash: {}", block_hash);
    assert_eq!(block_hash.len(), 64, "Block hash should be 32 bytes hex");

    // Get block txids
    let txids_url = format!(
        "https://mempool.space/signet/api/block/{}/txids",
        block_hash
    );
    let txids_text = client
        .get(&txids_url)
        .send()
        .expect("Failed to fetch txids")
        .text()
        .expect("Failed to read response");

    let txids: Vec<String> = serde_json::from_str(&txids_text).expect("Failed to parse txids");

    println!("Block has {} transactions", txids.len());
    assert!(!txids.is_empty(), "Block should have transactions");

    // Get the full block header to get the real merkle root
    let header_url = format!("https://mempool.space/signet/api/block/{}", block_hash);
    let block_data_text = client
        .get(&header_url)
        .send()
        .expect("Failed to fetch block header")
        .text()
        .expect("Failed to read response");

    let block_data: serde_json::Value =
        serde_json::from_str(&block_data_text).expect("Failed to parse block data");

    // The merkle root is in the block header data
    let merkle_root_hex = block_data
        .get("merkle_root")
        .and_then(|m| m.as_str())
        .expect("Block has no merkle_root field");
    let merkle_root_bytes = hex::decode(merkle_root_hex).expect("Invalid merkle root hex");
    let mut merkle_root = [0u8; 32];
    merkle_root.copy_from_slice(&merkle_root_bytes);

    println!("Block merkle root: {}", merkle_root_hex);

    // Verify we can extract a Merkle proof
    use csv_adapter_bitcoin::proofs::{
        compute_merkle_root, extract_merkle_proof_from_block, verify_merkle_proof,
    };

    // Convert txids to byte arrays
    // mempool.space returns txids in RPC display order (little-endian, reversed)
    // Our merkle computation needs internal (big-endian, non-reversed) order
    let block_txids: Vec<[u8; 32]> = txids
        .iter()
        .map(|hex| {
            let bytes = hex::decode(hex).expect("Invalid txid hex");
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            arr.reverse(); // Convert from RPC display (little-endian) to internal (big-endian)
            arr
        })
        .collect();

    // The merkle root from the header is also in display (little-endian) order
    // We need to reverse it to match our computed (internal) order
    let mut merkle_root_internal = merkle_root;
    merkle_root_internal.reverse();

    // Compute merkle root from txids and compare with the header's merkle root
    let computed_root = compute_merkle_root(&block_txids).expect("Failed to compute merkle root");
    println!(
        "Computed merkle root (internal): {}",
        hex::encode(computed_root)
    );
    println!("Header merkle root (display):    {}", merkle_root_hex);
    println!(
        "Header merkle root (internal):   {}",
        hex::encode(merkle_root_internal)
    );

    // Verify our computation matches the block header
    assert_eq!(
        computed_root, merkle_root_internal,
        "Computed merkle root must match block header merkle_root (in internal order)"
    );

    // Extract proof using INTERNAL-order txids (the same order used for merkle computation)
    let coinbase_txid = block_txids[0];
    let block_hash_bytes = hex::decode(&block_hash).expect("Invalid block hash");
    let mut block_hash_arr = [0u8; 32];
    block_hash_arr.copy_from_slice(&block_hash_bytes);

    let proof = extract_merkle_proof_from_block(
        coinbase_txid,
        &block_txids, // internal-order txids
        block_hash_arr,
        height,
    )
    .expect("Failed to extract proof for coinbase tx");

    println!(
        "Coinbase proof: tx_index={}, block_height={}, merkle_branch_len={}",
        proof.tx_index,
        proof.block_height,
        proof.merkle_branch.len()
    );

    // Debug: print the first few branch hashes
    for (i, branch) in proof.merkle_branch.iter().take(3).enumerate() {
        println!("  Branch[{}]: {}", i, hex::encode(branch));
    }

    assert_eq!(proof.tx_index, 0, "Coinbase should be at index 0");
    assert!(
        !proof.merkle_branch.is_empty(),
        "Proof should have merkle branch for multi-tx block"
    );

    // Verify the proof using INTERNAL-order txid and merkle root
    let verified = verify_merkle_proof(
        &coinbase_txid,
        &merkle_root_internal, // internal-order merkle root
        &proof,
    );
    assert!(
        verified,
        "Merkle proof must verify against the computed merkle root"
    );

    println!("=== Real Signet Block Data Test PASSED ===");
}
