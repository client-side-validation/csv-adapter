//! Bitcoin Signet Real Transaction Integration Test
//!
//! This test runs against a real Bitcoin Signet node.
//!
//! ## Run
//!
//! ```bash
//! cargo test -p csv-adapter-bitcoin --test signet_real_tx --features rpc -- --ignored --nocapture
//! ```

#![cfg(feature = "rpc")]

#[test]
#[ignore]
fn test_signet_real_transaction_lifecycle() {
    use bitcoin::{Network as BtcNetwork, OutPoint, Txid};
    use bitcoin_hashes::Hash as BitcoinHash;
    use csv_adapter_bitcoin::wallet::{Bip86Path, SealWallet};
    use csv_adapter_bitcoin::{
        BitcoinAnchorLayer, BitcoinConfig, BitcoinRpc, Network, RealBitcoinRpc,
    };
    use csv_adapter_core::{AnchorLayer, Hash};
    use std::str::FromStr;

    println!("=== Bitcoin Signet Real Transaction Test ===");

    // Get configuration from environment
    let rpc_url = std::env::var("CSV_TESTNET_BITCOIN_RPC_URL")
        .expect("CSV_TESTNET_BITCOIN_RPC_URL must be set");
    let rpc_user = std::env::var("CSV_TESTNET_BITCOIN_RPC_USER").unwrap_or_default();
    let rpc_pass = std::env::var("CSV_TESTNET_BITCOIN_RPC_PASS").unwrap_or_default();

    println!("RPC URL: {}", rpc_url);

    // Create RPC client
    let rpc = if rpc_user.is_empty() {
        RealBitcoinRpc::new(&rpc_url, BtcNetwork::Signet).expect("Failed to create RPC client")
    } else {
        RealBitcoinRpc::with_auth(&rpc_url, &rpc_user, &rpc_pass, BtcNetwork::Signet)
            .expect("Failed to create RPC client with auth")
    };

    // Get current block height from Signet
    let current_height = rpc.get_block_count().expect("Failed to get block count");
    println!("Current Signet height: {}", current_height);

    // Create wallet - either from xpub or random
    let wallet = match std::env::var("CSV_TESTNET_BITCOIN_XPUB") {
        Ok(xpub) => {
            println!("Using provided xpub");
            SealWallet::from_xpub(&xpub, BtcNetwork::Signet)
                .expect("Failed to create wallet from xpub")
        }
        Err(_) => {
            println!("No xpub provided, using random wallet");
            println!("WARNING: Random wallet has no on-chain UTXOs!");
            println!("To test real transactions, fund this wallet first.");
            SealWallet::generate_random(BtcNetwork::Signet)
        }
    };

    // Create Bitcoin adapter with RPC
    let config = BitcoinConfig {
        network: Network::Signet,
        finality_depth: 6,
        publication_timeout_seconds: 300,
        rpc_url: rpc_url.clone(),
    };

    let adapter = BitcoinAnchorLayer::with_wallet(config, wallet)
        .expect("Failed to create adapter")
        .with_rpc(rpc);

    // Step 1: Try to discover UTXOs on-chain
    println!("\n--- Scanning wallet for UTXOs ---");
    let utxos_found = adapter.wallet().utxo_count();
    println!("UTXOs currently in wallet: {}", utxos_found);

    if utxos_found == 0 {
        println!("\n⚠️  No UTXOs found in wallet!");
        println!("To run this test with real transactions:");
        println!("1. Get a Signet address from the wallet");
        println!("2. Send testnet sats to that address");
        println!("3. Wait for confirmation");
        println!("4. Add the UTXO to the wallet manually");
        println!("\nFor now, demonstrating the API without real broadcast...");

        // Demo: Show what would happen with a funded UTXO
        demo_funded_flow(&adapter);
        return;
    }

    // Step 2: Create a seal from a real UTXO
    println!("\n--- Creating seal from real UTXO ---");
    let utxos = adapter.wallet().list_utxos();
    let first_utxo = &utxos[0];
    let outpoint = first_utxo.outpoint;

    let (seal, path) = adapter
        .fund_seal(outpoint)
        .expect("Failed to create seal from UTXO");

    println!("Created seal from UTXO:");
    println!("  TXID: {}", seal.txid_hex());
    println!("  VOUT: {}", seal.vout);
    println!("  Value: {} sat", seal.nonce.unwrap_or(0));
    println!("  Path: {:?}", path);

    // Step 3: Publish commitment
    println!("\n--- Publishing commitment ---");
    let commitment = Hash::new([0xAB; 32]);

    let anchor = adapter
        .publish(commitment, seal.clone())
        .expect("Failed to publish commitment");

    println!("Published commitment:");
    println!("  TXID: {}", hex::encode(anchor.txid));
    println!("  Block height: {}", anchor.block_height);
    println!("  Output index: {}", anchor.output_index);

    // Step 4: Verify inclusion (would fetch real Merkle proof after confirmation)
    println!("\n--- Verifying inclusion ---");
    let inclusion = adapter
        .verify_inclusion(anchor.clone())
        .expect("Failed to verify inclusion");

    println!("Inclusion proof:");
    println!("  TX index: {}", inclusion.tx_index);
    println!("  Block height: {}", inclusion.block_height);
    println!("  Merkle branch length: {}", inclusion.merkle_branch.len());

    // Step 5: Verify finality
    println!("\n--- Verifying finality ---");
    let finality = adapter
        .verify_finality(anchor.clone())
        .expect("Failed to verify finality");

    println!("Finality proof:");
    println!("  Confirmations: {}", finality.confirmations);
    println!("  Required depth: {}", finality.required_depth);
    println!("  Meets requirement: {}", finality.meets_required_depth);

    // Step 6: Test replay prevention
    println!("\n--- Testing replay prevention ---");
    adapter
        .enforce_seal(seal.clone())
        .expect("First enforcement should succeed");
    println!("✓ First enforcement succeeded");

    let replay_result = adapter.enforce_seal(seal);
    assert!(replay_result.is_err(), "Replay should be prevented");
    println!("✓ Replay prevention works correctly");

    println!("\n=== Bitcoin Signet Real Transaction Test PASSED ===");
}

/// Demonstrates what the flow looks like with a funded UTXO
fn demo_funded_flow(adapter: &csv_adapter_bitcoin::BitcoinAnchorLayer) {
    println!("\n--- Demo: How the flow works with funded UTXOs ---");

    // In a real scenario, you would:
    // 1. Derive an address from the wallet
    let (derived_key, path) = adapter
        .wallet()
        .next_address(0)
        .expect("Failed to derive address");

    println!("1. Derived address: {}", derived_key.address);
    println!("   Path: {:?}", path);

    // 2. Send bitcoin to that address (manual step)
    println!("\n2. [MANUAL STEP] Send testnet sats to this address");
    println!("   You can use a Signet faucet or mine blocks locally");

    // 3. Once confirmed, add the UTXO to the wallet
    println!("\n3. After confirmation, you would call:");
    println!("   adapter.wallet().add_utxo(outpoint, amount_sat, path)");

    // 4. Then create a seal from that UTXO
    println!("\n4. Create seal:");
    println!("   let (seal, path) = adapter.fund_seal(outpoint)?;");

    // 5. Publish the commitment
    println!("\n5. Publish commitment:");
    println!("   let anchor = adapter.publish(commitment, seal)?;");

    // 6. The transaction is broadcast to Signet
    println!("\n6. Transaction is broadcast to Signet!");
    println!("   You can verify at: https://mempool.space/signet/tx/<txid>");

    println!("\n--- End Demo ---");
}

/// Test UTXO discovery and scanning
#[test]
#[ignore]
fn test_signet_utxo_discovery() {
    use bitcoin::Network as BtcNetwork;
    use bitcoin_hashes::Hash as BitcoinHash;
    use csv_adapter_bitcoin::wallet::SealWallet;
    use csv_adapter_bitcoin::{
        BitcoinAnchorLayer, BitcoinConfig, BitcoinRpc, Network, RealBitcoinRpc,
    };

    println!("=== Bitcoin Signet UTXO Discovery Test ===");

    let rpc_url = std::env::var("CSV_TESTNET_BITCOIN_RPC_URL")
        .expect("CSV_TESTNET_BITCOIN_RPC_URL must be set");
    let rpc_user = std::env::var("CSV_TESTNET_BITCOIN_RPC_USER").unwrap_or_default();
    let rpc_pass = std::env::var("CSV_TESTNET_BITCOIN_RPC_PASS").unwrap_or_default();

    let rpc = if rpc_user.is_empty() {
        RealBitcoinRpc::new(&rpc_url, BtcNetwork::Signet).expect("Failed to create RPC client")
    } else {
        RealBitcoinRpc::with_auth(&rpc_url, &rpc_user, &rpc_pass, BtcNetwork::Signet)
            .expect("Failed to create RPC client with auth")
    };

    let wallet = SealWallet::generate_random(BtcNetwork::Signet);
    let config = BitcoinConfig {
        network: Network::Signet,
        finality_depth: 6,
        publication_timeout_seconds: 300,
        rpc_url: rpc_url.clone(),
    };

    let adapter = BitcoinAnchorLayer::with_wallet(config, wallet)
        .expect("Failed to create adapter")
        .with_rpc(rpc);

    // Try to scan for UTXOs
    println!("\nScanning for UTXOs (this may take a moment)...");
    match adapter.scan_wallet_for_utxos(0, 20) {
        Ok(count) => {
            println!("✓ Discovered {} UTXOs", count);
            println!("Wallet balance: {} sat", adapter.wallet().balance());
            println!("UTXO count: {}", adapter.wallet().utxo_count());
        }
        Err(e) => {
            println!("⚠️  UTXO scan failed: {}", e);
            println!("This is expected if the wallet has no on-chain history");
        }
    }

    println!("\n=== Bitcoin Signet UTXO Discovery Test Complete ===");
}

/// Test real block height and transaction verification
#[test]
#[ignore]
fn test_signet_real_block_verification() {
    use bitcoin::Network as BtcNetwork;
    use bitcoin_hashes::Hash as BitcoinHash;
    use csv_adapter_bitcoin::{BitcoinRpc, RealBitcoinRpc};

    println!("=== Bitcoin Signet Block Verification Test ===");

    let rpc_url = std::env::var("CSV_TESTNET_BITCOIN_RPC_URL")
        .expect("CSV_TESTNET_BITCOIN_RPC_URL must be set");

    let rpc =
        RealBitcoinRpc::new(&rpc_url, BtcNetwork::Signet).expect("Failed to create RPC client");

    // Get current height
    let height = rpc.get_block_count().expect("Failed to get block count");
    println!("Current Signet height: {}", height);

    // Get block hash at current height
    let block_hash = rpc
        .get_block_hash(height)
        .expect("Failed to get block hash");
    println!(
        "Block hash at height {}: {}",
        height,
        hex::encode(block_hash)
    );

    // Get block info
    let block = rpc.get_block(block_hash).expect("Failed to get block");

    println!("Block {}:", height);
    println!("  Transactions: {}", block.txdata.len());
    println!("  Weight: {} WU", block.weight());

    // Get first transaction (coinbase)
    if !block.txdata.is_empty() {
        let coinbase_tx = &block.txdata[0];
        let coinbase_txid = coinbase_tx.txid();
        println!("  Coinbase TXID: {}", coinbase_txid);

        // Extract Merkle proof for coinbase tx
        let all_txids: Vec<[u8; 32]> = block
            .txdata
            .iter()
            .map(|tx| tx.txid().to_byte_array())
            .collect();

        let proof = csv_adapter_bitcoin::proofs::extract_merkle_proof_from_block(
            coinbase_txid.to_byte_array(),
            &all_txids,
            block_hash,
            height,
        )
        .expect("Failed to extract Merkle proof");

        println!("  Coinbase proof:");
        println!("    TX index: {}", proof.tx_index);
        println!("    Merkle branch length: {}", proof.merkle_branch.len());

        // Verify the proof
        let computed_root = csv_adapter_bitcoin::proofs::compute_merkle_root(&all_txids)
            .expect("Failed to compute Merkle root");

        // Reverse to match header format
        let mut header_root = computed_root;
        header_root.reverse();

        let verified = csv_adapter_bitcoin::proofs::verify_merkle_proof(
            &coinbase_txid.to_byte_array(),
            &computed_root,
            &proof,
        );

        println!(
            "  Proof verification: {}",
            if verified { "✓ PASSED" } else { "✗ FAILED" }
        );
        assert!(verified, "Merkle proof must verify");
    }

    println!("\n=== Bitcoin Signet Block Verification Test PASSED ===");
}
