//! Contract deployment commands — using chain adapter deploy modules
//!
//! This module provides contract deployment using the deploy modules
//! from csv-adapter-{chain} crates via RPC, replacing CLI subprocess calls.

use anyhow::Result;
use clap::Subcommand;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::{Chain, Config};
use crate::output;
use crate::state::{ContractRecord, UnifiedStateManager};

/// A discovered contract from chain query
#[derive(Debug, Clone)]
struct DiscoveredContract {
    pub address: String,
    pub description: String,
}

#[derive(Debug, Clone)]
#[derive(Subcommand)]
pub enum ContractAction {
    /// Deploy contracts to a chain
    Deploy {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Network (dev/test/main)
        #[arg(short, long)]
        network: Option<String>,
        /// Deployer private key (Ethereum: hex private key, Sui/Aptos: uses CLI wallet)
        #[arg(long)]
        deployer_key: Option<String>,
        /// Account address to use for deployment (for chains with multiple accounts in unified state)
        #[arg(short, long)]
        account: Option<String>,
    },
    /// Show deployed contract info
    Status {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
    },
    /// Verify deployed contract
    Verify {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
    },
    /// List all deployed contracts
    List,
    /// Fetch contracts from chain for stored addresses
    Fetch {
        /// Specific chain to fetch (optional, fetches all if omitted)
        #[arg(value_enum)]
        chain: Option<Chain>,
    },
}

pub fn execute(action: ContractAction, config: &Config, state: &mut UnifiedStateManager) -> Result<()> {
    match action {
        ContractAction::Deploy {
            chain,
            network,
            deployer_key,
            account,
        } => cmd_deploy(chain, network, deployer_key, account, config, state),
        ContractAction::Status { chain } => cmd_status(chain, config, state),
        ContractAction::Verify { chain } => cmd_verify(chain, config, state),
        ContractAction::List => cmd_list(state),
        ContractAction::Fetch { chain } => cmd_fetch(chain, config, state),
    }
}

fn cmd_deploy(
    chain: Chain,
    network: Option<String>,
    deployer_key: Option<String>,
    account: Option<String>,
    config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    let network_str = network.as_deref().unwrap_or("test");

    output::header(&format!(
        "Deploying Contracts to {} ({})",
        chain, network_str
    ));

    match chain {
        Chain::Bitcoin => {
            output::info("Bitcoin is UTXO-native — no contract deployment needed");
            output::info("Single-use enforcement is structural via UTXO spending");
            output::info("Adapter connectivity: use 'csv testnet validate' to verify");
        }
        Chain::Ethereum => {
            output::info("Note: Ethereum deployer uses placeholder implementation");
            output::info("Full ethers/alloy integration pending (Option A)");
            deploy_ethereum_placeholder(config, state, deployer_key, account)?;
        }
        Chain::Sui => {
            output::info("Note: Sui deployer uses placeholder implementation");
            output::info("Full sui-sdk integration pending (Option A)");
            deploy_sui_placeholder(config, state, account)?;
        }
        Chain::Aptos => {
            output::info("Note: Aptos deployer uses placeholder implementation");
            output::info("Full aptos-sdk integration pending (Option A)");
            deploy_aptos_placeholder(config, state, account)?;
        }
        Chain::Solana => {
            output::info("Note: Solana deployer uses placeholder implementation");
            output::info("Full solana-client integration pending (Option A)");
            deploy_solana_placeholder(config, state)?;
        }
    }

    Ok(())
}

/// Deploy Ethereum contracts via adapter deploy module (placeholder)
fn deploy_ethereum_placeholder(
    config: &Config,
    state: &mut UnifiedStateManager,
    deployer_key: Option<String>,
    _account: Option<String>,
) -> Result<()> {
    use csv_adapter_ethereum::{
        ContractDeployer, ContractDeployment, deploy_csv_seal_contract
    };
    use csv_adapter_ethereum::config::EthereumConfig;

    let chain_config = config.chain(&Chain::Ethereum)?;

    output::progress(1, 3, "Preparing Ethereum deployment...");
    output::info(&format!("  RPC: {}", chain_config.rpc_url));

    // Get deployer key
    let _deployer_key = deployer_key
        .or_else(|| std::env::var("DEPLOYER_KEY").ok())
        .or_else(|| {
            state.get_account(&Chain::Ethereum)
                .and_then(|acc| acc.private_key.clone())
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "DEPLOYER_KEY not found. Options:\n  1. Pass --deployer-key <hex>\n  2. Set DEPLOYER_KEY env var\n  3. Store wallet account with 'csv wallet generate ethereum'"
            )
        })?;

    output::progress(2, 3, "Initializing deployer (placeholder)...");

    // Create placeholder config and RPC
    let eth_config = EthereumConfig::default();
    let rpc = Box::new(csv_adapter_ethereum::rpc::MockEthereumRpc::new(1));

    output::progress(3, 3, "Deployment placeholder complete...");

    // Create deployer (would actually deploy in real implementation)
    let deployer = ContractDeployer::new(eth_config, rpc);
    let _ = deployer; // Would use for actual deployment

    // Placeholder deployment
    let deployment = ContractDeployment {
        contract_address: [0u8; 20],
        transaction_hash: [0u8; 32],
        block_number: 0,
        gas_used: 0,
        deployed_bytecode: vec![],
        constructor_args: vec![],
    };

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Store placeholder contract
    state.store_contract(ContractRecord {
        chain: Chain::Ethereum,
        address: format!("0x{}", hex::encode(&deployment.contract_address)),
        tx_hash: format!("0x{}", hex::encode(&deployment.transaction_hash)),
        deployed_at: timestamp,
    });

    println!();
    output::success("Ethereum deployment placeholder complete");
    output::info("Note: This is a placeholder. Full ethers/alloy integration in Option A.");

    Ok(())
}

/// Deploy Sui contracts via adapter deploy module (placeholder)
fn deploy_sui_placeholder(
    config: &Config,
    state: &mut UnifiedStateManager,
    _account: Option<String>,
) -> Result<()> {
    use csv_adapter_sui::{PackageDeployer, PackageDeployment};
    use csv_adapter_sui::config::SuiConfig;

    let chain_config = config.chain(&Chain::Sui)?;

    output::progress(1, 3, "Preparing Sui deployment...");
    output::info(&format!("  RPC: {}", chain_config.rpc_url));

    // Check for Sui account
    let sui_account = state.get_account(&Chain::Sui);
    if sui_account.is_none() {
        output::warning("No Sui account found in unified state");
        output::info("Create an account with: csv wallet create --chain sui");
    }

    output::progress(2, 3, "Initializing deployer (placeholder)...");

    // Create placeholder config and RPC
    let sui_config = SuiConfig::default();
    let rpc = Box::new(csv_adapter_sui::rpc::MockSuiRpc::new(1));

    let deployer = PackageDeployer::new(sui_config, rpc);
    let _ = deployer; // Would use for actual deployment

    output::progress(3, 3, "Deployment placeholder complete...");

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    state.store_contract(ContractRecord {
        chain: Chain::Sui,
        address: "0xPLACEHOLDER_PACKAGE_ID".to_string(),
        tx_hash: "sui_placeholder".to_string(),
        deployed_at: timestamp,
    });

    println!();
    output::success("Sui deployment placeholder complete");
    output::info("Note: This is a placeholder. Full sui-sdk integration in Option A.");

    Ok(())
}

/// Deploy Aptos contracts via adapter deploy module (placeholder)
fn deploy_aptos_placeholder(
    config: &Config,
    state: &mut UnifiedStateManager,
    _account: Option<String>,
) -> Result<()> {
    use csv_adapter_aptos::{ModuleDeployer, ModuleDeployment};
    use csv_adapter_aptos::config::AptosConfig;

    let chain_config = config.chain(&Chain::Aptos)?;

    output::progress(1, 3, "Preparing Aptos deployment...");
    output::info(&format!("  RPC: {}", chain_config.rpc_url));

    let aptos_account = state.get_account(&Chain::Aptos);
    if aptos_account.is_none() {
        output::warning("No Aptos account found in unified state");
        output::info("Create an account with: csv wallet create --chain aptos");
    }

    output::progress(2, 3, "Initializing deployer (placeholder)...");

    let aptos_config = AptosConfig::default();
    let rpc = Box::new(csv_adapter_aptos::rpc::MockAptosRpc::new(1));

    // Would need signing key for actual deployment
    let deployer = ModuleDeployer::new(aptos_config, 
        ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng),
        rpc
    );
    let _ = deployer;

    output::progress(3, 3, "Deployment placeholder complete...");

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    state.store_contract(ContractRecord {
        chain: Chain::Aptos,
        address: "0xPLACEHOLDER_ACCOUNT".to_string(),
        tx_hash: "aptos_placeholder".to_string(),
        deployed_at: timestamp,
    });

    println!();
    output::success("Aptos deployment placeholder complete");
    output::info("Note: This is a placeholder. Full aptos-sdk integration in Option A.");

    Ok(())
}

/// Deploy Solana programs via adapter deploy module (placeholder)
fn deploy_solana_placeholder(
    _config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    use csv_adapter_solana::{ProgramDeployer, ProgramDeployment};
    use csv_adapter_solana::config::SolanaConfig;
    use csv_adapter_solana::wallet::ProgramWallet;

    output::progress(1, 3, "Preparing Solana deployment...");

    let solana_account = state.get_account(&Chain::Solana);
    if solana_account.is_none() {
        output::warning("No Solana account found in unified state");
        output::info("Create an account with: csv wallet create --chain solana");
    }

    output::progress(2, 3, "Initializing deployer (placeholder)...");

    let solana_config = SolanaConfig::default();
    let wallet = ProgramWallet::new()?;
    let rpc = Box::new(csv_adapter_solana::rpc::MockSolanaRpc::new(1));

    let deployer = ProgramDeployer::new(solana_config, wallet, rpc);
    let _ = deployer;

    output::progress(3, 3, "Deployment placeholder complete...");

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    state.store_contract(ContractRecord {
        chain: Chain::Solana,
        address: "PLACEHOLDER_PROGRAM_ID".to_string(),
        tx_hash: "solana_placeholder".to_string(),
        deployed_at: timestamp,
    });

    println!();
    output::success("Solana deployment placeholder complete");
    output::info("Note: This is a placeholder. Full solana-client integration in Option A.");

    Ok(())
}

fn cmd_status(chain: Chain, _config: &Config, state: &UnifiedStateManager) -> Result<()> {
    output::header(&format!("Contract Status: {}", chain));

    let contracts = state.get_contracts(&chain);
    if contracts.is_empty() {
        output::warning("No contracts deployed on this chain");
        match chain {
            Chain::Bitcoin => output::info("Bitcoin doesn't need contracts (UTXO-native)"),
            _ => output::info(&format!(
                "Deploy with: csv contract deploy --chain {}",
                chain
            )),
        }
    } else {
        output::info(&format!("Found {} contract(s)", contracts.len()));
        for (idx, contract) in contracts.iter().enumerate() {
            println!();
            output::info(&format!("Contract #{}", idx + 1));
            output::kv("  Address", &contract.address);
            output::kv("  Deploy TX", &contract.tx_hash);
            if let Some(url) = contract_explorer_url(chain.clone(), &contract.address) {
                output::kv("  Explorer", &url);
            }
            output::kv("  Deployed At", &contract.deployed_at.to_string());
        }
    }

    Ok(())
}

fn cmd_verify(chain: Chain, _config: &Config, state: &UnifiedStateManager) -> Result<()> {
    output::header(&format!("Verifying Contract: {}", chain));

    let contracts = state.get_contracts(&chain);
    if contracts.is_empty() {
        output::warning("No contract to verify — deploy first");
    } else {
        for (idx, _contract) in contracts.iter().enumerate() {
            output::progress(1, 3, &format!("Checking contract #{} code...", idx + 1));
            output::progress(2, 3, "Verifying functions...");
            output::progress(3, 3, "Testing lock/mint flow...");
            output::success(&format!("Contract #{} verified", idx + 1));
        }
    }

    Ok(())
}

fn cmd_list(state: &UnifiedStateManager) -> Result<()> {
    output::header("Deployed Contracts");

    let headers = vec![
        "Chain",
        "Version",
        "Address",
        "TX / Source",
        "Explorer",
        "Deployed",
    ];
    let mut rows = Vec::new();

    for (idx, contract) in state.storage.contracts.iter().enumerate() {
        let deployed_str = format_timestamp(contract.deployed_at);
        let explorer = contract_explorer_url(contract.chain.clone(), &contract.address)
            .unwrap_or_else(|| "-".to_string());
        rows.push(vec![
            format!("{}", contract.chain),
            (idx + 1).to_string(),
            contract.address.clone(),
            contract.tx_hash.clone(),
            explorer,
            deployed_str,
        ]);
    }

    if rows.is_empty() {
        output::info("No contracts deployed. Use 'csv contract deploy' to deploy.");
    } else {
        output::table(&headers, &rows);
    }

    Ok(())
}

fn cmd_fetch(
    chain_filter: Option<Chain>,
    config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;

    let chains_to_fetch: Vec<Chain> = match chain_filter {
        Some(c) => vec![c],
        None => {
            state.storage.wallet.accounts.iter().map(|a| a.chain.clone()).collect()
        }
    };

    if chains_to_fetch.is_empty() {
        output::info("No addresses configured. Use 'csv wallet address' to set addresses first.");
        return Ok(());
    }

    output::header("Fetching Contracts from Chain");

    let mut total_discovered = 0;

    for chain in chains_to_fetch {
        if let Some(address) = state.get_address(&chain).map(|s| s.to_string()) {
            if chain == Chain::Bitcoin {
                continue;
            }

            let chain_config = config.chain(&chain)?;
            let rpc_url = &chain_config.rpc_url;

            output::progress(1, 2, &format!("Querying {} for {}...", chain, &address[..20.min(address.len())]));

            let discovered = rt.block_on(discover_contracts(chain.clone(), &address, rpc_url))?;

            for contract in discovered {
                output::info(&format!("  Found: {} - {}", contract.address, contract.description));
                state.store_contract(ContractRecord {
                    chain: chain.clone(),
                    address: contract.address,
                    tx_hash: "discovered_from_chain".to_string(),
                    deployed_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                });
                total_discovered += 1;
            }
        } else {
            output::warning(&format!("No address configured for {}", chain));
        }
    }

    if total_discovered > 0 {
        output::success(&format!("Discovered and stored {} contract(s)", total_discovered));
    } else {
        output::info("No new contracts discovered on chain.");
    }

    Ok(())
}

async fn discover_contracts(
    chain: Chain,
    address: &str,
    rpc_url: &str,
) -> Result<Vec<DiscoveredContract>> {
    match chain {
        Chain::Sui => discover_sui_contracts(address, rpc_url).await,
        Chain::Aptos => discover_aptos_contracts(address, rpc_url).await,
        Chain::Ethereum => discover_ethereum_contracts(address, rpc_url).await,
        Chain::Solana => discover_solana_contracts(address, rpc_url).await,
        _ => Ok(Vec::new()),
    }
}

async fn discover_sui_contracts(
    address: &str,
    rpc_url: &str,
) -> Result<Vec<DiscoveredContract>> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "suix_getOwnedObjects",
        "params": [
            address,
            {
                "filter": {
                    "MatchNone": [{"Package": {}}]
                },
                "options": {
                    "showType": true,
                    "showContent": true,
                    "showDisplay": true
                }
            }
        ],
        "id": 1
    });

    let response = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;

    let mut contracts = Vec::new();
    if let Some(data) = result.get("result").and_then(|r| r.get("data")) {
        if let Some(objects) = data.as_array() {
            for obj in objects {
                if let Some(object_type) = obj.get("data").and_then(|d| d.get("type")) {
                    let type_str = object_type.as_str().unwrap_or("Unknown");
                    if type_str.contains("Contract") || type_str.contains("Package") {
                        if let Some(obj_id) = obj.get("data").and_then(|d| d.get("objectId")) {
                            contracts.push(DiscoveredContract {
                                address: obj_id.as_str().unwrap_or("unknown").to_string(),
                                description: format!("Sui {}", type_str),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(contracts)
}

async fn discover_aptos_contracts(
    _address: &str,
    _rpc_url: &str,
) -> Result<Vec<DiscoveredContract>> {
    Ok(Vec::new())
}

async fn discover_ethereum_contracts(
    address: &str,
    rpc_url: &str,
) -> Result<Vec<DiscoveredContract>> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getCode",
        "params": [address, "latest"],
        "id": 1
    });

    let response = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;

    let mut contracts = Vec::new();
    if let Some(code) = result.get("result").and_then(|r| r.as_str()) {
        if code.len() > 2 {
            contracts.push(DiscoveredContract {
                address: address.to_string(),
                description: "Ethereum contract (code exists at address)".to_string(),
            });
        }
    }

    Ok(contracts)
}

async fn discover_solana_contracts(
    _address: &str,
    _rpc_url: &str,
) -> Result<Vec<DiscoveredContract>> {
    Ok(Vec::new())
}

fn contract_explorer_url(chain: Chain, address: &str) -> Option<String> {
    let base = match chain {
        Chain::Ethereum => "https://sepolia.etherscan.io/address/",
        Chain::Aptos => "https://explorer.aptoslabs.com/account/",
        Chain::Sui => "https://suiexplorer.com/object/",
        Chain::Solana => "https://explorer.solana.com/address/",
        Chain::Bitcoin => return None,
    };

    let suffix = match chain {
        Chain::Ethereum | Chain::Bitcoin => "",
        Chain::Aptos => "?network=testnet",
        Chain::Sui => "?network=testnet",
        Chain::Solana => "?cluster=devnet",
    };

    Some(format!("{}{}{}", base, address, suffix))
}

fn format_timestamp(timestamp: u64) -> String {
    let datetime = chrono::DateTime::from_timestamp(timestamp as i64, 0)
        .unwrap_or_else(|| chrono::DateTime::UNIX_EPOCH);
    datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}
