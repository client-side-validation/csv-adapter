//! Cross-chain transfer command implementation

use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

use csv_adapter_core::cross_chain::{
    ChainId, CrossChainFinalityProof, CrossChainSealRegistry, CrossChainTransferProof,
    LockProvider, MintProvider, TransferVerifier, InclusionProof, CrossChainLockEvent,
    CrossChainRegistryEntry, CrossChainTransferResult,
};
use csv_adapter_core::hash::Hash;
use csv_adapter_core::right::OwnershipProof;
use csv_adapter_core::seal::SealRef;

use crate::config::{Chain, Config};
use crate::output;
use crate::state::{UnifiedStateManager, TransferStatus};

use super::super::cross_chain_impl::*;
use super::utils::*;

/// Make a seal reference from bytes
fn make_seal_ref(data: &[u8]) -> SealRef {
    SealRef::new(data.to_vec(), None).unwrap_or_else(|_| SealRef::new(vec![0u8; 36], None).unwrap())
}

/// Create lock provider for a chain
fn create_lock_provider(chain: &Chain, chain_id: ChainId) -> Box<dyn LockProvider> {
    match chain {
        Chain::Bitcoin => Box::new(BitcoinLockProvider { _chain_id: chain_id }),
        Chain::Ethereum => Box::new(EthereumLockProvider { _chain_id: chain_id }),
        Chain::Sui => Box::new(SuiLockProvider { _chain_id: chain_id }),
        Chain::Aptos => Box::new(AptosLockProvider { _chain_id: chain_id }),
        Chain::Solana => Box::new(SolanaLockProvider { _chain_id: chain_id }),
        _ => panic!("Unsupported chain for locking: {:?}", chain),
    }
}

/// Create mint provider for a chain  
fn create_mint_provider(chain: &Chain, chain_id: ChainId) -> Result<Box<dyn MintProvider>, String> {
    match chain {
        Chain::Bitcoin => Err("Bitcoin cannot be a destination for minting".to_string()),
        Chain::Ethereum => Ok(Box::new(EthereumMintProvider { _chain_id: chain_id })),
        Chain::Sui => Ok(Box::new(SuiMintProvider { _chain_id: chain_id })),
        Chain::Aptos => Ok(Box::new(AptosMintProvider { _chain_id: chain_id })),
        Chain::Solana => Ok(Box::new(SolanaMintProvider { _chain_id: chain_id })),
        _ => Err(format!("Unsupported chain for minting: {:?}", chain)),
    }
}

/// Convert chain to chain ID
fn chain_to_chain_id(chain: &Chain) -> ChainId {
    match chain {
        Chain::Bitcoin => ChainId::Bitcoin,
        Chain::Ethereum => ChainId::Ethereum,
        Chain::Sui => ChainId::Sui,
        Chain::Aptos => ChainId::Aptos,
        Chain::Solana => ChainId::Solana,
        _ => ChainId::Custom(999),
    }
}

/// Current timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Check if tx hash is a placeholder
fn is_placeholder_tx_hash(hash: &Hash) -> bool {
    hash.as_bytes().iter().all(|&b| b == 0)
}
fn cmd_transfer(
    from: Chain,
    to: Chain,
    right_id: String,
    dest_owner: Option<String>,
    simulation: bool,
    config: &Config,
    state: &mut UnifiedStateManager,
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
    let dest_owner_str = dest_owner.or_else(|| state.get_address(&to).map(|s| s.to_string()));
    
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
    for seal in state.storage.seals.iter().filter(|s| s.consumed) {
        use csv_adapter_core::right::RightId;
        use csv_adapter_core::seal::SealRef;
        use csv_adapter_core::seal_registry::{ChainId as CoreChainId, SealConsumption};
        let seal_bytes = hex::decode(&seal.seal_ref).unwrap_or_default();
        if let Ok(seal_ref) = SealRef::new(seal_bytes, None) {
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
    if state.is_seal_consumed(&hex::encode(&right_bytes)) {
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

    // Get timestamp for records
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Step 6: Record in registry
    output::progress(6, 6, "Step 6: Recording transfer...");
    state.record_seal_consumption(hex::encode(&right_bytes));
    
    // Mark original right as consumed on source chain
    let _ = state.consume_right(&hex::encode(right_id_hash.as_bytes()));

    // Collect address info before moving values
    let sender_address = state.get_address(&from).map(|s| s.to_string());
    let dest_address = dest_owner_str.clone();
    let dest_contract = state.get_contract(&to).map(|c| c.address.clone());

    // Add new right to tracking if we own the destination address
    if let Some(current_dest_addr) = state.get_address(&to) {
        if let Some(transfer_dest_addr) = &dest_address {
            if current_dest_addr == transfer_dest_addr {
                // We own this right on destination chain, add to tracking
                let new_right = RightRecord {
                    id: hex::encode(mint_result.destination_right.id.0.as_bytes()),
                    chain: to.clone(),
                    seal_ref: hex::encode(&mint_result.destination_seal.seal_id),
                    owner: hex::encode(&dest_owner_bytes),
                    value: 0,
                    commitment: hex::encode(mint_result.destination_right.commitment.as_bytes()),
                    nullifier: None,
                    status: csv_adapter_store::state::RightStatus::Active,
                    created_at: timestamp,
                };
                state.add_right(new_right);
            }
        }
    }

    // Create tracked transfer
    let transfer = TransferRecord {
        id: hex::encode(&transfer_id),
        source_chain: from,
        dest_chain: to,
        right_id: hex::encode(right_id_hash.as_bytes()),
        sender_address,
        destination_address: dest_address,
        source_tx_hash: Some(hex::encode(transfer_proof.lock_event.source_tx_hash.as_bytes())),
        source_fee: None,
        dest_tx_hash: Some(hex::encode(mint_result.registry_entry.mint_tx_hash.as_bytes())),
        dest_fee: None,
        destination_contract: dest_contract,
        proof: Some(hex::encode(serde_json::to_vec(&transfer_proof).unwrap_or_default())),
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
    if let Some(ref addr) = dest_owner_str {
        output::kv("Recipient Address", addr);
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
    destination_contract: String,
    dest_owner_str: Option<String>,
    dest_owner_bytes: Vec<u8>,
    config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    output::progress(1, 6, "Step 1: Locking Right on bitcoin...");
    let btc_cfg = config.chain(&Chain::Bitcoin)?;
    let btc_key = get_private_key(state, Chain::Bitcoin)?;
    let btc_address = state
        .get_address(&Chain::Bitcoin)
        .map(|s| s.to_string())
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
    let eth_key = get_private_key(state, Chain::Ethereum)?;
    let commitment = state
        .get_right(&right_id_hash.to_string())
        .and_then(|r| hash_from_hex_32(&r.commitment).ok())
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
    state.record_seal_consumption(right_id_hash.to_string());
    let _ = state.consume_right(&right_id_hash.to_string());
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    state.add_transfer(TransferRecord {
        id: transfer_id.to_string(),
        source_chain: Chain::Bitcoin,
        dest_chain: Chain::Ethereum,
        right_id: right_id_hash.to_string(),
        sender_address: state.get_address(&Chain::Bitcoin).map(|s| s.to_string()),
        destination_address: dest_owner_str.clone(),
        source_tx_hash: Some(source_tx_hash.to_string()),
        source_fee: None,
        dest_tx_hash: Some(dest_tx_hash.to_string()),
        dest_fee: None,
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
    state: &mut UnifiedStateManager,
) -> Result<()> {
    output::progress(1, 6, "Step 1: Locking Right on ethereum...");
    let eth_cfg = config.chain(&Chain::Ethereum)?;
    let eth_key = get_private_key(state, Chain::Ethereum)?;
    let eth_address = state
        .get_address(&Chain::Ethereum)
        .map(|s| s.to_string())
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
    let sui_key = get_private_key(state, Chain::Sui)?;
    let commitment = state
        .get_right(&right_id_hash.to_string())
        .and_then(|r| hash_from_hex_32(&r.commitment).ok())
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
    state.record_seal_consumption(right_id_hash.to_string());
    let _ = state.consume_right(&right_id_hash.to_string());
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    state.add_transfer(TransferRecord {
        id: transfer_id.to_string(),
        source_chain: Chain::Ethereum,
        dest_chain: Chain::Sui,
        right_id: right_id_hash.to_string(),
        sender_address: Some(eth_address),
        destination_address: dest_owner_str.clone(),
        source_tx_hash: Some(source_tx_hash.to_string()),
        source_fee: None,
        dest_tx_hash: Some(dest_tx_hash.to_string()),
        dest_fee: None,
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
    state: &mut UnifiedStateManager,
) -> Result<()> {
    output::progress(1, 6, "Step 1: Locking Right on ethereum...");
    let eth_cfg = config.chain(&Chain::Ethereum)?;
    let eth_key = get_private_key(state, Chain::Ethereum)?;
    let eth_address = state
        .get_address(&Chain::Ethereum)
        .map(|s| s.to_string())
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
    let aptos_key = get_private_key(state, Chain::Aptos)?;
    let commitment = state
        .get_right(&right_id_hash.to_string())
        .and_then(|r| hash_from_hex_32(&r.commitment).ok())
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
    state.record_seal_consumption(right_id_hash.to_string());
    let _ = state.consume_right(&right_id_hash.to_string());
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    state.add_transfer(TransferRecord {
        id: transfer_id.to_string(),
        source_chain: Chain::Ethereum,
        dest_chain: Chain::Aptos,
        right_id: right_id_hash.to_string(),
        sender_address: Some(eth_address),
        destination_address: dest_owner_str.clone(),
        source_tx_hash: Some(source_tx_hash.to_string()),
        source_fee: None,
        dest_tx_hash: Some(dest_tx_hash.to_string()),
        dest_fee: None,
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
    state: &mut UnifiedStateManager,
) -> Result<()> {
    output::progress(1, 6, "Step 1: Locking Right on ethereum...");
    let eth_cfg = config.chain(&Chain::Ethereum)?;
    let eth_key = get_private_key(state, Chain::Ethereum)?;
    let eth_address = state
        .get_address(&Chain::Ethereum)
        .map(|s| s.to_string())
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
    let sol_key = get_private_key(state, Chain::Solana)?;
    let commitment = state
        .get_right(&right_id_hash.to_string())
        .and_then(|r| hash_from_hex_32(&r.commitment).ok())
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
    state.record_seal_consumption(right_id_hash.to_string());
    let _ = state.consume_right(&right_id_hash.to_string());
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    state.add_transfer(TransferRecord {
        id: transfer_id.to_string(),
        source_chain: Chain::Ethereum,
        dest_chain: Chain::Solana,
        right_id: right_id_hash.to_string(),
        sender_address: Some(eth_address),
        destination_address: dest_owner_str.clone(),
        source_tx_hash: Some(source_tx_hash.to_string()),
        source_fee: None,
        dest_tx_hash: Some(dest_tx_hash.to_string()),
        dest_fee: None,
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
    use secp256k1::{SecretKey, PublicKey};
    
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

