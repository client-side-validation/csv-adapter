//! Single real integration test against Bitcoin Signet
//!
//! This is the one test that proves the system works against a live network.
//! It connects to a public Signet API, fetches real block data, and verifies
//! that Merkle proof extraction and verification work on real transactions.
//!
//! Run with: cargo test --test signet_integration -- --ignored

#[test]
#[ignore] // Requires internet access
fn test_signet_real_merkle_proof() {
    use csv_adapter_bitcoin::proofs::{
        compute_merkle_root, extract_merkle_proof_from_block, verify_merkle_proof,
    };

    // Fetch a real Signet block header and txids via public API
    // Using mempool.space Signet API
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    // 1. Get the latest block height
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

    assert!(height > 0, "Signet should have blocks");

    // 2. Get the block hash for this height
    let hash_url = format!("https://mempool.space/signet/api/block-height/{}", height);
    let block_hash_hex = client
        .get(&hash_url)
        .send()
        .expect("Failed to fetch block hash")
        .text()
        .expect("Failed to read response")
        .trim()
        .to_string();

    let block_hash_bytes = hex::decode(&block_hash_hex).expect("Failed to decode block hash");
    assert_eq!(block_hash_bytes.len(), 32);
    let mut block_hash = [0u8; 32];
    block_hash.copy_from_slice(&block_hash_bytes);

    // 3. Get the block's txids
    let txids_url = format!(
        "https://mempool.space/signet/api/block/{}/txids",
        block_hash_hex
    );
    let txids_response = client
        .get(&txids_url)
        .send()
        .expect("Failed to fetch txids")
        .text()
        .expect("Failed to read response");

    let txids_hex: Vec<String> =
        serde_json::from_str(&txids_response).expect("Failed to parse txids");

    assert!(!txids_hex.is_empty(), "Block should have transactions");

    // Parse txids into byte arrays
    let txids: Vec<[u8; 32]> = txids_hex
        .iter()
        .map(|hex_str| {
            let bytes = hex::decode(hex_str).expect("Failed to decode txid");
            // Bitcoin txids are displayed in reversed byte order
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            arr.reverse(); // Convert from RPC display order to internal order
            arr
        })
        .collect();

    // 4. Compute the real merkle root from these txids
    let merkle_root = compute_merkle_root(&txids).expect("Should compute root");
    assert_ne!(merkle_root, [0u8; 32], "Merkle root should be non-zero");

    // 5. Extract a proof for the coinbase transaction (first txid)
    let coinbase_txid = txids[0];
    let proof = extract_merkle_proof_from_block(coinbase_txid, &txids, block_hash, height)
        .expect("Should extract proof for coinbase tx");

    // Verify the proof has the correct tx index
    assert_eq!(proof.tx_index, 0, "Coinbase should be at index 0");
    assert_eq!(proof.block_height, height);
    assert_eq!(proof.block_hash, block_hash);

    // 6. Verify the proof against the computed merkle root
    let verified = verify_merkle_proof(&coinbase_txid, &merkle_root, &proof);

    assert!(
        verified,
        "Merkle proof should verify against real block data"
    );

    // 7. If block has more than one tx, test a non-coinbase tx
    if txids.len() > 1 {
        let second_tx = txids[1];
        let proof2 = extract_merkle_proof_from_block(second_tx, &txids, block_hash, height)
            .expect("Should extract proof for second tx");

        assert_eq!(proof2.tx_index, 1, "Second tx should be at index 1");

        let verified2 = verify_merkle_proof(&second_tx, &merkle_root, &proof2);

        assert!(verified2, "Second tx proof should also verify");
    }
}
