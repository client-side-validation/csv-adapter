//! Seal management commands

use anyhow::Result;
use clap::Subcommand;


use crate::config::{Config, Chain};
use crate::state::State;
use crate::output;

#[derive(Subcommand)]
pub enum SealAction {
    /// Create a new seal on a chain
    Create {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Value (chain-specific)
        #[arg(short, long)]
        value: Option<u64>,
    },
    /// Consume a seal
    Consume {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Seal reference (chain-specific format)
        seal_ref: String,
    },
    /// Verify seal status
    Verify {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Seal reference
        seal_ref: String,
    },
    /// List consumed seals
    List {
        /// Filter by chain
        #[arg(short, long, value_enum)]
        chain: Option<Chain>,
    },
}

pub fn execute(action: SealAction, config: &Config, state: &mut State) -> Result<()> {
    match action {
        SealAction::Create { chain, value } => cmd_create(chain, value, config, state),
        SealAction::Consume { chain, seal_ref } => cmd_consume(chain, seal_ref, config, state),
        SealAction::Verify { chain, seal_ref } => cmd_verify(chain, seal_ref, config, state),
        SealAction::List { chain } => cmd_list(chain, state),
    }
}

fn cmd_create(chain: Chain, value: Option<u64>, _config: &Config, state: &mut State) -> Result<()> {
    output::header(&format!("Creating Seal on {}", chain));

    let seal_bytes: Vec<u8> = match chain {
        Chain::Bitcoin => {
            // UTXO seal: txid + vout
            vec![0x01; 36] // placeholder
        }
        Chain::Ethereum => {
            // Nullifier seal: contract address + slot
            vec![0x02; 52] // placeholder
        }
        Chain::Sui => {
            // Object seal: object ID + version
            vec![0x03; 40] // placeholder
        }
        Chain::Aptos => {
            // Resource seal: account address
            vec![0x04; 32] // placeholder
        }
    };

    state.record_seal_consumption(seal_bytes.clone());

    output::kv("Chain", &chain.to_string());
    output::kv_hash("Seal", &seal_bytes);
    output::kv("Value", &value.map(|v| v.to_string()).unwrap_or_else(|| "default".to_string()));
    output::kv("Status", "Created");

    Ok(())
}

fn cmd_consume(chain: Chain, seal_ref: String, _config: &Config, state: &mut State) -> Result<()> {
    output::header(&format!("Consuming Seal on {}", chain));

    let seal_bytes = hex::decode(seal_ref.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid seal reference: {}", e))?;

    if state.is_seal_consumed(&seal_bytes) {
        output::error("Seal already consumed");
        return Err(anyhow::anyhow!("Seal replay detected"));
    }

    state.record_seal_consumption(seal_bytes.clone());

    output::kv("Chain", &chain.to_string());
    output::kv_hash("Seal", &seal_bytes);
    output::success("Seal consumed");

    Ok(())
}

fn cmd_verify(chain: Chain, seal_ref: String, _config: &Config, state: &State) -> Result<()> {
    output::header(&format!("Verifying Seal on {}", chain));

    let seal_bytes = hex::decode(seal_ref.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid seal reference: {}", e))?;

    let consumed = state.is_seal_consumed(&seal_bytes);

    output::kv("Chain", &chain.to_string());
    output::kv_hash("Seal", &seal_bytes);
    output::kv("Status", if consumed { "Consumed" } else { "Unconsumed" });

    if !consumed {
        output::info("Seal is available for use");
    }

    Ok(())
}

fn cmd_list(chain: Option<Chain>, state: &State) -> Result<()> {
    output::header("Consumed Seals");

    if state.consumed_seals.is_empty() {
        output::info("No seals consumed");
    } else {
        let headers = vec!["#", "Seal (hex)", "Chain", "Consumed"];
        let mut rows = Vec::new();

        for (i, seal) in state.consumed_seals.iter().enumerate() {
            // Determine chain from seal prefix
            let chain_str = if seal.first() == Some(&0x01) {
                "Bitcoin"
            } else if seal.first() == Some(&0x02) {
                "Ethereum"
            } else if seal.first() == Some(&0x03) {
                "Sui"
            } else if seal.first() == Some(&0x04) {
                "Aptos"
            } else {
                "Unknown"
            };

            if let Some(ref filter_chain) = chain {
                let filter_prefix = match filter_chain {
                    Chain::Bitcoin => 0x01,
                    Chain::Ethereum => 0x02,
                    Chain::Sui => 0x03,
                    Chain::Aptos => 0x04,
                };
                if seal.first() != Some(&filter_prefix) {
                    continue;
                }
            }

            rows.push(vec![
                (i + 1).to_string(),
                hex::encode(seal)[..16].to_string(),
                chain_str.to_string(),
                "Yes".to_string(),
            ]);
        }

        output::table(&headers, &rows);
    }

    Ok(())
}
