//! Wallet balance checking commands (Phase 5 Compliant).
//!
//! Uses csv-adapter facade APIs only - no direct chain adapter dependencies.

use crate::config::{Chain, Config};
use crate::output;
use crate::state::UnifiedStateManager;
use anyhow::Result;

use csv_adapter::CsvClient;
use csv_adapter::StoreBackend;

/// Check balance for a specific chain.
pub async fn cmd_balance(
    chain: Chain,
    address: Option<String>,
    config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    let address = address.or_else(|| state.get_address(&chain).map(|s| s.to_string()));

    if let Some(addr) = address {
        output::header(&format!("{} Balance", chain));
        output::kv("Address", &addr);

        // Query balance from chain using csv-adapter facade
        match query_balance(&chain, &addr, config).await {
            Ok(balance) => {
                output::kv("Balance", &format!("{} {}", balance, chain_symbol(&chain)));
            }
            Err(e) => {
                output::error(&format!("Failed to query balance: {}", e));
                output::info("Balance query requires chain RPC to be configured");
            }
        }
    } else {
        output::warning(&format!("No {} address found in wallet", chain));
        output::info(&format!(
            "Generate one with: csv wallet generate --chain {}",
            chain
        ));
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

/// Query balance from chain using csv-adapter facade APIs.
///
/// This function uses only the unified CsvClient facade, avoiding direct
/// chain adapter dependencies per Phase 5 of the Production Guarantee Plan.
async fn query_balance(chain: &Chain, address: &str, config: &Config) -> Result<f64> {
    use csv_adapter::prelude::NetworkType;
    use csv_adapter_core::Chain as CoreChain;

    // Map CLI Chain to core Chain
    let core_chain = match chain {
        Chain::Bitcoin => CoreChain::Bitcoin,
        Chain::Ethereum => CoreChain::Ethereum,
        Chain::Solana => CoreChain::Solana,
        Chain::Sui => CoreChain::Sui,
        Chain::Aptos => CoreChain::Aptos,
    };

    // Build CSV client with the requested chain enabled
    let client = CsvClient::builder()
        .with_chain(core_chain)
        .with_store_backend(StoreBackend::InMemory)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build CSV client: {}", e))?;

    // Get chain facade and query balance through the unified facade
    let clean_address = address.strip_prefix("0x").unwrap_or(address);
    
    // Initialize adapters with the correct network (testnet by default for CLI)
    let network = if config.network().is_testnet() {
        NetworkType::Testnet
    } else {
        NetworkType::Mainnet
    };

    // Execute async operations using the existing tokio runtime
    let balance_info = async {
        client.init_adapters(network).await.map_err(|e| {
            csv_adapter::CsvError::AdapterError {
                chain: core_chain,
                message: format!("Failed to initialize adapters: {}", e),
            }
        })?;
        client.chain_facade().get_balance(core_chain, clean_address).await
    }.await;

    match balance_info {
        Ok(balance_info) => Ok(balance_info.available as f64 / 1e8), // Convert from satoshis to BTC for Bitcoin, adjust for other chains as needed
        Err(e) => {
            // Check if it's a configuration error
            if matches!(e, csv_adapter::CsvError::ChainNotEnabled(_)) {
                Err(anyhow::anyhow!(
                    "Balance query via facade requires RPC configuration. \
                     Please configure the appropriate RPC_URL environment variable for {:?}.",
                    chain
                ))
            } else {
                Err(anyhow::anyhow!("Failed to query balance: {}", e))
            }
        }
    }
}

/// Get symbol for chain.
fn chain_symbol(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "BTC",
        Chain::Ethereum => "ETH",
        Chain::Sui => "SUI",
        Chain::Aptos => "APT",
        Chain::Solana => "SOL",
    }
}
