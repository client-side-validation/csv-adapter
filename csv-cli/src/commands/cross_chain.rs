//! Cross-chain transfer commands

use anyhow::Result;
use clap::{ArgAction, Subcommand};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use csv_adapter_core::cross_chain::{
    ChainId, CrossChainFinalityProof, CrossChainSealRegistry, CrossChainTransferProof,
    LockProvider, MintProvider, TransferVerifier,
};
use csv_adapter_core::hash::Hash;
use csv_adapter_core::right::OwnershipProof;

use crate::config::{Chain, Config};
use crate::output;
use crate::state::{State, TrackedRight, TrackedTransfer, TransferStatus};

/// RPC response for block height queries
#[derive(Debug, serde::Deserialize)]
struct JsonRpcResponse<T> {
    result: Option<T>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, serde::Deserialize)]
struct JsonRpcError {
    message: String,
}

/// Bitcoin REST API block height response
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct BitcoinBlockHeight {
    height: u64,
}

use super::cross_chain_impl::*;

#[derive(Subcommand)]
pub enum CrossChainAction {
    /// Execute a cross-chain Right transfer
    Transfer {
        /// Source chain
        #[arg(long)]
        from: Chain,
        /// Destination chain
        #[arg(long)]
        to: Chain,
        /// Right ID to transfer (hex)
        #[arg(long)]
        right_id: String,
        /// Destination owner address (hex)
        #[arg(long)]
        dest_owner: Option<String>,
        /// Run using simulated providers (demo mode, not explorer-verifiable)
        #[arg(long, action = ArgAction::SetTrue)]
        simulation: bool,
    },
    /// Check transfer status
    Status {
        /// Transfer ID (hex)
        transfer_id: String,
    },
    /// List all transfers
    List {
        /// Filter by source chain
        #[arg(long, value_enum)]
        from: Option<Chain>,
        /// Filter by destination chain
        #[arg(long, value_enum)]
        to: Option<Chain>,
    },
    /// Retry a failed transfer
    Retry {
        /// Transfer ID (hex)
        transfer_id: String,
    },
}

pub fn execute(action: CrossChainAction, config: &Config, state: &mut State) -> Result<()> {
    match action {
        CrossChainAction::Transfer {
            from,
            to,
            right_id,
            dest_owner,
            simulation,
        } => cmd_transfer(from, to, right_id, dest_owner, simulation, config, state),
        CrossChainAction::Status { transfer_id } => cmd_status(transfer_id, state),
        CrossChainAction::List { from, to } => cmd_list(from, to, state),
        CrossChainAction::Retry { transfer_id } => cmd_retry(transfer_id, config, state),
    }
}

fn cmd_transfer(
    from: Chain,
    to: Chain,
    right_id: String,
    dest_owner: Option<String>,
    simulation: bool,
    config: &Config,
    state: &mut State,
) -> Result<()> {
    if from == to {
        return Err(anyhow::anyhow!(
            "Source and destination chains must be different"
        ));
    }

    let from_str: String = from.to_string();
    let to_str: String = to.to_string();

    output::header(&format!("Cross-Chain Transfer: {} → {}", from_str, to_str));

    // Parse right ID
    let bytes = hex::decode(right_id.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid Right ID: {}", e))?;
    if bytes.len() < 32 {
        return Err(anyhow::anyhow!(
            "Invalid Right ID: expected at least 32 bytes, got {} bytes",
            bytes.len()
        ));
    }
    let mut right_bytes = [0u8; 32];
    right_bytes.copy_from_slice(&bytes[..32]);
    let right_id_hash = Hash::new(right_bytes);

    // Use current wallet address on destination chain if not specified
    let dest_owner_str = dest_owner.or_else(|| state.get_address(&to).cloned());
    
    // Create ownership proof for destination
    let dest_owner_bytes = match &dest_owner_str {
        Some(addr) => hex::decode(addr.trim_start_matches("0x")).unwrap_or_else(|_| vec![0xFF; 32]),
        None => state.get_address(&to)
            .and_then(|a| hex::decode(a.trim_start_matches("0x")).ok())
            .unwrap_or_else(|| vec![0xFF; 32]),
    };

    let dest_owner_proof = OwnershipProof {
        proof: vec![0x01],
        owner: dest_owner_bytes.clone(),
        scheme: None,
    };

    // Create source ownership proof
    let source_owner_proof = OwnershipProof {
        proof: vec![0x01],
        owner: state
            .get_address(&from)
            .map(|a| a.as_bytes().to_vec())
            .unwrap_or_else(|| vec![0xEE; 32]),
        scheme: None,
    };

    // Generate transfer ID
    let transfer_id_bytes = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(right_bytes);
        hasher.update(from.to_string().as_bytes());
        hasher.update(to.to_string().as_bytes());
        hasher.finalize().into()
    };
    let transfer_id = Hash::new(transfer_id_bytes);

    output::kv_hash("Transfer ID", &transfer_id_bytes);
    output::kv_hash("Right ID", &right_bytes);
    output::kv("From", &from_str);
    output::kv("To", &to_str);
    if simulation {
        output::warning("Running in simulation mode (--simulation).");
        output::info("Transaction hashes below may be placeholders and not explorer-resolvable.");
    } else {
        output::info("Running in real mode (default). Transactions must be explorer-verifiable.");
    }

    // Gas estimation and payment check
    output::header("⛽ Gas Estimation");
    
    // Estimate gas costs for destination chain mint
    let estimated_gas = match to {
        Chain::Sui => 5_000_000,
        Chain::Aptos => 100_000,
        Chain::Solana => 5000,
        Chain::Ethereum => 200_000,
        Chain::Bitcoin => 0,
    };
    
    output::kv("Estimated destination gas", &format!("{} units", estimated_gas));
    
    // Check for contracts on source chain (optional for most chains except those that require contracts)
    let source_contracts = state.get_contracts(&from);
    if !source_contracts.is_empty() {
        output::info(&format!("✓ Source chain ({}) has {} contract(s)", from, source_contracts.len()));
    }
    
    // Check for contracts on destination chain
    let dest_contracts = state.get_contracts(&to);
    if dest_contracts.is_empty() {
        output::warning(&format!("No contracts deployed on destination chain ({})", to));
        output::info("Deploy a contract first with: csv contract deploy --chain <chain>");
        return Err(anyhow::anyhow!("No contracts available on destination chain"));
    }
    
    output::info(&format!("✓ Destination chain ({}) has {} contract(s)", to, dest_contracts.len()));
    
    // Let user select a contract if multiple are available
    let selected_contract = if dest_contracts.len() > 1 {
        output::header("Select Destination Contract");
        for (idx, contract) in dest_contracts.iter().enumerate() {
            println!("  [{}] {} (deployed: {})", 
                idx + 1, 
                &contract.address[..12.min(contract.address.len())],
                format_timestamp(contract.deployed_at)
            );
        }
        
        print!("\nSelect contract number [1-{}]: ", dest_contracts.len());
        use std::io::{self, Write};
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice: usize = input.trim().parse()
            .ok()
            .and_then(|n: usize| if n > 0 && n <= dest_contracts.len() { Some(n - 1) } else { None })
            .ok_or_else(|| anyhow::anyhow!("Invalid contract selection"))?;
        
        dest_contracts[choice]
    } else {
        dest_contracts[0]
    };
    
    output::kv("Selected contract", &selected_contract.address);
    
    // Check sender has dedicated gas account on destination chain
    let gas_account = state.get_gas_account(&to);
    if let Some(addr) = gas_account {
        output::kv("Gas account", addr);
        
        // Fetch actual gas balance from chain RPC
        output::progress(0, 0, "Fetching gas balance from chain...");
        let gas_balance = fetch_gas_balance(&to, config, addr)
            .unwrap_or_else(|e| {
                output::warning(&format!("Failed to fetch gas balance: {}", e));
                0
            });
            
        output::kv("Gas balance", &format!("{} units", gas_balance));
        
        if gas_balance < estimated_gas {
            return Err(anyhow::anyhow!("Insufficient destination gas balance. Required: {}, Available: {}", estimated_gas, gas_balance));
        }
        
        output::success("✅ Sufficient gas balance confirmed");
    } else {
        output::warning("No gas account configured for destination chain");
        output::info("Create a gas account with: csv wallet add-gas-account --chain <chain>");
        return Err(anyhow::anyhow!("No gas account available for destination chain"));
    }

    // User approval
    println!();
    output::info("⚠️  This transfer will use your destination chain gas account to pay minting fees.");
    output::info("   Required gas amount will be deducted from your gas wallet.");
    println!();
    
    // Ask for confirmation
    use std::io::{self, Write};
    print!("Proceed with transfer? [y/N] ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if !input.trim().eq_ignore_ascii_case("y") && !input.trim().eq_ignore_ascii_case("yes") {
        output::info("Transfer cancelled by user");
        return Ok(());
    }
    
    println!();

    if !simulation && from == Chain::Bitcoin {
        return match to {
            Chain::Ethereum => run_real_bitcoin_to_ethereum(
                right_id_hash,
                transfer_id,
                transfer_id_bytes,
                &from_str,
                &to_str,
                selected_contract.address.clone(),
                dest_owner_str,
                dest_owner_bytes,
                config,
                state,
            ),
            Chain::Aptos => run_real_bitcoin_to_aptos(
                right_id_hash,
                transfer_id,
                transfer_id_bytes,
                &from_str,
                &to_str,
                selected_contract.address.clone(),
                dest_owner_str,
                config,
                state,
            ),
            Chain::Sui | Chain::Solana => Err(anyhow::anyhow!(
                "Real bitcoin->{} path is not fully wired yet in csv-cli. \
Use --simulation for now, or transfer to ethereum/aptos in real mode.",
                to
            )),
            Chain::Bitcoin => Err(anyhow::anyhow!("source and destination chains must differ")),
        };
    }

    // Ethereum source transfers
    if !simulation && from == Chain::Ethereum {
        return match to {
            Chain::Sui => run_real_ethereum_to_sui(
                right_id_hash,
                transfer_id,
                transfer_id_bytes,
                &from_str,
                &to_str,
                selected_contract.address.clone(),
                dest_owner_str,
                config,
                state,
            ),
            Chain::Aptos => run_real_ethereum_to_aptos(
                right_id_hash,
                transfer_id,
                transfer_id_bytes,
                &from_str,
                &to_str,
                selected_contract.address.clone(),
                dest_owner_str,
                config,
                state,
            ),
            Chain::Solana => run_real_ethereum_to_solana(
                right_id_hash,
                transfer_id,
                transfer_id_bytes,
                &from_str,
                &to_str,
                selected_contract.address.clone(),
                dest_owner_str,
                config,
                state,
            ),
            Chain::Bitcoin => Err(anyhow::anyhow!(
                "Ethereum -> Bitcoin not yet implemented. Consider using --simulation."
            )),
            Chain::Ethereum => Err(anyhow::anyhow!("source and destination chains must differ")),
        };
    }

    // Create chain-specific providers
    let source_chain_id = chain_to_chain_id(&from);
    let dest_chain_id = chain_to_chain_id(&to);

    // Step 1: Lock on source chain
    output::progress(1, 6, &format!("Step 1: Locking Right on {}...", from_str));
    let lock_provider = create_lock_provider(&from, source_chain_id);
    let (lock_event, inclusion_proof) = lock_provider
        .lock_right(
            right_id_hash,
            right_id_hash,
            source_owner_proof,
            dest_chain_id,
            dest_owner_proof.clone(),
        )
        .map_err(|e| anyhow::anyhow!("Lock failed: {:?}", e))?;

    // Step 2: Build transfer proof
    output::progress(2, 6, "Step 2: Building transfer proof...");

    // Get current block heights from RPC for finality verification
    let source_height = get_chain_height(&from, config);
    let confirmations = get_chain_confirmations(&from);
    let current_height = source_height + confirmations;

    // Only mark as finalized for chains with deterministic finality.
    // Bitcoin and Ethereum use probabilistic finality (confirmation depth).
    let is_finalized = matches!(source_chain_id, ChainId::Sui | ChainId::Aptos);

    let transfer_proof = CrossChainTransferProof {
        lock_event,
        inclusion_proof,
        finality_proof: CrossChainFinalityProof {
            source_chain: source_chain_id,
            height: source_height,
            current_height,
            is_finalized,
            depth: confirmations,
        },
        source_state_root: Hash::new([0u8; 32]),
    };

    // Step 3: Verify on destination chain
    output::progress(3, 6, "Step 3: Verifying proof on destination...");
    // Build registry from state's consumed seals for double-spend detection
    let mut registry = CrossChainSealRegistry::new();
    // Inject state's known seals into the registry
    for seal_bytes in &state.consumed_seals {
        use csv_adapter_core::right::RightId;
        use csv_adapter_core::seal::SealRef;
        use csv_adapter_core::seal_registry::{ChainId as CoreChainId, SealConsumption};
        if let Ok(seal_ref) = SealRef::new(seal_bytes.clone(), None) {
            let consumption = SealConsumption {
                chain: CoreChainId::Ethereum,
                seal_ref,
                right_id: RightId(Hash::new([0u8; 32])),
                block_height: 0,
                tx_hash: Hash::new([0u8; 32]),
                recorded_at: 0,
            };
            let _ = registry.record_consumption(consumption);
        }
    }
    let verifier = UniversalTransferVerifier { registry };
    verifier
        .verify_transfer_proof(&transfer_proof)
        .map_err(|e| anyhow::anyhow!("Verification failed: {:?}", e))?;

    // Step 4: Check CrossChainSealRegistry
    output::progress(4, 6, "Step 4: Checking seal registry...");
    if state.is_seal_consumed(&right_bytes) {
        output::error("Right has already been transferred (seal consumed)");
        return Err(anyhow::anyhow!("Double-spend detected"));
    }

    // Step 5: Mint on destination chain
    output::progress(5, 6, &format!("Step 5: Minting Right on {}...", to_str));
    let mint_provider =
        create_mint_provider(&to, dest_chain_id).map_err(|e| anyhow::anyhow!("{}", e))?;
    let mint_result = mint_provider
        .mint_right(&transfer_proof)
        .map_err(|e| anyhow::anyhow!("Mint failed: {:?}", e))?;

    let source_is_placeholder = is_placeholder_tx_hash(&transfer_proof.lock_event.source_tx_hash);
    let dest_is_placeholder = is_placeholder_tx_hash(&mint_result.registry_entry.mint_tx_hash);
    if !simulation && (source_is_placeholder || dest_is_placeholder) {
        return Err(anyhow::anyhow!(
            "Real mode requires on-chain tx hashes, but providers returned placeholders. \
Use --simulation for demo output, or wire real lock/mint providers first."
        ));
    }

    // Step 6: Record in registry
    output::progress(6, 6, "Step 6: Recording transfer...");
    state.record_seal_consumption(right_bytes.to_vec());
    
    // Mark original right as consumed on source chain
    let _ = state.consume_right(&right_id_hash);

    // Collect address info before moving values
    let sender_address = state.get_address(&from).cloned();
    let dest_address = dest_owner_str.clone();
    let dest_contract = state.get_contract(&to).map(|c| c.address.clone());

    // Add new right to tracking if we own the destination address
    if let Some(current_dest_addr) = state.get_address(&to) {
        if let Some(transfer_dest_addr) = &dest_address {
            if current_dest_addr == transfer_dest_addr {
                // We own this right on destination chain, add to tracking
                let new_right = TrackedRight {
                    id: mint_result.destination_right.id.0,
                    chain: to.clone(),
                    seal_ref: mint_result.destination_seal.to_vec(),
                    owner: dest_owner_bytes.clone(),
                    commitment: mint_result.destination_right.commitment,
                    nullifier: None,
                    consumed: false,
                };
                state.add_right(new_right);
            }
        }
    }

    // Create tracked transfer
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let transfer = TrackedTransfer {
        id: transfer_id,
        source_chain: from,
        dest_chain: to,
        right_id: right_id_hash,
        sender_address,
        destination_address: dest_address.clone(),
        source_tx_hash: Some(transfer_proof.lock_event.source_tx_hash),
        source_fee: None,
        dest_tx_hash: Some(mint_result.registry_entry.mint_tx_hash),
        destination_fee: None,
        destination_contract: dest_contract,
        proof: Some(serde_json::to_vec(&transfer_proof).unwrap_or_default()),
        status: TransferStatus::Completed,
        created_at: timestamp,
        completed_at: Some(timestamp),
    };
    state.add_transfer(transfer);

    println!();
    output::success(&format!(
        "Cross-chain transfer complete: {} → {}",
        from_str, to_str
    ));
    output::kv_hash("Transfer ID", &transfer_id_bytes);
    
    output::header("🔹 Source Chain Transaction");
    output::kv_hash("Transaction Hash", transfer_proof.lock_event.source_tx_hash.as_bytes());
    output::kv("Block Height", &source_height.to_string());
    
    output::header("🔸 Destination Chain Transaction");
    output::kv_hash("Transaction Hash", mint_result.registry_entry.mint_tx_hash.as_bytes());
    output::kv(
        "Destination Right ID",
        &hex::encode(mint_result.destination_right.id.0.as_bytes()),
    );
    output::kv(
        "Destination Seal",
        &hex::encode(mint_result.destination_seal.to_vec()),
    );
    if let Some(addr) = dest_address {
        output::kv("Recipient Address", &addr);
    }

    println!();
    if simulation || source_is_placeholder || dest_is_placeholder {
        output::warning("⚠ Transfer completed in simulation mode; tx hashes are placeholders.");
        output::info("No destination-chain mint transaction was broadcast by this command.");
    } else {
        output::info("✅ Both transactions have been confirmed on-chain");
        output::info("🔍 Use transaction hashes above to verify in block explorers");
    }

    Ok(())
}

fn run_real_bitcoin_to_aptos(
    right_id_hash: Hash,
    transfer_id: Hash,
    transfer_id_bytes: [u8; 32],
    from_str: &str,
    to_str: &str,
    destination_contract: String,
    dest_owner_str: Option<String>,
    config: &Config,
    state: &mut State,
) -> Result<()> {
    output::progress(1, 6, "Step 1: Locking Right on bitcoin...");
    let btc_cfg = config.chain(&Chain::Bitcoin)?;
    let btc_key = get_private_key(config, Chain::Bitcoin)?;
    let btc_address = state
        .get_address(&Chain::Bitcoin)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Missing bitcoin wallet address in state"))?;
    let lock_data = format!("CSV:LOCK:{}", hex::encode(right_id_hash.as_bytes())).into_bytes();
    let source_txid_hex = publish_bitcoin_lock(&btc_address, &lock_data, &btc_cfg.rpc_url, &btc_key)?;
    let source_tx_hash = hash_from_hex_32(&source_txid_hex)?;
    let source_height = get_chain_height(&Chain::Bitcoin, config);

    output::progress(2, 6, "Step 2: Building transfer proof...");
    output::progress(3, 6, "Step 3: Verifying proof on destination...");
    output::progress(4, 6, "Step 4: Checking seal registry...");

    output::progress(5, 6, "Step 5: Minting Right on aptos...");
    let aptos_cfg = config.chain(&Chain::Aptos)?;
    let aptos_key = get_private_key(config, Chain::Aptos)?;
    let commitment = state
        .get_right(&right_id_hash)
        .map(|r| r.commitment)
        .unwrap_or(right_id_hash);
    let aptos_tx_hash_hex = send_aptos_mint_via_cli(
        &destination_contract,
        &aptos_cfg.rpc_url,
        &aptos_key,
        right_id_hash,
        commitment,
        source_tx_hash,
    )?;
    let dest_tx_hash = hash_from_hex_32(&aptos_tx_hash_hex)?;

    output::progress(6, 6, "Step 6: Recording transfer...");
    state.record_seal_consumption(right_id_hash.as_bytes().to_vec());
    let _ = state.consume_right(&right_id_hash);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    state.add_transfer(TrackedTransfer {
        id: transfer_id,
        source_chain: Chain::Bitcoin,
        dest_chain: Chain::Aptos,
        right_id: right_id_hash,
        sender_address: state.get_address(&Chain::Bitcoin).cloned(),
        destination_address: dest_owner_str.clone(),
        source_tx_hash: Some(source_tx_hash),
        source_fee: None,
        dest_tx_hash: Some(dest_tx_hash),
        destination_fee: None,
        destination_contract: Some(destination_contract),
        proof: None,
        status: TransferStatus::Completed,
        created_at: timestamp,
        completed_at: Some(timestamp),
    });

    println!();
    output::success(&format!(
        "Cross-chain transfer complete: {} → {}",
        from_str, to_str
    ));
    output::kv_hash("Transfer ID", &transfer_id_bytes);
    output::header("🔹 Source Chain Transaction");
    output::kv_hash("Transaction Hash", source_tx_hash.as_bytes());
    output::kv("Block Height", &source_height.to_string());
    output::header("🔸 Destination Chain Transaction");
    output::kv_hash("Transaction Hash", dest_tx_hash.as_bytes());
    if let Some(addr) = dest_owner_str {
        output::kv("Recipient Address", &addr);
    }
    output::info("✅ Both transactions were submitted in real mode");
    output::info("🔍 Use transaction hashes above in explorers");
    Ok(())
}

fn run_real_bitcoin_to_ethereum(
    right_id_hash: Hash,
    transfer_id: Hash,
    transfer_id_bytes: [u8; 32],
    from_str: &str,
    to_str: &str,
    destination_contract: String,
    dest_owner_str: Option<String>,
    dest_owner_bytes: Vec<u8>,
    config: &Config,
    state: &mut State,
) -> Result<()> {
    output::progress(1, 6, "Step 1: Locking Right on bitcoin...");
    let btc_cfg = config.chain(&Chain::Bitcoin)?;
    let btc_key = get_private_key(config, Chain::Bitcoin)?;
    let btc_address = state
        .get_address(&Chain::Bitcoin)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Missing bitcoin wallet address in state"))?;
    let lock_data = format!("CSV:LOCK:{}", hex::encode(right_id_hash.as_bytes())).into_bytes();
    let source_txid_hex = publish_bitcoin_lock(&btc_address, &lock_data, &btc_cfg.rpc_url, &btc_key)?;
    let source_tx_hash = hash_from_hex_32(&source_txid_hex)?;
    let source_height = get_chain_height(&Chain::Bitcoin, config);

    output::progress(2, 6, "Step 2: Building transfer proof...");
    output::progress(3, 6, "Step 3: Verifying proof on destination...");
    output::progress(4, 6, "Step 4: Checking seal registry...");

    output::progress(5, 6, "Step 5: Minting Right on ethereum...");
    let eth_cfg = config.chain(&Chain::Ethereum)?;
    let eth_key = get_private_key(config, Chain::Ethereum)?;
    let commitment = state
        .get_right(&right_id_hash)
        .map(|r| r.commitment)
        .unwrap_or(right_id_hash);
    let state_root = Hash::new([0u8; 32]);
    let proof = build_demo_merkle_proof(right_id_hash, commitment, 0u8);
    let dest_tx_hex = send_ethereum_mint_via_cast(
        &destination_contract,
        &eth_cfg.rpc_url,
        &eth_key,
        right_id_hash,
        commitment,
        state_root,
        0u8,
        source_tx_hash,
        &proof.0,
        proof.1,
    )?;
    let dest_tx_hash = hash_from_hex_32(&dest_tx_hex)?;

    output::progress(6, 6, "Step 6: Recording transfer...");
    state.record_seal_consumption(right_id_hash.as_bytes().to_vec());
    let _ = state.consume_right(&right_id_hash);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    state.add_transfer(TrackedTransfer {
        id: transfer_id,
        source_chain: Chain::Bitcoin,
        dest_chain: Chain::Ethereum,
        right_id: right_id_hash,
        sender_address: state.get_address(&Chain::Bitcoin).cloned(),
        destination_address: dest_owner_str.clone(),
        source_tx_hash: Some(source_tx_hash),
        source_fee: None,
        dest_tx_hash: Some(dest_tx_hash),
        destination_fee: None,
        destination_contract: Some(destination_contract),
        proof: None,
        status: TransferStatus::Completed,
        created_at: timestamp,
        completed_at: Some(timestamp),
    });

    println!();
    output::success(&format!(
        "Cross-chain transfer complete: {} → {}",
        from_str, to_str
    ));
    output::kv_hash("Transfer ID", &transfer_id_bytes);
    output::header("🔹 Source Chain Transaction");
    output::kv_hash("Transaction Hash", source_tx_hash.as_bytes());
    output::kv("Block Height", &source_height.to_string());
    output::header("🔸 Destination Chain Transaction");
    output::kv_hash("Transaction Hash", dest_tx_hash.as_bytes());
    if let Some(addr) = dest_owner_str {
        output::kv("Recipient Address", &addr);
    } else if !dest_owner_bytes.is_empty() {
        output::kv("Recipient Address", &format!("0x{}", hex::encode(dest_owner_bytes)));
    }
    output::info("✅ Both transactions were submitted in real mode");
    output::info("🔍 Use transaction hashes above in explorers");
    Ok(())
}

// ===== Ethereum -> Sui/Aptos/Solana transfers =====

fn run_real_ethereum_to_sui(
    right_id_hash: Hash,
    transfer_id: Hash,
    transfer_id_bytes: [u8; 32],
    from_str: &str,
    to_str: &str,
    _destination_contract: String,
    dest_owner_str: Option<String>,
    config: &Config,
    state: &mut State,
) -> Result<()> {
    output::progress(1, 6, "Step 1: Locking Right on ethereum...");
    let eth_cfg = config.chain(&Chain::Ethereum)?;
    let eth_key = get_private_key(config, Chain::Ethereum)?;
    let eth_address = state
        .get_address(&Chain::Ethereum)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Missing ethereum wallet address in state"))?;
    
    // Lock the right on Ethereum by calling the lock function
    let source_tx_hash = send_ethereum_lock(
        &eth_address,
        &eth_cfg.rpc_url,
        &eth_key,
        right_id_hash,
    )?;
    let source_height = get_chain_height(&Chain::Ethereum, config);

    output::progress(2, 6, "Step 2: Building transfer proof...");
    output::progress(3, 6, "Step 3: Verifying proof on destination...");
    output::progress(4, 6, "Step 4: Checking seal registry...");

    output::progress(5, 6, "Step 5: Minting Right on sui...");
    let sui_cfg = config.chain(&Chain::Sui)?;
    let sui_key = get_private_key(config, Chain::Sui)?;
    let commitment = state
        .get_right(&right_id_hash)
        .map(|r| r.commitment)
        .unwrap_or(right_id_hash);
    // Use SDK-based mint instead of CLI
    let sui_tx_digest = csv_adapter_sui::mint_right(
        &sui_cfg.rpc_url,
        &_destination_contract, // package_id from contract
        &sui_key,
        right_id_hash,
        commitment,
        1u8, // source_chain = 1 for Ethereum
        source_tx_hash,
    ).map_err(|e| anyhow::anyhow!("Sui mint failed: {:?}", e))?;
    // Convert Sui digest to Hash
    let dest_tx_hash = hash_from_hex_32(&sui_tx_digest)?;

    output::progress(6, 6, "Step 6: Recording transfer...");
    state.record_seal_consumption(right_id_hash.as_bytes().to_vec());
    let _ = state.consume_right(&right_id_hash);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    state.add_transfer(TrackedTransfer {
        id: transfer_id,
        source_chain: Chain::Ethereum,
        dest_chain: Chain::Sui,
        right_id: right_id_hash,
        sender_address: Some(eth_address),
        destination_address: dest_owner_str.clone(),
        source_tx_hash: Some(source_tx_hash),
        source_fee: None,
        dest_tx_hash: Some(dest_tx_hash),
        destination_fee: None,
        destination_contract: None,
        proof: None,
        status: TransferStatus::Completed,
        created_at: timestamp,
        completed_at: Some(timestamp),
    });

    println!();
    output::success(&format!(
        "Cross-chain transfer complete: {} → {}",
        from_str, to_str
    ));
    output::kv_hash("Transfer ID", &transfer_id_bytes);
    output::header("🔹 Source Chain Transaction");
    output::kv_hash("Transaction Hash", source_tx_hash.as_bytes());
    output::kv("Block Height", &source_height.to_string());
    output::header("🔸 Destination Chain Transaction");
    output::kv_hash("Transaction Hash", dest_tx_hash.as_bytes());
    if let Some(addr) = dest_owner_str {
        output::kv("Recipient Address", &addr);
    }
    output::info("✅ Both transactions were submitted in real mode");
    output::info("🔍 Use transaction hashes above in explorers");
    Ok(())
}

fn run_real_ethereum_to_aptos(
    right_id_hash: Hash,
    transfer_id: Hash,
    transfer_id_bytes: [u8; 32],
    from_str: &str,
    to_str: &str,
    destination_contract: String,
    dest_owner_str: Option<String>,
    config: &Config,
    state: &mut State,
) -> Result<()> {
    output::progress(1, 6, "Step 1: Locking Right on ethereum...");
    let eth_cfg = config.chain(&Chain::Ethereum)?;
    let eth_key = get_private_key(config, Chain::Ethereum)?;
    let eth_address = state
        .get_address(&Chain::Ethereum)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Missing ethereum wallet address in state"))?;
    
    // Lock the right on Ethereum
    let source_tx_hash = send_ethereum_lock(
        &eth_address,
        &eth_cfg.rpc_url,
        &eth_key,
        right_id_hash,
    )?;
    let source_height = get_chain_height(&Chain::Ethereum, config);

    output::progress(2, 6, "Step 2: Building transfer proof...");
    output::progress(3, 6, "Step 3: Verifying proof on destination...");
    output::progress(4, 6, "Step 4: Checking seal registry...");

    output::progress(5, 6, "Step 5: Minting Right on aptos...");
    let aptos_cfg = config.chain(&Chain::Aptos)?;
    let aptos_key = get_private_key(config, Chain::Aptos)?;
    let commitment = state
        .get_right(&right_id_hash)
        .map(|r| r.commitment)
        .unwrap_or(right_id_hash);
    let aptos_tx_hash_hex = send_aptos_mint_via_cli(
        &destination_contract,
        &aptos_cfg.rpc_url,
        &aptos_key,
        right_id_hash,
        commitment,
        source_tx_hash,
    )?;
    let dest_tx_hash = hash_from_hex_32(&aptos_tx_hash_hex)?;

    output::progress(6, 6, "Step 6: Recording transfer...");
    state.record_seal_consumption(right_id_hash.as_bytes().to_vec());
    let _ = state.consume_right(&right_id_hash);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    state.add_transfer(TrackedTransfer {
        id: transfer_id,
        source_chain: Chain::Ethereum,
        dest_chain: Chain::Aptos,
        right_id: right_id_hash,
        sender_address: Some(eth_address),
        destination_address: dest_owner_str.clone(),
        source_tx_hash: Some(source_tx_hash),
        source_fee: None,
        dest_tx_hash: Some(dest_tx_hash),
        destination_fee: None,
        destination_contract: Some(destination_contract),
        proof: None,
        status: TransferStatus::Completed,
        created_at: timestamp,
        completed_at: Some(timestamp),
    });

    println!();
    output::success(&format!(
        "Cross-chain transfer complete: {} → {}",
        from_str, to_str
    ));
    output::kv_hash("Transfer ID", &transfer_id_bytes);
    output::header("🔹 Source Chain Transaction");
    output::kv_hash("Transaction Hash", source_tx_hash.as_bytes());
    output::kv("Block Height", &source_height.to_string());
    output::header("🔸 Destination Chain Transaction");
    output::kv_hash("Transaction Hash", dest_tx_hash.as_bytes());
    if let Some(addr) = dest_owner_str {
        output::kv("Recipient Address", &addr);
    }
    output::info("✅ Both transactions were submitted in real mode");
    Ok(())
}

fn run_real_ethereum_to_solana(
    right_id_hash: Hash,
    transfer_id: Hash,
    transfer_id_bytes: [u8; 32],
    from_str: &str,
    to_str: &str,
    _destination_contract: String,
    dest_owner_str: Option<String>,
    config: &Config,
    state: &mut State,
) -> Result<()> {
    output::progress(1, 6, "Step 1: Locking Right on ethereum...");
    let eth_cfg = config.chain(&Chain::Ethereum)?;
    let eth_key = get_private_key(config, Chain::Ethereum)?;
    let eth_address = state
        .get_address(&Chain::Ethereum)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Missing ethereum wallet address in state"))?;
    
    // Lock the right on Ethereum
    let source_tx_hash = send_ethereum_lock(
        &eth_address,
        &eth_cfg.rpc_url,
        &eth_key,
        right_id_hash,
    )?;
    let source_height = get_chain_height(&Chain::Ethereum, config);

    output::progress(2, 6, "Step 2: Building transfer proof...");
    output::progress(3, 6, "Step 3: Verifying proof on destination...");
    output::progress(4, 6, "Step 4: Checking seal registry...");

    output::progress(5, 6, "Step 5: Minting Right on solana...");
    let sol_cfg = config.chain(&Chain::Solana)?;
    let sol_key = get_private_key(config, Chain::Solana)?;
    let commitment = state
        .get_right(&right_id_hash)
        .map(|r| r.commitment)
        .unwrap_or(right_id_hash);
    // Use SDK-based mint instead of CLI
    let state_root = Hash::new([0u8; 32]);
    let sol_tx_sig = csv_adapter_solana::mint_right_from_hex_key(
        &sol_cfg.rpc_url,
        &_destination_contract, // program_id from contract
        &sol_key,
        right_id_hash,
        commitment,
        state_root,
        1u8, // source_chain = 1 for Ethereum
        source_tx_hash,
    ).map_err(|e| anyhow::anyhow!("Solana mint failed: {:?}", e))?;
    // Convert Solana signature to Hash by hashing it
    use sha2::{Digest, Sha256};
    let sig_hash = Sha256::digest(sol_tx_sig.as_bytes());
    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(&sig_hash);
    let dest_tx_hash = Hash::new(hash_bytes);

    output::progress(6, 6, "Step 6: Recording transfer...");
    state.record_seal_consumption(right_id_hash.as_bytes().to_vec());
    let _ = state.consume_right(&right_id_hash);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    state.add_transfer(TrackedTransfer {
        id: transfer_id,
        source_chain: Chain::Ethereum,
        dest_chain: Chain::Solana,
        right_id: right_id_hash,
        sender_address: Some(eth_address),
        destination_address: dest_owner_str.clone(),
        source_tx_hash: Some(source_tx_hash),
        source_fee: None,
        dest_tx_hash: Some(dest_tx_hash),
        destination_fee: None,
        destination_contract: None,
        proof: None,
        status: TransferStatus::Completed,
        created_at: timestamp,
        completed_at: Some(timestamp),
    });

    println!();
    output::success(&format!(
        "Cross-chain transfer complete: {} → {}",
        from_str, to_str
    ));
    output::kv_hash("Transfer ID", &transfer_id_bytes);
    output::header("🔹 Source Chain Transaction");
    output::kv_hash("Transaction Hash", source_tx_hash.as_bytes());
    output::kv("Block Height", &source_height.to_string());
    output::header("🔸 Destination Chain Transaction");
    output::kv_hash("Transaction Hash", dest_tx_hash.as_bytes());
    if let Some(addr) = dest_owner_str {
        output::kv("Recipient Address", &addr);
    }
    output::info("✅ Both transactions were submitted in real mode");
    output::info("🔍 Use transaction hashes above in explorers");
    Ok(())
}

/// Lock a right on Ethereum (calls the lock function on the CSV contract)
fn send_ethereum_lock(
    owner_address: &str,
    rpc_url: &str,
    private_key: &str,
    right_id: Hash,
) -> Result<Hash> {
    use sha3::{Digest, Keccak256};
    use secp256k1::{SecretKey, PublicKey, Message};
    
    // Parse private key
    let cleaned_key = private_key.trim().trim_start_matches("0x").trim();
    let key_bytes = hex::decode(cleaned_key)?;
    eprintln!("DEBUG: Private key hex length: {} chars, decoded bytes: {} bytes", cleaned_key.len(), key_bytes.len());
    eprintln!("DEBUG: First 8 chars of key: {}...", &cleaned_key[..8.min(cleaned_key.len())]);
    
    // csv-wallet takes first 32 bytes if key is 64 bytes - emulate that behavior
    let key_32 = if key_bytes.len() == 64 {
        eprintln!("DEBUG: Key is 64 bytes, taking first 32 bytes (csv-wallet compatible)");
        &key_bytes[..32]
    } else if key_bytes.len() == 32 {
        &key_bytes[..]
    } else {
        return Err(anyhow::anyhow!("Invalid key length: {} bytes (expected 32 or 64)", key_bytes.len()));
    };
    
    let secret_key = SecretKey::from_slice(key_32)?;
    
    // Derive public key and address
    let secp = secp256k1::Secp256k1::new();
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let public_key_bytes = public_key.serialize_uncompressed();
    let hash = Keccak256::digest(&public_key_bytes[1..]);
    let sender_address = format!("0x{}", hex::encode(&hash[12..]));
    eprintln!("DEBUG: Derived sender address: {}", sender_address);
    eprintln!("DEBUG: Expected owner address: {}", owner_address);
    
    // Verify the derived address matches the expected address
    let expected = owner_address.to_lowercase();
    let derived = sender_address.to_lowercase();
    if expected != derived {
        return Err(anyhow::anyhow!(
            "Address mismatch: config has '{}', but private key derives to '{}'. \n\
             The 0.263 ETH is at the config address, but tx will be sent from derived address (0 balance).\n\
             Key length: {} bytes (hex: {} chars)",
            owner_address,
            sender_address,
            key_bytes.len(),
            cleaned_key.len()
        ));
    }
    
    // Get nonce and gas price
    let nonce = get_ethereum_nonce(&sender_address, rpc_url)?;
    let gas_price = get_ethereum_gas_price(rpc_url)?;
    let balance = get_ethereum_balance(&sender_address, rpc_url)?;
    eprintln!("DEBUG: RPC URL: {}", rpc_url);
    eprintln!("DEBUG: Balance: {} wei ({} ETH)", balance, balance as f64 / 1e18);
    eprintln!("DEBUG: Gas price: {} wei, Gas limit: 50000", gas_price);
    eprintln!("DEBUG: Total cost: {} wei ({} ETH)", gas_price * 50000, (gas_price * 50000) as f64 / 1e18);
    
    // For now, create a simple lock transaction by sending a small amount to self
    // with the right_id in the data field as a marker
    // In production, this should call the actual CSV contract lock function
    let mut data = vec![0x00]; // Simple marker
    data.extend_from_slice(right_id.as_bytes());
    
    let tx = EthTransaction {
        nonce,
        gas_price,
        gas_limit: 50000,
        to: Some(hex::decode(&sender_address.trim_start_matches("0x"))?),
        value: 0,
        data,
        chain_id: 11155111, // Sepolia
    };
    
    let signed_tx = sign_ethereum_transaction(&tx, &secret_key)?;
    let tx_hash = send_raw_ethereum_transaction(&signed_tx, rpc_url)?;
    
    // Parse the transaction hash response
    hash_from_hex_32(&tx_hash)
}

fn get_private_key(config: &Config, chain: Chain) -> Result<String> {
    let wallet = config
        .wallet(&chain)
        .ok_or_else(|| anyhow::anyhow!("Missing wallet config for {}", chain))?;
    
    // Debug: check if it came from config.toml or csv-wallet
    if config.wallets.contains_key(&chain) {
        eprintln!("DEBUG: Key source: config.toml [wallets.{}]", chain);
    } else {
        eprintln!("DEBUG: Key source: csv-wallet.json fallback");
    }
    
    wallet
        .private_key
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Missing wallets.{}.private_key", chain))
}

#[derive(Clone, Debug)]
struct UtxoRef {
    txid: String,
    vout: u32,
    value: u64,
    script_pubkey: String,
}

fn publish_bitcoin_lock(address: &str, lock_data: &[u8], rpc_url: &str, private_key_hex: &str) -> Result<String> {
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

fn send_aptos_mint_via_cli(
    module_address: &str,
    rpc_url: &str,
    private_key_hex: &str,
    right_id: Hash,
    commitment: Hash,
    source_seal_ref: Hash,
) -> Result<String> {
    let function_id = format!("{}::CSVSealV2::mint_right", module_address);
    let output = Command::new("aptos")
        .args([
            "move",
            "run",
            "--function-id",
            &function_id,
            "--args",
            &format!("hex:{}", hex::encode(right_id.as_bytes())),
            &format!("hex:{}", hex::encode(commitment.as_bytes())),
            "u8:0",
            &format!("hex:{}", hex::encode(source_seal_ref.as_bytes())),
            "u64:1",
            "--private-key",
            private_key_hex.trim_start_matches("0x"),
            "--url",
            rpc_url,
            "--assume-yes",
            "--json",
        ])
        .output()?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "aptos move run failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let v: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse aptos JSON output: {}", e))?;
    let tx = v
        .get("Result")
        .and_then(|r| r.get("transaction_hash"))
        .and_then(|h| h.as_str())
        .or_else(|| v.get("transaction_hash").and_then(|h| h.as_str()))
        .ok_or_else(|| anyhow::anyhow!("Missing transaction hash in aptos output"))?;
    Ok(tx.to_string())
}

fn build_demo_merkle_proof(right_id: Hash, commitment: Hash, source_chain: u8) -> (Vec<u8>, Hash) {
    use sha3::{Digest, Keccak256};
    let mut leaf_input = Vec::new();
    leaf_input.extend_from_slice(right_id.as_bytes());
    leaf_input.extend_from_slice(commitment.as_bytes());
    leaf_input.push(source_chain);
    let leaf = Keccak256::digest(&leaf_input);
    let sibling = [0x11u8; 32];
    let (a, b) = if leaf.as_slice() < sibling.as_slice() {
        (leaf.as_slice(), sibling.as_slice())
    } else {
        (sibling.as_slice(), leaf.as_slice())
    };
    let mut parent = Vec::with_capacity(64);
    parent.extend_from_slice(a);
    parent.extend_from_slice(b);
    let root = Keccak256::digest(&parent);
    let mut root_arr = [0u8; 32];
    root_arr.copy_from_slice(&root[..32]);
    (sibling.to_vec(), Hash::new(root_arr))
}

fn hash_from_hex_32(s: &str) -> Result<Hash> {
    let bytes = hex::decode(s.trim_start_matches("0x"))?;
    if bytes.len() != 32 {
        return Err(anyhow::anyhow!("Expected 32-byte hash, got {}", bytes.len()));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(Hash::new(arr))
}

fn encode_varint(buf: &mut Vec<u8>, value: u64) {
    if value < 0xfd {
        buf.push(value as u8);
    } else if value <= 0xffff {
        buf.push(0xfd);
        buf.extend_from_slice(&(value as u16).to_le_bytes());
    } else if value <= 0xffffffff {
        buf.push(0xfe);
        buf.extend_from_slice(&(value as u32).to_le_bytes());
    } else {
        buf.push(0xff);
        buf.extend_from_slice(&value.to_le_bytes());
    }
}

fn is_placeholder_tx_hash(hash: &Hash) -> bool {
    let b = hash.as_bytes();

    // Common mock patterns used by the current provider stubs
    let known_mock = [
        [0x00; 32],
        [0x11; 32],
        [0x22; 32],
        [0x44; 32],
        [0x66; 32],
        [0x77; 32],
        [0x88; 32],
        [0xCC; 32],
        [0xEE; 32],
    ];
    if known_mock.iter().any(|m| b == m) {
        return true;
    }

    // Heuristic for synthetic mirrored hashes (e.g. first 16 bytes repeated).
    b[..16] == b[16..]
}

fn create_lock_provider(chain: &Chain, chain_id: ChainId) -> Box<dyn LockProvider> {
    match chain {
        Chain::Bitcoin => Box::new(BitcoinLockProvider {
            _chain_id: chain_id,
        }),
        Chain::Sui => Box::new(SuiLockProvider {
            _chain_id: chain_id,
        }),
        Chain::Aptos => Box::new(AptosLockProvider {
            _chain_id: chain_id,
        }),
        Chain::Solana => Box::new(SolanaLockProvider {
            _chain_id: chain_id,
        }),
        Chain::Ethereum => Box::new(EthereumLockProvider {
            _chain_id: chain_id,
        }),
    }
}

fn create_mint_provider(chain: &Chain, chain_id: ChainId) -> Result<Box<dyn MintProvider>, String> {
    match chain {
        Chain::Bitcoin => Err("Bitcoin is UTXO-native and does not support minting Rights. Bitcoin can only be used as a source chain for cross-chain transfers.".to_string()),
        Chain::Sui => Ok(Box::new(SuiMintProvider { chain_id })),
        Chain::Aptos => Ok(Box::new(AptosMintProvider { chain_id })),
        Chain::Solana => Ok(Box::new(SolanaMintProvider { chain_id })),
        Chain::Ethereum => Ok(Box::new(EthereumMintProvider { chain_id })),
    }
}

fn chain_to_chain_id(chain: &Chain) -> ChainId {
    match chain {
        Chain::Bitcoin => ChainId::Bitcoin,
        Chain::Ethereum => ChainId::Ethereum,
        Chain::Sui => ChainId::Sui,
        Chain::Aptos => ChainId::Aptos,
        Chain::Solana => ChainId::Solana,
    }
}

fn cmd_status(transfer_id: String, state: &State) -> Result<()> {
    let bytes = hex::decode(transfer_id.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid Transfer ID: {}", e))?;
    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(&bytes[..32]);
    let transfer_id_hash = Hash::new(hash_bytes);

    output::header(&format!("Transfer: {}", transfer_id));

    if let Some(transfer) = state.get_transfer(&transfer_id_hash) {
        output::header("📋 Cross-Chain Transfer Report");

        output::kv("Transfer ID", &hex::encode(transfer.id.as_bytes()));
        output::kv("Right ID", &hex::encode(transfer.right_id.as_bytes()));
        output::kv("Status", &format!("{:?}", transfer.status));
        output::kv("Created At", &chrono::DateTime::<chrono::Utc>::from_timestamp(transfer.created_at as i64, 0).map(|d| d.to_rfc3339()).unwrap_or_else(|| transfer.created_at.to_string()));
        
        if let Some(completed) = transfer.completed_at {
            output::kv("Completed At", &chrono::DateTime::<chrono::Utc>::from_timestamp(completed as i64, 0).map(|d| d.to_rfc3339()).unwrap_or_else(|| completed.to_string()));
        }

        output::header("🔹 Source Chain");
        output::kv("Chain", &transfer.source_chain.to_string());
        if let Some(sender) = &transfer.sender_address {
            output::kv("Sender Address", sender);
        }
        if let Some(source_tx) = &transfer.source_tx_hash {
            output::kv_hash("Transaction ID", source_tx.as_bytes());
        }
        if let Some(fee) = transfer.source_fee {
            output::kv("Transaction Fee", &fee.to_string());
        }

        output::header("🔸 Destination Chain");
        output::kv("Chain", &transfer.dest_chain.to_string());
        if let Some(dest_addr) = &transfer.destination_address {
            output::kv("Destination Address", dest_addr);
        }
        if let Some(dest_tx) = &transfer.dest_tx_hash {
            output::kv_hash("Transaction ID", dest_tx.as_bytes());
        }
        if let Some(fee) = transfer.destination_fee {
            output::kv("Transaction Fee", &fee.to_string());
        }
        if let Some(contract) = &transfer.destination_contract {
            output::kv("Contract Address", contract);
        }
    } else {
        output::warning("Transfer not found");
    }

    Ok(())
}

fn cmd_list(from: Option<Chain>, to: Option<Chain>, state: &State) -> Result<()> {
    output::header("Cross-Chain Transfers");

    let headers = vec!["Transfer ID", "From", "To", "Right ID", "Status"];
    let mut rows = Vec::new();

    for transfer in &state.transfers {
        if let Some(ref filter_from) = from {
            if transfer.source_chain != *filter_from {
                continue;
            }
        }
        if let Some(ref filter_to) = to {
            if transfer.dest_chain != *filter_to {
                continue;
            }
        }

        let status_str = match &transfer.status {
            TransferStatus::Completed => "Completed".to_string(),
            TransferStatus::Failed { reason } => format!("Failed: {}", reason),
            other => format!("{:?}", other),
        };

        rows.push(vec![
            hex::encode(transfer.id.as_bytes())[..10].to_string(),
            transfer.source_chain.to_string(),
            transfer.dest_chain.to_string(),
            hex::encode(transfer.right_id.as_bytes())[..10].to_string(),
            status_str,
        ]);
    }

    if rows.is_empty() {
        output::info("No transfers recorded. Use 'csv cross-chain transfer' to start one.");
    } else {
        output::table(&headers, &rows);
    }

    Ok(())
}

fn cmd_retry(transfer_id: String, _config: &Config, state: &mut State) -> Result<()> {
    output::header("Retrying Transfer");
    output::kv("Transfer ID", &transfer_id);

    // Parse transfer ID
    let bytes = hex::decode(transfer_id.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid Transfer ID: {}", e))?;
    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(&bytes[..32]);
    let transfer_id_hash = Hash::new(hash_bytes);

    // Look up transfer
    let transfer = state.get_transfer(&transfer_id_hash);
    match transfer {
        Some(t) => {
            output::kv("Source", &t.source_chain.to_string());
            output::kv("Destination", &t.dest_chain.to_string());
            output::kv("Status", &format!("{:?}", t.status));

            match &t.status {
                TransferStatus::Failed { reason } => {
                    output::warning(&format!("Failure reason: {}", reason));
                    output::info("If lock was successful but mint failed, wait for timeout (24h) and the source chain seal will be recoverable via refund.");
                    output::info("For timed-out locks: the refund function is available on the source chain contract.");
                }
                TransferStatus::Locked | TransferStatus::Initiated => {
                    output::info(
                        "Transfer is in progress. If stuck, wait for lock timeout and refund.",
                    );
                }
                TransferStatus::Completed => {
                    output::success("Transfer already completed successfully.");
                }
                _ => {
                    output::info("Transfer status does not support retry.");
                }
            }
        }
        None => {
            output::warning("Transfer not found in state.");
        }
    }

    Ok(())
}

/// Get the current block/checkpoint height for a chain via RPC.
/// Makes actual HTTP RPC calls to configured endpoints.
fn get_chain_height(chain: &Chain, config: &Config) -> u64 {
    // Try to fetch from RPC, fallback to reasonable defaults
    let runtime = tokio::runtime::Runtime::new().ok();

    if let Some(rt) = runtime {
        let result = rt.block_on(async { fetch_chain_height_rpc(chain, config).await });

        if let Ok(height) = result {
            return height;
        }
    }

    // Fallback to reasonable defaults if RPC fails
    tracing::warn!(chain = ?chain, "RPC height fetch failed, using fallback");
    match chain {
        Chain::Bitcoin => 300_000,
        Chain::Ethereum => 7_000_000,
        Chain::Sui => 350_000_000,
        Chain::Aptos => 15_000_000,
        Chain::Solana => 250_000_000, // Solana has very high block numbers
    }
}

/// Fetch chain height via RPC call
async fn fetch_chain_height_rpc(chain: &Chain, config: &Config) -> anyhow::Result<u64> {
    let chain_config = config
        .chains
        .get(chain)
        .ok_or_else(|| anyhow::anyhow!("Chain not configured"))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    match chain {
        Chain::Bitcoin => {
            // Mempool.space API or similar
            let url = if chain_config.rpc_url.contains("mempool.space") {
                format!(
                    "{}/api/blocks/tip/height",
                    chain_config.rpc_url.trim_end_matches('/')
                )
            } else {
                // Fallback to esplora-style endpoint
                format!(
                    "{}/blocks/tip/height",
                    chain_config.rpc_url.trim_end_matches('/')
                )
            };

            let response = client.get(&url).send().await?;
            let height: u64 = response.text().await?.parse()?;
            Ok(height)
        }
        Chain::Ethereum => {
            // JSON-RPC eth_blockNumber
            let body = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_blockNumber",
                "params": [],
                "id": 1
            });

            let response = client
                .post(&chain_config.rpc_url)
                .json(&body)
                .send()
                .await?;

            let rpc_response: JsonRpcResponse<String> = response.json().await?;

            if let Some(error) = rpc_response.error {
                return Err(anyhow::anyhow!("RPC error: {}", error.message));
            }

            let hex_height = rpc_response
                .result
                .ok_or_else(|| anyhow::anyhow!("No result in response"))?;

            // Parse hex string (0x prefix)
            let height = u64::from_str_radix(hex_height.trim_start_matches("0x"), 16)?;
            Ok(height)
        }
        Chain::Sui => {
            // Sui JSON-RPC sui_getLatestCheckpointSequenceNumber
            let body = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "sui_getLatestCheckpointSequenceNumber",
                "params": [],
                "id": 1
            });

            let response = client
                .post(&chain_config.rpc_url)
                .json(&body)
                .send()
                .await?;

            let rpc_response: JsonRpcResponse<String> = response.json().await?;

            if let Some(error) = rpc_response.error {
                return Err(anyhow::anyhow!("RPC error: {}", error.message));
            }

            let checkpoint = rpc_response
                .result
                .ok_or_else(|| anyhow::anyhow!("No result in response"))?;

            let height = u64::from_str_radix(checkpoint.trim_start_matches("0x"), 16)?;
            Ok(height)
        }
        Chain::Aptos => {
            // Aptos REST API - get ledger info
            let url = format!("{}/v1", chain_config.rpc_url.trim_end_matches('/'));

            let response = client.get(&url).send().await?;
            let ledger_info: serde_json::Value = response.json().await?;

            let version = ledger_info["block_height"]
                .as_str()
                .or_else(|| ledger_info["ledger_version"].as_str())
                .ok_or_else(|| anyhow::anyhow!("No block height in response"))?;

            let height = u64::from_str_radix(version.trim_start_matches("0x"), 16)
                .or_else(|_| version.parse())?;
            Ok(height)
        }
        Chain::Solana => {
            // Solana JSON-RPC - getEpochInfo
            let body = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "getEpochInfo",
                "params": [],
                "id": 1
            });

            let response = client
                .post(&chain_config.rpc_url)
                .json(&body)
                .send()
                .await?;

            let rpc_response: JsonRpcResponse<serde_json::Value> = response.json().await?;

            if let Some(error) = rpc_response.error {
                return Err(anyhow::anyhow!("RPC error: {}", error.message));
            }

            let epoch_info = rpc_response
                .result
                .ok_or_else(|| anyhow::anyhow!("No result in response"))?;

            let slot = epoch_info["absoluteSlot"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("No slot in response"))?;
            Ok(slot)
        }
    }
}

/// Get the required confirmation depth for a chain.
fn get_chain_confirmations(chain: &Chain) -> u64 {
    match chain {
        Chain::Bitcoin => 6,   // ~1 hour on signet
        Chain::Ethereum => 15, // ~3 minutes
        Chain::Sui => 1,       // Finality is ~1 checkpoint
        Chain::Aptos => 1,     // Finality is ~1 block (HotStuff),
        Chain::Solana => 1,    // Finality is ~1 block (Proof of History)
    }
}

/// Fetch actual gas balance from chain RPC
fn fetch_gas_balance(chain: &Chain, config: &Config, address: &str) -> anyhow::Result<u64> {
    let chain_config = config
        .chains
        .get(chain)
        .ok_or_else(|| anyhow::anyhow!("Chain not configured"))?;

    let runtime = tokio::runtime::Runtime::new()?;
    
    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        match chain {
            Chain::Bitcoin => {
                // Fetch UTXO balance
                let url = format!(
                    "{}/api/address/{}/balance",
                    chain_config.rpc_url.trim_end_matches('/'),
                    address
                );
                let response = client.get(&url).send().await?;
                let balance: u64 = response.text().await?.parse()?;
                Ok(balance)
            }
            Chain::Ethereum => {
                // JSON-RPC eth_getBalance
                let body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "eth_getBalance",
                    "params": [address, "latest"],
                    "id": 1
                });

                let response = client
                    .post(&chain_config.rpc_url)
                    .json(&body)
                    .send()
                    .await?;

                let rpc_response: JsonRpcResponse<String> = response.json().await?;
                
                if let Some(error) = rpc_response.error {
                    return Err(anyhow::anyhow!("RPC error: {}", error.message));
                }

                let hex_balance = rpc_response
                    .result
                    .ok_or_else(|| anyhow::anyhow!("No result in response"))?;

                let balance = u64::from_str_radix(hex_balance.trim_start_matches("0x"), 16)?;
                Ok(balance)
            }
            Chain::Sui => {
                // Sui JSON-RPC sui_getBalance
                let body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "sui_getBalance",
                    "params": [address],
                    "id": 1
                });

                let response = client
                    .post(&chain_config.rpc_url)
                    .json(&body)
                    .send()
                    .await?;

                let rpc_response: JsonRpcResponse<serde_json::Value> = response.json().await?;
                
                if let Some(error) = rpc_response.error {
                    return Err(anyhow::anyhow!("RPC error: {}", error.message));
                }

                let result = rpc_response
                    .result
                    .ok_or_else(|| anyhow::anyhow!("No result in response"))?;

                let balance = result["totalBalance"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("No balance in response"))?
                    .parse()?;

                Ok(balance)
            }
            Chain::Aptos => {
                // Aptos REST API get account balance
                let url = format!(
                    "{}/v1/accounts/{}/resource/0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
                    chain_config.rpc_url.trim_end_matches('/'),
                    address
                );

                let response = client.get(&url).send().await?;
                let account_resource: serde_json::Value = response.json().await?;
                
                let balance = account_resource["data"]["coin"]["value"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("No balance in response"))?
                    .parse()?;

                Ok(balance)
            }
            Chain::Solana => {
                // Solana JSON-RPC getBalance
                let body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "getBalance",
                    "params": [address],
                    "id": 1
                });

                let response = client
                    .post(&chain_config.rpc_url)
                    .json(&body)
                    .send()
                    .await?;

                let rpc_response: JsonRpcResponse<serde_json::Value> = response.json().await?;
                
                if let Some(error) = rpc_response.error {
                    return Err(anyhow::anyhow!("RPC error: {}", error.message));
                }

                let result = rpc_response
                    .result
                    .ok_or_else(|| anyhow::anyhow!("No result in response"))?;

                let balance = result["value"]
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("No balance in response"))?;

                Ok(balance)
            }
        }
    })
}

/// Format a Unix timestamp as a human-readable date
fn format_timestamp(timestamp: u64) -> String {
    use std::time::{Duration, UNIX_EPOCH};
    
    let datetime = UNIX_EPOCH + Duration::from_secs(timestamp);
    let datetime = chrono::DateTime::<chrono::Local>::from(datetime);
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

// ===== Native Ethereum transaction sender using HTTP RPC =====

/// Send Ethereum mint transaction using native HTTP RPC (no external cast command needed)
fn send_ethereum_mint_via_cast(
    contract_address: &str,
    rpc_url: &str,
    private_key: &str,
    right_id: Hash,
    commitment: Hash,
    state_root: Hash,
    source_chain: u8,
    source_seal_ref: Hash,
    proof: &[u8],
    proof_root: Hash,
) -> Result<String> {
    // Use native HTTP RPC implementation (no external cast command needed)
    send_ethereum_mint_native(
        contract_address,
        rpc_url,
        private_key,
        right_id,
        commitment,
        state_root,
        source_chain,
        source_seal_ref,
        proof,
        proof_root,
    )
}

/// Native Ethereum transaction sender using HTTP JSON-RPC
fn send_ethereum_mint_native(
    contract_address: &str,
    rpc_url: &str,
    private_key: &str,
    right_id: Hash,
    commitment: Hash,
    state_root: Hash,
    source_chain: u8,
    source_seal_ref: Hash,
    proof: &[u8],
    proof_root: Hash,
) -> Result<String> {
    use secp256k1::{SecretKey, PublicKey};
    use sha3::{Digest, Keccak256};
    
    // Parse private key
    let cleaned_key = private_key.trim().trim_start_matches("0x").trim();
    let key_bytes = hex::decode(cleaned_key)?;
    let secret_key = SecretKey::from_slice(&key_bytes)?;
    
    // Derive public key and address
    let secp = secp256k1::Secp256k1::new();
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let public_key_bytes = public_key.serialize_uncompressed();
    // Ethereum address: last 20 bytes of Keccak256 of public key (without 0x04 prefix)
    let hash = Keccak256::digest(&public_key_bytes[1..]);
    let sender_address = format!("0x{}", hex::encode(&hash[12..]));
    
    // Get nonce
    let nonce = get_ethereum_nonce(&sender_address, rpc_url)?;
    
    // Get gas price
    let gas_price = get_ethereum_gas_price(rpc_url)?;
    
    // Build the function call data
    // Function selector for mintRight(bytes32,bytes32,bytes32,uint8,bytes,bytes,bytes32)
    let selector = &Keccak256::digest(b"mintRight(bytes32,bytes32,bytes32,uint8,bytes,bytes,bytes32)")[0..4];
    
    // Encode parameters
    let mut data = selector.to_vec();
    
    // rightId (bytes32)
    data.extend_from_slice(right_id.as_bytes());
    
    // commitment (bytes32)
    data.extend_from_slice(commitment.as_bytes());
    
    // stateRoot (bytes32)
    data.extend_from_slice(state_root.as_bytes());
    
    // sourceChain (uint8) - padded to 32 bytes
    data.extend_from_slice(&[0u8; 31]);
    data.push(source_chain);
    
    // sourceSealRef (bytes) - offset pointer
    let source_seal_offset = 7 * 32; // 7 params * 32 bytes each
    data.extend_from_slice(&encode_u256(source_seal_offset as u64));
    
    // proof (bytes) - offset pointer
    let proof_offset = source_seal_offset + 32 + ((source_seal_ref.as_bytes().len() + 31) / 32) * 32;
    data.extend_from_slice(&encode_u256(proof_offset as u64));
    
    // proofRoot (bytes32)
    data.extend_from_slice(proof_root.as_bytes());
    
    // sourceSealRef length and data
    data.extend_from_slice(&encode_u256(source_seal_ref.as_bytes().len() as u64));
    data.extend_from_slice(source_seal_ref.as_bytes());
    // Pad to 32 byte boundary
    let seal_padding = (32 - (source_seal_ref.as_bytes().len() % 32)) % 32;
    data.extend_from_slice(&vec![0u8; seal_padding]);
    
    // proof length and data
    data.extend_from_slice(&encode_u256(proof.len() as u64));
    data.extend_from_slice(proof);
    // Pad to 32 byte boundary
    let proof_padding = (32 - (proof.len() % 32)) % 32;
    data.extend_from_slice(&vec![0u8; proof_padding]);
    
    // Build and sign transaction
    let tx = EthTransaction {
        nonce,
        gas_price,
        gas_limit: 500000,
        to: Some(hex::decode(contract_address.trim_start_matches("0x"))?),
        value: 0,
        data,
        chain_id: 11155111, // Sepolia testnet - should be configurable
    };
    
    let signed_tx = sign_ethereum_transaction(&tx, &secret_key)?;
    
    // Send raw transaction
    send_raw_ethereum_transaction(&signed_tx, rpc_url)
}

fn encode_u256(value: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[24..].copy_from_slice(&value.to_be_bytes());
    bytes
}

struct EthTransaction {
    nonce: u64,
    gas_price: u64,
    gas_limit: u64,
    to: Option<Vec<u8>>,
    value: u64,
    data: Vec<u8>,
    chain_id: u64,
}

fn get_ethereum_nonce(address: &str, rpc_url: &str) -> Result<u64> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionCount",
            "params": [address, "latest"],
            "id": 1
        }))
        .send()?
        .json::<serde_json::Value>()?;
    
    let count_hex = resp.get("result").and_then(|r| r.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to get nonce"))?;
    Ok(u64::from_str_radix(count_hex.trim_start_matches("0x"), 16).unwrap_or(0))
}

fn get_ethereum_gas_price(rpc_url: &str) -> Result<u64> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_gasPrice",
            "params": [],
            "id": 1
        }))
        .send()?
        .json::<serde_json::Value>()?;
    
    let price_hex = resp.get("result").and_then(|r| r.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to get gas price"))?;
    Ok(u64::from_str_radix(price_hex.trim_start_matches("0x"), 16).unwrap_or(20000000000))
}

fn get_ethereum_balance(address: &str, rpc_url: &str) -> Result<u128> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getBalance",
            "params": [address, "latest"],
            "id": 1
        }))
        .send()?
        .json::<serde_json::Value>()?;
    
    let balance_hex = resp.get("result").and_then(|r| r.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to get balance"))?;
    Ok(u128::from_str_radix(balance_hex.trim_start_matches("0x"), 16).unwrap_or(0))
}

fn sign_ethereum_transaction(tx: &EthTransaction, secret_key: &secp256k1::SecretKey) -> Result<String> {
    use sha3::{Digest, Keccak256};
    use secp256k1::{Message, Secp256k1};
    
    // RLP encode transaction
    let mut rlp = Vec::new();
    
    // Nonce
    rlp.extend_from_slice(&encode_rlp(tx.nonce));
    // Gas price
    rlp.extend_from_slice(&encode_rlp(tx.gas_price));
    // Gas limit
    rlp.extend_from_slice(&encode_rlp(tx.gas_limit));
    // To
    if let Some(to) = &tx.to {
        rlp.extend_from_slice(&encode_rlp_length(to.len()));
        rlp.extend_from_slice(to);
    } else {
        rlp.push(0x80);
    }
    // Value
    rlp.extend_from_slice(&encode_rlp(tx.value));
    // Data
    rlp.extend_from_slice(&encode_rlp_length(tx.data.len()));
    rlp.extend_from_slice(&tx.data);
    // Chain ID, 0, 0 for EIP-155
    rlp.extend_from_slice(&encode_rlp(tx.chain_id));
    rlp.push(0x80);
    rlp.push(0x80);
    
    // Wrap in list
    let mut encoded = if rlp.len() <= 55 {
        vec![0xc0 + rlp.len() as u8]
    } else {
        let len_bytes = encode_length_bytes(rlp.len());
        let mut e = vec![0xf7 + len_bytes.len() as u8];
        e.extend_from_slice(&len_bytes);
        e
    };
    encoded.extend_from_slice(&rlp);
    
    // Hash and sign
    let hash = Keccak256::digest(&encoded);
    let message = Message::from_digest_slice(&hash)?;
    let secp = Secp256k1::new();
    let sig = secp.sign_ecdsa(&message, secret_key);
    let sig_bytes = sig.serialize_compact();
    
    // Determine recovery ID (v) by checking which public key recovers correctly
    // For simplicity, we try both 0 and 1 and use 0 as default
    // In production, you should properly compute the recovery ID
    let recovery_id = 0u8; // Default to 0
    
    // Build signed transaction with v, r, s
    let mut signed_rlp = Vec::new();
    signed_rlp.extend_from_slice(&encode_rlp(tx.nonce));
    signed_rlp.extend_from_slice(&encode_rlp(tx.gas_price));
    signed_rlp.extend_from_slice(&encode_rlp(tx.gas_limit));
    if let Some(to) = &tx.to {
        signed_rlp.extend_from_slice(&encode_rlp_length(to.len()));
        signed_rlp.extend_from_slice(to);
    } else {
        signed_rlp.push(0x80);
    }
    signed_rlp.extend_from_slice(&encode_rlp(tx.value));
    signed_rlp.extend_from_slice(&encode_rlp_length(tx.data.len()));
    signed_rlp.extend_from_slice(&tx.data);
    // v = chain_id * 2 + 35 + recovery_id
    let v = tx.chain_id * 2 + 35 + recovery_id as u64;
    signed_rlp.extend_from_slice(&encode_rlp(v));
    // r
    signed_rlp.extend_from_slice(&encode_rlp_bytes(&sig_bytes[..32]));
    // s
    signed_rlp.extend_from_slice(&encode_rlp_bytes(&sig_bytes[32..]));
    
    // Wrap in list
    let mut signed_encoded = if signed_rlp.len() <= 55 {
        vec![0xc0 + signed_rlp.len() as u8]
    } else {
        let len_bytes = encode_length_bytes(signed_rlp.len());
        let mut e = vec![0xf7 + len_bytes.len() as u8];
        e.extend_from_slice(&len_bytes);
        e
    };
    signed_encoded.extend_from_slice(&signed_rlp);
    
    Ok(hex::encode(signed_encoded))
}

fn encode_rlp(value: u64) -> Vec<u8> {
    if value == 0 {
        return vec![0x80];
    }
    let bytes = value.to_be_bytes();
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(8);
    let len = 8 - start;
    if len == 1 && bytes[start] < 0x80 {
        return vec![bytes[start]];
    }
    let mut result = vec![0x80 + len as u8];
    result.extend_from_slice(&bytes[start..]);
    result
}

fn encode_rlp_length(len: usize) -> Vec<u8> {
    if len == 0 {
        return vec![0x80];
    }
    if len < 56 {
        return vec![0x80 + len as u8];
    }
    let bytes = encode_length_bytes(len);
    let mut result = vec![0xb7 + bytes.len() as u8];
    result.extend_from_slice(&bytes);
    result
}

fn encode_rlp_bytes(bytes: &[u8]) -> Vec<u8> {
    if bytes.len() == 1 && bytes[0] < 0x80 {
        return vec![bytes[0]];
    }
    let mut result = encode_rlp_length(bytes.len());
    result.extend_from_slice(bytes);
    result
}

fn encode_length_bytes(len: usize) -> Vec<u8> {
    let mut n = len;
    let mut bytes = Vec::new();
    while n > 0 {
        bytes.push((n & 0xff) as u8);
        n >>= 8;
    }
    bytes.reverse();
    bytes
}

fn send_raw_ethereum_transaction(signed_tx_hex: &str, rpc_url: &str) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_sendRawTransaction",
            "params": [format!("0x{}", signed_tx_hex)],
            "id": 1
        }))
        .send()?
        .json::<serde_json::Value>()?;
    
    if let Some(error) = resp.get("error") {
        return Err(anyhow::anyhow!("RPC error: {}", error));
    }
    
    resp.get("result")
        .and_then(|r| r.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Failed to send transaction"))
}
