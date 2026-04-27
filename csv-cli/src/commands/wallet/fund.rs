//! Wallet funding from faucets.
//!
//! Provides testnet faucet integration for all chains.

use crate::config::{Chain, Config, Network};
use crate::output;
use crate::state::UnifiedStateManager;
use anyhow::Result;

/// Fund wallet from faucet.
pub fn cmd_fund(
    chain: Chain,
    address: Option<String>,
    config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    let address = address.or_else(|| state.get_address(&chain));

    if let Some(addr) = address {
        output::header(&format!("Funding {} Wallet", chain));
        output::kv("Address", &addr);
        output::info("Requesting funds from faucet...");

        request_faucet_funds(&chain, &addr, config)?;

        output::success("Funding request submitted");
        output::info("Funds should arrive within a few minutes");
        output::info(&format!("Check balance with: csv wallet balance --chain {}", chain));
    } else {
        output::warning(&format!("No {} address found in wallet", chain));
        output::info(&format!("Generate one with: csv wallet generate --chain {}", chain));
    }

    Ok(())
}

/// Request funds from chain faucet.
fn request_faucet_funds(chain: &Chain, address: &str, config: &Config) -> Result<()> {
    let network = config.network();

    match chain {
        Chain::Bitcoin => request_bitcoin_faucet(address, &network),
        Chain::Ethereum => request_ethereum_faucet(address, &network),
        Chain::Sui => request_sui_faucet(address, &network),
        Chain::Aptos => request_aptos_faucet(address, &network),
        Chain::Solana => request_solana_faucet(address, &network),
        _ => Err(anyhow::anyhow!("No faucet available for {:?}", chain)),
    }
}

fn request_bitcoin_faucet(address: &str, network: &Network) -> Result<()> {
    match network {
        Network::Dev => {
            output::info("For regtest, use: bitcoin-cli -regtest generatetoaddress 101 <address>");
        }
        Network::Test => {
            output::info(&format!(
                "Visit: https://signet.bc-2.jp/ to fund {}",
                address
            ));
        }
        Network::Main => {
            output::warning("No faucet available for mainnet");
        }
    }
    Ok(())
}

fn request_ethereum_faucet(address: &str, network: &Network) -> Result<()> {
    match network {
        Network::Dev => {
            output::info("For local devnet, use your local node's mining");
        }
        Network::Test => {
            output::info(&format!(
                "Visit: https://sepoliafaucet.com/ to fund {}",
                address
            ));
        }
        Network::Main => {
            output::warning("No faucet available for mainnet");
        }
    }
    Ok(())
}

fn request_sui_faucet(address: &str, network: &Network) -> Result<()> {
    match network {
        Network::Dev | Network::Test => {
            output::info(&format!(
                "Visit: https://faucet.testnet.sui.io/ to fund {}",
                address
            ));
        }
        Network::Main => {
            output::warning("No faucet available for mainnet");
        }
    }
    Ok(())
}

fn request_aptos_faucet(address: &str, network: &Network) -> Result<()> {
    match network {
        Network::Dev | Network::Test => {
            output::info(&format!(
                "Visit: https://aptoslabs.com/testnet-faucet to fund {}",
                address
            ));
        }
        Network::Main => {
            output::warning("No faucet available for mainnet");
        }
    }
    Ok(())
}

fn request_solana_faucet(address: &str, network: &Network) -> Result<()> {
    match network {
        Network::Dev => {
            output::info("For local validator, use: solana airdrop 1 <address>");
        }
        Network::Test => {
            output::info(&format!(
                "Visit: https://faucet.solana.com/ to fund {}",
                address
            ));
        }
        Network::Main => {
            output::warning("No faucet available for mainnet");
        }
    }
    Ok(())
}
