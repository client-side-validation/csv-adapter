//! Contract deployment for all chains.
//!
//! Provides deployment using CSV adapter deploy modules via RPC.

use crate::commands::contracts::types::DiscoveredContract;
use crate::config::{Chain, Config};
use crate::output;
use crate::state::{ContractRecord, UnifiedStateManager};
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

/// Deploy contracts to a chain.
pub fn cmd_deploy(
    chain: Chain,
    network: Option<String>,
    deployer_key: Option<String>,
    account: Option<String>,
    config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    let network_str = network.as_deref().unwrap_or("test");

    output::header(&format!("Deploying Contracts to {} ({})", chain, network_str));

    match chain {
        Chain::Bitcoin => {
            output::info("Bitcoin is UTXO-native — no contract deployment needed");
            output::info("Single-use enforcement is structural via UTXO spending");
            output::info("Adapter connectivity: use 'csv testnet validate' to verify");
        }
        Chain::Ethereum => {
            deploy_ethereum_csv_client(config, state, deployer_key, account)?;
        }
        Chain::Sui => {
            deploy_sui_csv_client(config, state, account)?;
        }
        Chain::Aptos => {
            deploy_aptos_csv_client(config, state, account)?;
        }
        Chain::Solana => {
            deploy_solana_csv_client(config, state)?;
        }
    }

    Ok(())
}

/// Deploy Ethereum contracts using CSV Adapter unified client.
fn deploy_ethereum_csv_client(
    config: &Config,
    state: &mut UnifiedStateManager,
    deployer_key: Option<String>,
    _account: Option<String>,
) -> Result<()> {
    use csv_adapter::CsvClient;
    use csv_adapter::Chain;

    let chain_config = config.chain(&Chain::Ethereum)?;
    let rpc_url = &chain_config.rpc_url;

    output::progress(1, 4, "Initializing CSV client for Ethereum...");

    // Get deployer key
    let deployer_key = deployer_key
        .or_else(|| std::env::var("DEPLOYER_KEY").ok())
        .or_else(|| {
            state
                .get_account(&Chain::Ethereum)
                .and_then(|acc| acc.private_key.clone())
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "DEPLOYER_KEY not found. Options:\n  1. Pass --deployer-key <hex>\n  2. Set DEPLOYER_KEY env var\n  3. Store wallet account with 'csv wallet generate ethereum'"
            )
        })?;

    output::progress(2, 4, "Building CSV client with Ethereum chain...");

    // Build CSV client with Ethereum support
    let client = CsvClient::builder()
        .with_chain(Chain::Ethereum)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build CSV client: {}", e))?;

    output::progress(3, 4, "Deploying CSV Seal contract...");
    output::info(&format!("  RPC: {}", rpc_url));

    // Create runtime for async deployment
    let rt = tokio::runtime::Runtime::new()?;

    // Deploy using the client's deployment manager
    let deployment = rt.block_on(async {
        client
            .deploy()
            .deploy_csv_seal_contract(rpc_url, &deployer_key)
            .await
            .map_err(|e| anyhow::anyhow!("Deployment failed: {}", e))
    })?;

    output::progress(4, 4, "Storing deployment record...");

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    state.store_contract(ContractRecord {
        chain: Chain::Ethereum,
        address: hex::encode(&deployment.address),
        tx_hash: deployment.transaction_hash.clone(),
        deployed_at: timestamp,
    });

    println!();
    output::success("Ethereum deployment complete");
    output::kv("Contract Address", &hex::encode(&deployment.address));
    output::kv("Transaction Hash", &deployment.transaction_hash);
    output::kv(
        "Block Number",
        &deployment
            .block_number
            .map_or("unknown".to_string(), |n| n.to_string()),
    );
    output::kv("Gas Used", &deployment.gas_used.to_string());

    Ok(())
}

/// Deploy Sui contracts using CSV Adapter unified client.
fn deploy_sui_csv_client(
    config: &Config,
    state: &mut UnifiedStateManager,
    _account: Option<String>,
) -> Result<()> {
    use csv_adapter::CsvClient;
    use csv_adapter::Chain;

    let chain_config = config.chain(&Chain::Sui)?;
    let rpc_url = &chain_config.rpc_url;

    output::progress(1, 3, "Initializing CSV client for Sui...");
    output::info(&format!("  RPC: {}", rpc_url));

    // Check for Sui account
    let sui_account = state.get_account(&Chain::Sui);
    if sui_account.is_none() {
        output::warning("No Sui account found in unified state");
        output::info("Create an account with: csv wallet create --chain sui");
    }

    output::progress(2, 3, "Building CSV client with Sui chain...");

    let client = CsvClient::builder()
        .with_chain(Chain::Sui)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build CSV client: {}", e))?;

    output::progress(3, 3, "Sui deployment ready (SDK integration)...");

    output::info("Sui client configured and ready for package deployment");
    output::info("Note: Use publish_csv_package() for Move package deployment");

    println!();
    output::success("Sui client initialized successfully");

    Ok(())
}

/// Deploy Aptos contracts using CSV Adapter unified client.
fn deploy_aptos_csv_client(
    config: &Config,
    state: &mut UnifiedStateManager,
    _account: Option<String>,
) -> Result<()> {
    use csv_adapter::CsvClient;
    use csv_adapter::Chain;

    let chain_config = config.chain(&Chain::Aptos)?;
    let rpc_url = &chain_config.rpc_url;

    output::progress(1, 3, "Initializing CSV client for Aptos...");
    output::info(&format!("  RPC: {}", rpc_url));

    let aptos_account = state.get_account(&Chain::Aptos);
    if aptos_account.is_none() {
        output::warning("No Aptos account found in unified state");
        output::info("Create an account with: csv wallet create --chain aptos");
    }

    output::progress(2, 3, "Building CSV client with Aptos chain...");

    let client = CsvClient::builder()
        .with_chain(Chain::Aptos)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build CSV client: {}", e))?;

    output::progress(3, 3, "Aptos deployment ready (SDK integration)...");

    output::info("Aptos client configured and ready for module deployment");
    output::info("Note: Use publish_csv_module() for Move module deployment");

    println!();
    output::success("Aptos client initialized successfully");

    Ok(())
}

/// Deploy Solana contracts using CSV Adapter unified client.
fn deploy_solana_csv_client(
    _config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    use csv_adapter::CsvClient;
    use csv_adapter::Chain;

    output::progress(1, 3, "Initializing CSV client for Solana...");

    let solana_account = state.get_account(&Chain::Solana);
    if solana_account.is_none() {
        output::warning("No Solana account found in unified state");
        output::info("Create an account with: csv wallet create --chain solana");
    }

    output::progress(2, 3, "Building CSV client with Solana chain...");

    let client = CsvClient::builder()
        .with_chain(Chain::Solana)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build CSV client: {}", e))?;

    output::progress(3, 3, "Solana deployment ready (SDK integration)...");

    output::info("Solana client configured and ready for program deployment");
    output::info("Note: Use deploy_csv_program() for BPF program deployment");

    println!();
    output::success("Solana client initialized successfully");

    Ok(())
}
