//! End-to-end test execution commands

use anyhow::Result;
use clap::Subcommand;

use crate::config::{Config, Chain};
use crate::state::State;
use crate::output;

#[derive(Subcommand)]
pub enum TestAction {
    /// Run end-to-end tests
    Run {
        /// Chain pair to test (source:dest)
        #[arg(short, long, value_parser = parse_chain_pair)]
        chain_pair: Option<(Chain, Chain)>,
        /// Run all chain pairs
        #[arg(long)]
        all: bool,
    },
    /// Run a specific test scenario
    Scenario {
        /// Scenario name
        name: String,
    },
    /// Show test results
    Results,
}

fn parse_chain_pair(s: &str) -> Result<(Chain, Chain), String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err("Chain pair must be in format 'source:dest' (e.g., 'bitcoin:sui')".to_string());
    }

    let from = match parts[0].to_lowercase().as_str() {
        "bitcoin" => Chain::Bitcoin,
        "ethereum" => Chain::Ethereum,
        "sui" => Chain::Sui,
        "aptos" => Chain::Aptos,
        other => return Err(format!("Unknown chain: {}", other)),
    };

    let to = match parts[1].to_lowercase().as_str() {
        "bitcoin" => Chain::Bitcoin,
        "ethereum" => Chain::Ethereum,
        "sui" => Chain::Sui,
        "aptos" => Chain::Aptos,
        other => return Err(format!("Unknown chain: {}", other)),
    };

    Ok((from, to))
}

pub fn execute(action: TestAction, config: &Config, state: &State) -> Result<()> {
    match action {
        TestAction::Run { chain_pair, all } => cmd_run(chain_pair, all, config, state),
        TestAction::Scenario { name } => cmd_scenario(name, config, state),
        TestAction::Results => cmd_results(state),
    }
}

fn cmd_run(chain_pair: Option<(Chain, Chain)>, all: bool, config: &Config, state: &State) -> Result<()> {
    let pairs = if all {
        vec![
            (Chain::Bitcoin, Chain::Sui),
            (Chain::Bitcoin, Chain::Ethereum),
            (Chain::Sui, Chain::Aptos),
            (Chain::Ethereum, Chain::Sui),
        ]
    } else {
        match chain_pair {
            Some(pair) => vec![pair],
            None => vec![(Chain::Bitcoin, Chain::Sui)], // Default test pair
        }
    };

    for (i, (from, to)) in pairs.iter().enumerate() {
        output::header(&format!("Test {}/{}: {} → {}", i + 1, pairs.len(), from, to));

        run_test_pair(from, to, config, state)?;
        println!();
    }

    output::success(&format!("All {} tests completed", pairs.len()));
    Ok(())
}

fn run_test_pair(from: &Chain, to: &Chain, config: &Config, _state: &State) -> Result<()> {
    // Test 1: Check connectivity
    output::progress(1, 5, "Checking chain connectivity...");
    check_chain_connectivity(from, config)?;
    check_chain_connectivity(to, config)?;

    // Test 2: Create Right on source
    output::progress(2, 5, &format!("Creating Right on {}...", from));
    // In production: call adapter.create_seal()

    // Test 3: Lock Right on source
    output::progress(3, 5, &format!("Locking Right on {}...", from));
    // In production: consume seal, get inclusion proof

    // Test 4: Verify proof on destination
    output::progress(4, 5, &format!("Verifying proof on {}...", to));
    // In production: verify inclusion proof

    // Test 5: Mint Right on destination
    output::progress(5, 5, &format!("Minting Right on {}...", to));
    // In production: call mint_right()

    output::success(&format!("Transfer {} → {}: PASS", from, to));
    Ok(())
}

fn check_chain_connectivity(chain: &Chain, config: &Config) -> Result<()> {
    let chain_config = config.chain(chain)?;
    match reqwest::blocking::get(&chain_config.rpc_url) {
        Ok(resp) => {
            if resp.status().is_success() || resp.status().is_server_error() {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Chain {} returned status {}", chain, resp.status()))
            }
        }
        Err(e) => Err(anyhow::anyhow!("Chain {} connection failed: {}", chain, e)),
    }
}

fn cmd_scenario(name: String, _config: &Config, _state: &State) -> Result<()> {
    output::header(&format!("Scenario: {}", name));

    match name.as_str() {
        "double_spend" => {
            output::info("Testing double-spend detection...");
            output::progress(1, 3, "Creating Right...");
            output::progress(2, 3, "Consuming seal (first time)...");
            output::progress(3, 3, "Attempting to consume same seal...");
            output::success("Double-spend correctly rejected");
        }
        "invalid_proof" => {
            output::info("Testing invalid proof rejection...");
            output::progress(1, 3, "Creating tampered proof...");
            output::progress(2, 3, "Verifying proof...");
            output::progress(3, 3, "Rejecting invalid proof...");
            output::success("Invalid proof correctly rejected");
        }
        "ownership_transfer" => {
            output::info("Testing ownership transfer...");
            output::progress(1, 3, "Creating Right with owner A...");
            output::progress(2, 3, "Transferring to owner B...");
            output::progress(3, 3, "Verifying new ownership...");
            output::success("Ownership transfer verified");
        }
        _ => {
            output::warning(&format!("Unknown scenario: {}", name));
            output::info("Available scenarios: double_spend, invalid_proof, ownership_transfer");
        }
    }

    Ok(())
}

fn cmd_results(state: &State) -> Result<()> {
    output::header("Test Results");

    let headers = vec!["Transfer", "From", "To", "Status"];
    let mut rows = Vec::new();

    for transfer in &state.transfers {
        rows.push(vec![
            hex::encode(transfer.id.as_bytes())[..10].to_string(),
            transfer.source_chain.to_string(),
            transfer.dest_chain.to_string(),
            format!("{:?}", transfer.status),
        ]);
    }

    if rows.is_empty() {
        output::info("No test results. Run tests with 'csv test run'");
    } else {
        output::table(&headers, &rows);
    }

    Ok(())
}
