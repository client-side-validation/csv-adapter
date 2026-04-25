//! Seal management commands

use anyhow::Result;
use clap::Subcommand;

use crate::config::{Chain, Config};
use crate::output;
use crate::state::{UnifiedStateManager, SealRecord};

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

pub fn execute(action: SealAction, config: &Config, state: &mut UnifiedStateManager) -> Result<()> {
    match action {
        SealAction::Create { chain, value } => cmd_create(chain, value, config, state),
        SealAction::Consume { chain, seal_ref } => cmd_consume(chain, seal_ref, config, state),
        SealAction::Verify { chain, seal_ref } => cmd_verify(chain, seal_ref, config, state),
        SealAction::List { chain } => cmd_list(chain, state),
    }
}

fn cmd_create(chain: Chain, value: Option<u64>, _config: &Config, state: &mut UnifiedStateManager) -> Result<()> {
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
        Chain::Solana => {
            // Program-derived address seal
            vec![0x05; 32] // placeholder
        }
    };

    state.record_seal_consumption(hex::encode(&seal_bytes));

    output::kv("Chain", &chain.to_string());
    output::kv_hash("Seal", &seal_bytes);
    output::kv(
        "Value",
        &value
            .map(|v| v.to_string())
            .unwrap_or_else(|| "default".to_string()),
    );
    output::kv("Status", "Created");

    Ok(())
}

fn cmd_consume(chain: Chain, seal_ref: String, _config: &Config, state: &mut UnifiedStateManager) -> Result<()> {
    output::header(&format!("Consuming Seal on {}", chain));

    let seal_bytes = hex::decode(seal_ref.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid seal reference: {}", e))?;

    if state.is_seal_consumed(&hex::encode(&seal_bytes)) {
        output::error("Seal already consumed");
        return Err(anyhow::anyhow!("Seal replay detected"));
    }

    state.record_seal_consumption(hex::encode(&seal_bytes));

    output::kv("Chain", &chain.to_string());
    output::kv_hash("Seal", &seal_bytes);
    output::success("Seal consumed");

    Ok(())
}

fn cmd_verify(chain: Chain, seal_ref: String, _config: &Config, state: &UnifiedStateManager) -> Result<()> {
    output::header(&format!("Verifying Seal on {}", chain));

    let seal_bytes = hex::decode(seal_ref.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid seal reference: {}", e))?;

    let consumed = state.is_seal_consumed(&hex::encode(&seal_bytes));

    output::kv("Chain", &chain.to_string());
    output::kv_hash("Seal", &seal_bytes);
    output::kv("Status", if consumed { "Consumed" } else { "Unconsumed" });

    if !consumed {
        output::info("Seal is available for use");
    }

    Ok(())
}

fn cmd_list(chain: Option<Chain>, state: &UnifiedStateManager) -> Result<()> {
    output::header("Consumed Seals");

    let consumed_seals: Vec<&SealRecord> = state.storage.seals.iter().filter(|s| s.consumed).collect();

    if consumed_seals.is_empty() {
        output::info("No seals consumed");
    } else {
        let headers = vec!["#", "Seal (hex)", "Chain", "Consumed"];
        let mut rows = Vec::new();

        for (i, seal) in consumed_seals.iter().enumerate() {
            // Filter by chain if specified
            if let Some(ref filter_chain) = chain {
                if &seal.chain != filter_chain {
                    continue;
                }
            }

            rows.push(vec![
                (i + 1).to_string(),
                seal.seal_ref[..16.min(seal.seal_ref.len())].to_string(),
                seal.chain.to_string(),
                "Yes".to_string(),
            ]);
        }

        output::table(&headers, &rows);
    }

    Ok(())
}
