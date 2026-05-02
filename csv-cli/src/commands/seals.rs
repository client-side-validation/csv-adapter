//! Seal management commands (Phase 5 Compliant)
//!
//! Uses csv-adapter facade APIs only - no direct chain adapter dependencies.

use anyhow::Result;
use clap::Subcommand;

use csv_adapter::CsvClient;
use csv_adapter_core::Chain;

use crate::config::{Chain as ConfigChain, Config};
use crate::output;
use crate::state::{SealRecord, UnifiedStateManager};
use crate::commands::cross_chain::to_core_chain;

#[derive(Subcommand)]
pub enum SealAction {
    /// Create a new seal on a chain
    Create {
        /// Chain name
        #[arg(value_enum)]
        chain: ConfigChain,
        /// Value (chain-specific)
        #[arg(short, long)]
        value: Option<u64>,
    },
    /// Consume a seal
    Consume {
        /// Chain name
        #[arg(value_enum)]
        chain: ConfigChain,
        /// Seal reference (chain-specific format)
        seal_ref: String,
    },
    /// Verify seal status
    Verify {
        /// Chain name
        #[arg(value_enum)]
        chain: ConfigChain,
        /// Seal reference
        seal_ref: String,
    },
    /// List consumed seals
    List {
        /// Filter by chain
        #[arg(short, long, value_enum)]
        chain: Option<ConfigChain>,
    },
}

pub fn execute(action: SealAction, _config: &Config, state: &mut UnifiedStateManager) -> Result<()> {
    match action {
        SealAction::Create { chain, value } => cmd_create(chain, value, state),
        SealAction::Consume { chain, seal_ref } => cmd_consume(chain, seal_ref, state),
        SealAction::Verify { chain, seal_ref } => cmd_verify(chain, seal_ref, state),
        SealAction::List { chain } => cmd_list(chain, state),
    }
}

fn cmd_create(
    chain: ConfigChain,
    value: Option<u64>,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    output::header(&format!("Creating Seal on {}", chain));

    let core_chain = to_core_chain(chain.clone());

    // Phase 5: Use facade client to create seal via RightsManager
    let client = CsvClient::builder()
        .with_chain(core_chain)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create CSV client: {}", e))?;

    // Create a seal by creating a basic Right (which creates a seal)
    let rights = client.rights();

    // Generate a commitment for seal creation
    let commitment = csv_adapter_core::Hash::new(generate_commitment());

    // Create the right (which internally creates a seal via the facade)
    match rights.create(commitment, core_chain) {
        Ok(right) => {
            let seal_id = hex::encode(right.id.as_bytes());
            let value_sat = value.unwrap_or(100_000);

            output::kv("Chain", &chain.to_string());
            output::kv("Seal ID", &seal_id[..16.min(seal_id.len())]);
            output::kv("Value", &format!("{} satoshis", value_sat));
            output::kv("Right ID", &hex::encode(right.id.as_bytes())[..16]);
            output::success("Seal created successfully via facade");

            // Record in state
            state.storage.seals.push(SealRecord {
                seal_ref: seal_id,
                chain,
                value: value_sat,
                consumed: false,
                created_at: chrono::Utc::now().timestamp() as u64,
            });
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to create seal via facade: {}", e));
        }
    }

    Ok(())
}

/// Generate a random 32-byte commitment for seal creation.
fn generate_commitment() -> [u8; 32] {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

fn cmd_consume(
    chain: ConfigChain,
    seal_ref: String,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    output::header(&format!("Consuming Seal on {}", chain));

    let seal_bytes = hex::decode(seal_ref.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid seal reference: {}", e))?;
    let seal_hex = hex::encode(&seal_bytes);

    if state.is_seal_consumed(&seal_hex) {
        output::error("Seal already consumed");
        return Err(anyhow::anyhow!("Seal replay detected"));
    }

    let core_chain = to_core_chain(chain.clone());

    // Use facade to consume the seal
    let client = CsvClient::builder()
        .with_chain(core_chain)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create CSV client: {}", e))?;

    // Find the right associated with this seal and burn it (consuming the seal)
    let rights = client.rights();

    // Create a RightId from the seal reference
    let right_id_bytes: [u8; 32] = seal_bytes[..32.min(seal_bytes.len())]
        .try_into()
        .unwrap_or_else(|_| {
            let mut padded = [0u8; 32];
            padded[..seal_bytes.len().min(32)].copy_from_slice(&seal_bytes[..seal_bytes.len().min(32)]);
            padded
        });
    let right_id = csv_adapter_core::RightId::new(right_id_bytes);

    // Burn the right, which consumes the seal
    match rights.burn(&right_id) {
        Ok(()) => {
            state.record_seal_consumption(seal_hex.clone());

            output::kv("Chain", &chain.to_string());
            output::kv_hash("Seal", &seal_bytes);
            output::success("Seal consumed via facade");
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to consume seal via facade: {}", e));
        }
    }

    Ok(())
}

fn cmd_verify(
    chain: ConfigChain,
    seal_ref: String,
    state: &UnifiedStateManager,
) -> Result<()> {
    output::header(&format!("Verifying Seal on {}", chain));

    let seal_bytes = hex::decode(seal_ref.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid seal reference: {}", e))?;

    let core_chain = to_core_chain(chain.clone());

    // Use facade to verify seal status
    let client = CsvClient::builder()
        .with_chain(core_chain)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create CSV client: {}", e))?;

    // Query the right status via the rights manager
    let rights = client.rights();

    // Create a RightId from the seal reference
    let right_id_bytes: [u8; 32] = seal_bytes[..32.min(seal_bytes.len())]
        .try_into()
        .unwrap_or_else(|_| {
            let mut padded = [0u8; 32];
            padded[..seal_bytes.len().min(32)].copy_from_slice(&seal_bytes[..seal_bytes.len().min(32)]);
            padded
        });
    let right_id = csv_adapter_core::RightId::new(right_id_bytes);

    // Check if the right exists and get its status
    let local_consumed = state.is_seal_consumed(&hex::encode(&seal_bytes));

    match rights.get(&right_id) {
        Ok(Some(right)) => {
            // Right exists in the system
            let status = if local_consumed || right.nullifier.is_some() { "Consumed" } else { "Unconsumed" };
            output::kv("Chain", &chain.to_string());
            output::kv_hash("Seal", &seal_bytes);
            output::kv("Status", status);
            output::kv("Right ID", &hex::encode(right.id.as_bytes())[..16]);

            if !local_consumed && right.nullifier.is_none() {
                output::info("Seal is available for use");
            }
        }
        Ok(None) => {
            // Right not found in the system
            output::kv("Chain", &chain.to_string());
            output::kv_hash("Seal", &seal_bytes);
            output::kv("Status", if local_consumed { "Consumed" } else { "Unknown" });

            if local_consumed {
                output::info("Seal was consumed locally but not found in facade");
            } else {
                output::warning("Seal not found in the system");
            }
        }
        Err(e) => {
            // Query failed, fall back to local state
            output::warning(&format!("Facade query failed: {}", e));
            output::kv("Chain", &chain.to_string());
            output::kv_hash("Seal", &seal_bytes);
            output::kv("Status", if local_consumed { "Consumed" } else { "Unconsumed" });
        }
    }

    Ok(())
}

fn cmd_list(chain: Option<ConfigChain>, state: &UnifiedStateManager) -> Result<()> {
    output::header("Consumed Seals");

    let consumed_seals: Vec<&SealRecord> =
        state.storage.seals.iter().filter(|s| s.consumed).collect();

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
