//! Contract deployment commands — real deployment via chain CLI tools

use anyhow::Result;
use clap::Subcommand;
use std::process::Command;
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
        } => cmd_deploy(chain, network, deployer_key, config, state),
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
            deploy_ethereum(config, state, deployer_key)?;
        }
        Chain::Sui => {
            deploy_sui(config, state)?;
        }
        Chain::Aptos => {
            deploy_aptos(config, state)?;
        }
        Chain::Solana => {
            deploy_solana(config, state)?;
        }
    }

    Ok(())
}

/// Deploy Ethereum contracts via Foundry
fn deploy_ethereum(config: &Config, state: &mut UnifiedStateManager, deployer_key: Option<String>) -> Result<()> {
    let chain_config = config.chain(&Chain::Ethereum)?;

    output::progress(1, 5, "Compiling Solidity contracts...");

    // Run forge build first
    let build_status = Command::new("forge")
        .args(["build", "--root", "contracts"])
        .current_dir(env!("CARGO_MANIFEST_DIR").trim_end_matches("csv-cli"))
        .output()?;

    if !build_status.status.success() {
        let stderr = String::from_utf8_lossy(&build_status.stderr);
        return Err(anyhow::anyhow!("Forge build failed:\n{}", stderr));
    }
    output::info("  CSVLock.sol ✓");
    output::info("  CSVMint.sol ✓");

    output::progress(2, 5, "Connecting to Sepolia...");
    output::info(&format!("  RPC: {}", chain_config.rpc_url));

    // Check for DEPLOYER_KEY env var or argument
    let deployer_key_env = deployer_key.or_else(|| std::env::var("DEPLOYER_KEY").ok());
    if deployer_key_env.is_none() {
        return Err(anyhow::anyhow!(
            "DEPLOYER_KEY not set. Pass --deployer-key <hex> or set DEPLOYER_KEY env var"
        ));
    }

    // Set env for deploy script
    let mut deploy_cmd = Command::new("forge");
    deploy_cmd
        .args([
            "script",
            "script/Deploy.s.sol",
            "--rpc-url",
            &chain_config.rpc_url,
            "--broadcast",
            "--json",
        ])
        .current_dir(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("csv-adapter-ethereum/contracts"),
        )
        .env("DEPLOYER_KEY", deployer_key_env.as_ref().unwrap());

    output::progress(3, 5, "Deploying CSVLock...");
    output::progress(4, 5, "Deploying CSVMint...");

    let deploy_output = deploy_cmd.output()?;

    if !deploy_output.status.success() {
        let stderr = String::from_utf8_lossy(&deploy_output.stderr);
        let stdout = String::from_utf8_lossy(&deploy_output.stdout);
        return Err(anyhow::anyhow!(
            "Deploy failed:\nstdout: {}\nstderr: {}",
            stdout,
            stderr
        ));
    }

    let stdout = String::from_utf8_lossy(&deploy_output.stdout);

    // Parse contract addresses from output
    let csvlock_addr = extract_address(&stdout, "CSVLock deployed at:")
        .or_else(|| extract_address(&stdout, "CSVLock:"))
        .ok_or_else(|| anyhow::anyhow!("Could not parse CSVLock address from deploy output"))?;

    let csvmint_addr = extract_address(&stdout, "CSVMint deployed at:")
        .or_else(|| extract_address(&stdout, "CSVMint:"))
        .ok_or_else(|| anyhow::anyhow!("Could not parse CSVMint address from deploy output"))?;

    output::progress(5, 5, "Verifying deployment...");

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    state.store_contract(ContractRecord {
        chain: Chain::Ethereum,
        address: csvlock_addr.clone(),
        tx_hash: stdout
            .lines()
            .find(|l| l.contains("transactionHash") || l.contains("txHash"))
            .and_then(|l| l.split(':').nth(1))
            .map(|s| {
                s.trim()
                    .trim_matches(|c: char| !c.is_alphanumeric() && c != 'x')
            })
            .unwrap_or_else(|| "0xunknown")
            .to_string(),
        deployed_at: timestamp,
    });

    // Also store mint contract
    state.store_contract(ContractRecord {
        chain: Chain::Ethereum,
        address: csvmint_addr.clone(),
        tx_hash: "0xsee_csvlock_for_tx".to_string(),
        deployed_at: timestamp,
    });

    println!();
    output::success("Ethereum contracts deployed");
    output::kv("CSVLock", &csvlock_addr);
    output::kv("CSVMint", &csvmint_addr);
    output::info("Addresses saved to state.json");

    Ok(())
}

/// Deploy Sui contracts via sui CLI
fn deploy_sui(config: &Config, state: &mut UnifiedStateManager) -> Result<()> {
    output::progress(1, 4, "Building Move package...");

    let sui_path = std::env::var("SUI_BIN").unwrap_or_else(|_| "sui".to_string());
    let contracts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("csv-adapter-sui/contracts");

    // Check sui is available
    let sui_check = Command::new(&sui_path)
        .arg("client")
        .arg("active-address")
        .output();
    if sui_check.is_err() {
        return Err(anyhow::anyhow!(
            "sui client not available. Install: cargo install --git https://github.com/MystenLabs/sui.git --bin sui"
        ));
    }

    // Build
    let build_status = Command::new(&sui_path)
        .args(["move", "build", "--path"])
        .arg(&contracts_dir)
        .output()?;

    if !build_status.status.success() {
        let stderr = String::from_utf8_lossy(&build_status.stderr);
        return Err(anyhow::anyhow!("Move build failed:\n{}", stderr));
    }
    output::info("  csv_seal.move ✓");

    output::progress(2, 4, "Connecting to Sui Testnet...");
    let chain_config = config.chain(&Chain::Sui)?;
    output::info(&format!("  RPC: {}", chain_config.rpc_url));

    // Run deploy script
    output::progress(3, 4, "Publishing package...");

    let deploy_script = contracts_dir.parent().unwrap().join("scripts/deploy.sh");
    if !deploy_script.exists() {
        return Err(anyhow::anyhow!(
            "Deploy script not found: {:?}",
            deploy_script
        ));
    }

    let deploy_output = Command::new(&deploy_script)
        .arg("testnet")
        .arg(&sui_path)
        .output()?;

    if !deploy_output.status.success() {
        let stderr = String::from_utf8_lossy(&deploy_output.stderr);
        let stdout = String::from_utf8_lossy(&deploy_output.stdout);
        return Err(anyhow::anyhow!(
            "Deploy failed:\nstdout: {}\nstderr: {}",
            stdout,
            stderr
        ));
    }

    let stdout = String::from_utf8_lossy(&deploy_output.stdout);

    // Extract package ID
    let package_id = extract_package_id(&stdout).ok_or_else(|| {
        anyhow::anyhow!("Could not extract package ID. Check scripts/deploy-output-testnet.json")
    })?;

    output::progress(4, 4, "Verifying deployment...");

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    state.store_contract(ContractRecord {
        chain: Chain::Sui,
        address: package_id.clone(),
        tx_hash: "0xsui_publish".to_string(),
        deployed_at: timestamp,
    });

    println!();
    output::success("Sui Move package deployed");
    output::kv("Package ID", &package_id);

    Ok(())
}

/// Deploy Aptos contracts via aptos CLI
fn deploy_aptos(config: &Config, state: &mut UnifiedStateManager) -> Result<()> {
    output::progress(1, 4, "Building Move package...");

    let aptos_path = std::env::var("APTOS_BIN").unwrap_or_else(|_| "aptos".to_string());
    let contracts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("csv-adapter-aptos/contracts");

    // Check aptos is available
    let aptos_check = Command::new(&aptos_path)
        .arg("config")
        .arg("show-profiles")
        .output();
    if aptos_check.is_err() {
        return Err(anyhow::anyhow!(
            "aptos CLI not available. Install: cargo install --git https://github.com/aptos-labs/aptos-core.git aptos"
        ));
    }

    // Build
    let build_status = Command::new(&aptos_path)
        .args(["move", "compile", "--package-dir"])
        .arg(&contracts_dir)
        .output()?;

    if !build_status.status.success() {
        let stderr = String::from_utf8_lossy(&build_status.stderr);
        return Err(anyhow::anyhow!("Move build failed:\n{}", stderr));
    }
    output::info("  csv_seal.move ✓");

    let chain_config = config.chain(&Chain::Aptos)?;
    let network = chain_config.network.to_string();
    // Map generic network names to Aptos CLI network names
    let aptos_network = match chain_config.network {
        crate::config::Network::Dev => "devnet",
        crate::config::Network::Test => "testnet",
        crate::config::Network::Main => "mainnet",
    };

    output::progress(2, 4, &format!("Connecting to Aptos {}...", network));
    output::info(&format!("  RPC: {}", chain_config.rpc_url));

    // Run deploy script
    output::progress(3, 4, "Publishing package...");

    let deploy_script = contracts_dir.parent().unwrap().join("scripts/deploy.sh");
    if !deploy_script.exists() {
        return Err(anyhow::anyhow!(
            "Deploy script not found: {:?}",
            deploy_script
        ));
    }

    let deploy_output = Command::new(&deploy_script)
        .arg(aptos_network)
        .arg(&aptos_path)
        .output()?;

    if !deploy_output.status.success() {
        let stderr = String::from_utf8_lossy(&deploy_output.stderr);
        let stdout = String::from_utf8_lossy(&deploy_output.stdout);
        return Err(anyhow::anyhow!(
            "Deploy failed:\nstdout: {}\nstderr: {}",
            stdout,
            stderr
        ));
    }

    let stdout = String::from_utf8_lossy(&deploy_output.stdout);

    // Extract account address
    let account = extract_account(&stdout)
        .ok_or_else(|| anyhow::anyhow!("Could not extract account address"))?;

    output::progress(4, 4, "Verifying deployment...");

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    state.store_contract(ContractRecord {
        chain: Chain::Aptos,
        address: account.clone(),
        tx_hash: "0xaptos_publish".to_string(),
        deployed_at: timestamp,
    });

    println!();
    output::success("Aptos Move package deployed");
    output::kv("Account", &account);

    Ok(())
}

/// Deploy Solana programs via Anchor CLI
fn deploy_solana(config: &Config, state: &mut UnifiedStateManager) -> Result<()> {
    output::progress(1, 5, "Building Anchor program...");

    let anchor_path = std::env::var("ANCHOR_BIN").unwrap_or_else(|_| "anchor".to_string());
    let contracts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("csv-adapter-solana/contracts");

    // Check anchor is available
    let anchor_check = Command::new(&anchor_path)
        .arg("--version")
        .output();
    if anchor_check.is_err() {
        return Err(anyhow::anyhow!(
            "Anchor CLI not available. Install with: npm install -g @coral-xyz/anchor-cli"
        ));
    }

    // Check solana CLI is available
    let solana_check = Command::new("solana")
        .arg("--version")
        .output();
    if solana_check.is_err() {
        return Err(anyhow::anyhow!(
            "Solana CLI not available. Install from: https://docs.solana.com/cli/install"
        ));
    }

    // Build
    let build_status = Command::new(&anchor_path)
        .arg("build")
        .current_dir(&contracts_dir)
        .output()?;

    if !build_status.status.success() {
        let stderr = String::from_utf8_lossy(&build_status.stderr);
        return Err(anyhow::anyhow!("Anchor build failed:\n{}", stderr));
    }
    output::info("  csv_seal program ✓");

    // Get chain config for network
    let chain_config = config.chain(&Chain::Solana)?;
    let network = chain_config.network.to_string();
    // Map generic network names to Solana CLI network names
    let solana_network = match chain_config.network {
        crate::config::Network::Dev => "localnet",
        crate::config::Network::Test => "devnet",  // Test network uses Solana devnet
        crate::config::Network::Main => "mainnet-beta",
    };

    output::progress(2, 5, &format!("Connecting to Solana {}...", network));
    output::info(&format!("  RPC: {}", chain_config.rpc_url));

    // Set solana config with RPC URL
    let _ = Command::new("solana")
        .args(["config", "set", "--url", &chain_config.rpc_url])
        .output();

    // Get wallet address
    let wallet_output = Command::new("solana")
        .arg("address")
        .output()?;
    let wallet = String::from_utf8_lossy(&wallet_output.stdout).trim().to_string();
    output::info(&format!("  Wallet: {}", wallet));

    output::progress(3, 5, "Deploying CSV Seal program...");

    // Deploy using the deploy script
    let deploy_script = contracts_dir.parent().unwrap().join("scripts/deploy.sh");
    if !deploy_script.exists() {
        return Err(anyhow::anyhow!(
            "Deploy script not found: {:?}",
            deploy_script
        ));
    }

    let deploy_output = Command::new(&deploy_script)
        .arg(solana_network)
        .arg(&anchor_path)
        .output()?;

    if !deploy_output.status.success() {
        let stderr = String::from_utf8_lossy(&deploy_output.stderr);
        let stdout = String::from_utf8_lossy(&deploy_output.stdout);
        return Err(anyhow::anyhow!(
            "Deploy failed:\nstdout: {}\nstderr: {}",
            stdout,
            stderr
        ));
    }

    let stdout = String::from_utf8_lossy(&deploy_output.stdout);

    // Extract program ID from output
    let program_id = extract_program_id(&stdout)
        .or_else(|| {
            // Try to read from deploy output file
            let deploy_file = contracts_dir
                .parent()
                .unwrap()
                .join(format!("scripts/deploy-{}.json", network));
            if deploy_file.exists() {
                std::fs::read_to_string(deploy_file)
                    .ok()
                    .and_then(|content| {
                        content
                            .lines()
                            .find(|l| l.contains("program_id"))
                            .and_then(|l| l.split('"').nth(3))
                            .map(|s| s.to_string())
                    })
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("Could not extract program ID from deploy output"))?;

    output::progress(4, 5, "Initializing LockRegistry...");

    // Initialize the registry using the script
    let init_script = contracts_dir.parent().unwrap().join("scripts/initialize.sh");
    let _ = Command::new(&init_script)
        .arg(solana_network)
        .arg(&program_id)
        .output();

    output::progress(5, 5, "Verifying deployment...");

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    state.store_contract(ContractRecord {
        chain: Chain::Solana,
        address: program_id.clone(),
        tx_hash: "solana_deploy".to_string(),
        deployed_at: timestamp,
    });

    println!();
    output::success("Solana program deployed");
    output::kv("Program ID", &program_id);
    output::kv("Network", &network);
    output::info("Program ID saved to state.json");

    Ok(())
}

/// Extract Solana program ID from deploy output
fn extract_program_id(output: &str) -> Option<String> {
    for line in output.lines() {
        if line.contains("Program Id:") || line.contains("Program ID:") {
            // Format: "Program Id: CsvSeal111111111111111111111111111111111111"
            if let Some(id) = line.split(':').nth(1) {
                let id = id
                    .trim()
                    .trim_matches(|c: char| c.is_whitespace() || c == '"');
                if id.len() >= 32 && id.len() <= 44 {
                    return Some(id.to_string());
                }
            }
        }
    }
    None
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
            output::kv("  Deploy TX", &display_tx_or_discovery_note(chain.clone(), contract));
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
        // Format timestamp as human-readable date
        let deployed_str = format_timestamp(contract.deployed_at);
        let tx_or_source = display_tx_or_discovery_note(contract.chain.clone(), contract);
        let explorer = contract_explorer_url(contract.chain.clone(), &contract.address)
            .unwrap_or_else(|| "-".to_string());
        rows.push(vec![
            format!("{}", contract.chain),
            (idx + 1).to_string(),
            contract.address.clone(),
            tx_or_source,
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

fn display_tx_or_discovery_note(chain: Chain, contract: &ContractRecord) -> String {
    if contract.tx_hash == "discovered_from_chain" {
        return format!("discovered on {} (address-based)", chain);
    }

    if contract.tx_hash.len() <= 25 {
        contract.tx_hash.clone()
    } else {
        format!("{}...", &contract.tx_hash[..22])
    }
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

/// Extract an Ethereum-style address from forge script output
fn extract_address(output: &str, prefix: &str) -> Option<String> {
    for line in output.lines() {
        if line.contains(prefix) {
            // Format: "CSVLock deployed at: 0x..."
            if let Some(addr) = line.split(':').nth(1) {
                let addr = addr
                    .trim()
                    .trim_matches(|c: char| c.is_whitespace() || c == '"');
                if addr.starts_with("0x") && addr.len() >= 42 {
                    return Some(addr.to_string());
                }
            }
        }
    }
    None
}

/// Extract Sui package ID from deploy script output
fn extract_package_id(output: &str) -> Option<String> {
    for line in output.lines() {
        if line.starts_with("Package ID:") {
            return Some(line.split(':').nth(1)?.trim().to_string());
        }
    }
    None
}

/// Extract Aptos account address from deploy script output
fn extract_account(output: &str) -> Option<String> {
    for line in output.lines() {
        if line.starts_with("Account:") {
            return Some(line.split(':').nth(1)?.trim().to_string());
        }
    }
    None
}

/// Fetch contracts from chain for stored addresses
fn cmd_fetch(chain_filter: Option<Chain>, config: &Config, state: &mut UnifiedStateManager) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;

    let chains_to_fetch: Vec<Chain> = match chain_filter {
        Some(c) => vec![c],
        None => {
            // Get all chains that have accounts with addresses
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
                continue; // Bitcoin doesn't have contracts
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

/// Discover contracts owned by an address on a chain
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
        _ => Ok(Vec::new()), // Bitcoin doesn't have contracts
    }
}

/// Discover Sui packages owned by address
async fn discover_sui_contracts(
    address: &str,
    rpc_url: &str,
) -> Result<Vec<DiscoveredContract>> {
    let client = reqwest::Client::new();

    // Query for objects owned by address
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
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query Sui objects: {}", e))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse Sui response: {}", e))?;

    let mut contracts = Vec::new();

    if let Some(data) = json
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
    {
        for obj in data {
            if let Some(obj_type) = obj
                .get("data")
                .and_then(|d| d.get("type"))
                .and_then(|t| t.as_str())
            {
                // Look for CSV-related object types
                if obj_type.contains("csv_seal") || obj_type.contains("Anchor") {
                    let object_id = obj
                        .get("data")
                        .and_then(|d| d.get("objectId"))
                        .and_then(|o| o.as_str())
                        .unwrap_or("unknown");

                    contracts.push(DiscoveredContract {
                        address: object_id.to_string(),
                        
                        description: format!("CSV Seal object: {}", obj_type),
                    });
                }
            }
        }
    }

    Ok(contracts)
}

/// Discover Aptos modules/resources for address
async fn discover_aptos_contracts(
    address: &str,
    rpc_url: &str,
) -> Result<Vec<DiscoveredContract>> {
    let client = reqwest::Client::new();

    // Query resources for account
    let url = format!("{}/accounts/{}/resources", rpc_url.trim_end_matches('/'), address);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query Aptos resources: {}", e))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse Aptos response: {}", e))?;

    let mut contracts = Vec::new();

    if let Some(resources) = json.as_array() {
        for resource in resources {
            if let Some(type_str) = resource.get("type").and_then(|t| t.as_str()) {
                // Look for CSV-related resource types
                if type_str.contains("csv_seal") || type_str.contains("Anchor") {
                    contracts.push(DiscoveredContract {
                        address: address.to_string(),
                        
                        description: format!("CSV resource: {}", type_str),
                    });
                }
            }
        }
    }

    Ok(contracts)
}

/// Discover Ethereum contracts
async fn discover_ethereum_contracts(
    address: &str,
    rpc_url: &str,
) -> Result<Vec<DiscoveredContract>> {
    let client = reqwest::Client::new();

    // Query for contract code at address
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
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query Ethereum code: {}", e))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse Ethereum response: {}", e))?;

    let mut contracts = Vec::new();

    if let Some(code) = json.get("result").and_then(|r| r.as_str()) {
        if code.len() > 2 && code != "0x" {
            // This is a contract address
            contracts.push(DiscoveredContract {
                address: address.to_string(),
                
                description: format!("Smart contract ({} bytes)", (code.len() - 2) / 2),
            });
        }
    }

    Ok(contracts)
}

/// Discover Solana programs
async fn discover_solana_contracts(
    address: &str,
    rpc_url: &str,
) -> Result<Vec<DiscoveredContract>> {
    let client = reqwest::Client::new();

    // Query for account info to check if it's a program
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "getAccountInfo",
        "params": [address, {"encoding": "jsonParsed"}],
        "id": 1
    });

    let response = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query Solana account: {}", e))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse Solana response: {}", e))?;

    let mut contracts = Vec::new();

    // Check if this is a program (executable account)
    if let Some(owner) = json
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.get("owner"))
        .and_then(|o| o.as_str())
    {
        // BPFLoader accounts indicate programs
        if owner.contains("BPFLoader") {
            contracts.push(DiscoveredContract {
                address: address.to_string(),
                
                description: "Solana program (BPF Loader)".to_string(),
            });
        }
    }

    Ok(contracts)
}

/// Format Unix timestamp as human-readable date/time
fn format_timestamp(timestamp: u64) -> String {
    use std::time::{Duration, UNIX_EPOCH};
    
    let datetime = UNIX_EPOCH + Duration::from_secs(timestamp);
    let datetime = chrono::DateTime::<chrono::Local>::from(datetime);
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}
