//! Wallet balance checking commands.
//!
//! Provides balance queries and wallet listing.

use crate::config::{Chain, Config};
use crate::output;
use crate::state::UnifiedStateManager;
use anyhow::Result;

/// Check balance for a specific chain.
pub fn cmd_balance(
    chain: Chain,
    address: Option<String>,
    _config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    let address = address.or_else(|| state.get_address(&chain));

    if let Some(addr) = address {
        output::header(&format!("{} Balance", chain));
        output::kv("Address", &addr);

        // Query balance from chain
        let balance = query_balance(&chain, &addr)?;
        output::kv("Balance", &format!("{} {}", balance, chain_symbol(&chain)));
    } else {
        output::warning(&format!("No {} address found in wallet", chain));
        output::info(&format!("Generate one with: csv wallet generate --chain {}", chain));
    }

    Ok(())
}

/// List all wallets.
pub fn cmd_list(_config: &Config, state: &mut UnifiedStateManager) -> Result<()> {
    output::header("Wallet Addresses");

    let chains = vec![
        Chain::Bitcoin,
        Chain::Ethereum,
        Chain::Sui,
        Chain::Aptos,
        Chain::Solana,
    ];

    let mut found_any = false;
    for chain in chains {
        if let Some(address) = state.get_address(&chain) {
            output::kv(&format!("{}", chain), &address);
            found_any = true;
        }
    }

    if !found_any {
        output::warning("No wallets found");
        output::info("Generate wallets with: csv wallet generate --chain <chain>");
        output::info("Or use one-command setup: csv wallet init");
    }

    Ok(())
}

/// Query balance from chain (placeholder implementation).
fn query_balance(chain: &Chain, address: &str) -> Result<f64> {
    // In a real implementation, this would query the chain's RPC
    // For now, return a placeholder
    output::info(&format!("Querying {} balance for {}...", chain, address));
    
    // Placeholder: would actually query the chain
    Ok(0.0)
}

/// Get symbol for chain.
fn chain_symbol(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "BTC",
        Chain::Ethereum => "ETH",
        Chain::Sui => "SUI",
        Chain::Aptos => "APT",
        Chain::Solana => "SOL",
        _ => "UNK",
    }
}
