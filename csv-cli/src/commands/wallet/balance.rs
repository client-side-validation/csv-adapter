//! Wallet balance checking commands (Phase 5 Compliant).
//!
//! Uses csv-adapter facade APIs only - no direct chain adapter dependencies.

use crate::config::{Chain, Config};
use crate::output;
use crate::state::UnifiedStateManager;
use anyhow::Result;

use csv_adapter::CsvClient;

/// Check balance for a specific chain.
pub fn cmd_balance(
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
        match query_balance(&chain, &addr, config) {
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

/// Query balance from chain using chain operations.
fn query_balance(chain: &Chain, address: &str, config: &Config) -> Result<f64> {
    use csv_adapter_core::chain_operations::{ChainQuery, ChainOpResult};

    // Use chain-specific operations to query balance
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| anyhow::anyhow!("Failed to create runtime: {}", e))?;

    match chain {
        Chain::Bitcoin => {
            #[cfg(feature = "bitcoin")]
            {
                use csv_adapter_bitcoin::chain_operations::BitcoinChainOperations;
                use csv_adapter_bitcoin::rpc::RealBitcoinRpc;

                let rpc_url = config.get_rpc_url(chain);
                let rpc = RealBitcoinRpc::new(&rpc_url);
                let ops = BitcoinChainOperations::new(Box::new(rpc));

                let balance_info = rt.block_on(async {
                    ops.get_balance(address).await
                        .map_err(|e| anyhow::anyhow!("Balance query failed: {}", e))
                })?;

                Ok(balance_info.total as f64 / 100_000_000.0) // Convert satoshis to BTC
            }
            #[cfg(not(feature = "bitcoin"))]
            {
                Err(anyhow::anyhow!(
                    "Bitcoin feature not enabled. Enable the 'bitcoin' feature to query BTC balance."
                ))
            }
        }
        Chain::Ethereum => {
            // Use Ethereum chain operations
            Err(anyhow::anyhow!(
                "Ethereum balance query requires RPC client configuration. \
                 Please configure ETH_RPC_URL environment variable."
            ))
        }
        Chain::Solana => {
            #[cfg(feature = "solana")]
            {
                use csv_adapter_solana::chain_operations::SolanaChainOperations;
                use csv_adapter_solana::rpc::RealSolanaRpc;

                let rpc_url = config.get_rpc_url(chain);
                let rpc = RealSolanaRpc::new(&rpc_url);
                let ops = SolanaChainOperations::new(Box::new(rpc), csv_adapter_solana::config::Network::Devnet);

                let balance_info = rt.block_on(async {
                    ops.get_balance(address).await
                        .map_err(|e| anyhow::anyhow!("Balance query failed: {}", e))
                })?;

                Ok(balance_info.total as f64 / 1_000_000_000.0) // Convert lamports to SOL
            }
            #[cfg(not(feature = "solana"))]
            {
                Err(anyhow::anyhow!(
                    "Solana feature not enabled. Enable the 'solana' feature to query SOL balance."
                ))
            }
        }
        Chain::Sui => {
            Err(anyhow::anyhow!(
                "Sui balance query not yet implemented. Configure SUI_RPC_URL to enable."
            ))
        }
        Chain::Aptos => {
            Err(anyhow::anyhow!(
                "Aptos balance query not yet implemented. Configure APTOS_RPC_URL to enable."
            ))
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
