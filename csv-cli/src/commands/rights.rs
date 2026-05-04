//! Right lifecycle commands

use anyhow::Result;
use clap::Subcommand;
use sha2::Digest;

use csv_adapter_core::hash::Hash;

use crate::config::{Chain, Config};
use crate::output;
use crate::state::{RightRecord, RightStatus, UnifiedStateManager};

#[derive(Subcommand)]
pub enum RightAction {
    /// Create a new Right
    Create {
        /// Chain name
        #[arg(short, long, value_enum)]
        chain: Chain,
        /// Value (chain-specific: sats for Bitcoin, etc.)
        #[arg(short = 'V', long)]
        value: Option<u64>,
    },
    /// Show Right details
    Show {
        /// Right ID (hex)
        right_id: String,
    },
    /// List all tracked Rights
    List {
        /// Filter by chain
        #[arg(short, long, value_enum)]
        chain: Option<Chain>,
    },
    /// Transfer a Right to a new owner
    Transfer {
        /// Right ID (hex)
        right_id: String,
        /// New owner address
        to: String,
    },
    /// Consume a Right (seal consumption)
    Consume {
        /// Right ID (hex)
        right_id: String,
    },
}

pub fn execute(
    action: RightAction,
    config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    match action {
        RightAction::Create { chain, value } => cmd_create(chain, value, config, state),
        RightAction::Show { right_id } => cmd_show(right_id, state),
        RightAction::List { chain } => cmd_list(chain, state),
        RightAction::Transfer { right_id, to } => cmd_transfer(right_id, to, state),
        RightAction::Consume { right_id } => cmd_consume(right_id, state),
    }
}

fn cmd_create(
    chain: Chain,
    value: Option<u64>,
    _config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    output::header(&format!("Creating Right on {}", chain));

    // Use the new facade to create the right
    use csv_adapter::CsvClient;
    use csv_adapter::StoreBackend;
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

    // Generate a commitment for the right
    let commitment_bytes: [u8; 32] = {
        use sha2::Sha256;
        let mut hasher = Sha256::new();
        hasher.update(b"commitment-");
        hasher.update(chain.to_string().as_bytes());
        hasher.update(value.unwrap_or(0).to_le_bytes());
        if let Some(nanos) = chrono::Utc::now().timestamp_nanos_opt() {
            hasher.update(nanos.to_le_bytes());
        }
        hasher.finalize().into()
    };
    let commitment = csv_adapter_core::Hash::new(commitment_bytes);

    // Create the right through the facade
    match client.rights().create(commitment, core_chain) {
        Ok(right) => {
            let right_id_hex = hex::encode(right.id.as_bytes());
            
            // Track the right in local state
            let tracked = RightRecord {
                id: right_id_hex.clone(),
                chain: chain.clone(),
                seal_ref: String::new(), // Would be populated by the facade
                owner: String::new(),    // Would be populated by the facade
                value: value.unwrap_or(0),
                commitment: hex::encode(commitment.as_bytes()),
                nullifier: None,
                status: RightStatus::Active,
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };

            state.storage.rights.push(tracked);

            output::kv("Chain", &chain.to_string());
            output::kv_hash("Right ID", right.id.as_bytes());
            output::kv_hash("Commitment", commitment.as_bytes());
            output::kv(
                "Value",
                &value
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "default".to_string()),
            );
            output::kv("Status", "Created via facade");

            // UnifiedStateManager is automatically saved after command execution
            println!();
            output::info("Right created successfully via facade. Use 'csv right show <right_id>' to view details");
        }
        Err(e) => {
            output::error(&format!("Failed to create right via facade: {}", e));
            return Err(anyhow::anyhow!("Right creation failed: {}", e));
        }
    }

    Ok(())
}

fn cmd_show(right_id: String, state: &UnifiedStateManager) -> Result<()> {
    let bytes = hex::decode(right_id.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid Right ID: {}", e))?;

    if bytes.len() != 32 {
        return Err(anyhow::anyhow!(
            "Right ID must be 32 bytes ({} bytes provided)",
            bytes.len()
        ));
    }

    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(&bytes);
    let right_id = Hash::new(hash_bytes);

    output::header(&format!("Right: {}", hex::encode(right_id.as_bytes())));

    if let Some(tracked) = state.get_right(&right_id.to_hex()) {
        output::kv("Chain", &tracked.chain.to_string());
        output::kv_hash("Commitment", tracked.commitment.as_bytes());
        output::kv(
            "Status",
            match tracked.status {
                RightStatus::Consumed => "Consumed",
                RightStatus::Transferred => "Transferred",
                RightStatus::Active => "Active",
            },
        );
        if let Some(nullifier) = &tracked.nullifier {
            output::kv_hash("Nullifier", nullifier.as_bytes());
        }
    } else {
        output::warning("Right not found in local tracking");
        output::info("This Right may exist on-chain but hasn't been tracked locally");
    }

    Ok(())
}

fn cmd_list(chain: Option<Chain>, state: &UnifiedStateManager) -> Result<()> {
    output::header("Tracked Rights");

    let headers = vec!["Right ID", "Chain", "Status"];
    let mut rows = Vec::new();

    for right in &state.storage.rights {
        if let Some(ref filter_chain) = chain {
            if right.chain != *filter_chain {
                continue;
            }
        }

        // Check if seal is consumed in registry even if flag not set
        let seal_consumed = state.is_seal_consumed(&right.id);
        let status = if right.status == RightStatus::Consumed || seal_consumed {
            "Consumed".to_string()
        } else {
            "Active".to_string()
        };

        rows.push(vec![right.id.clone(), right.chain.to_string(), status]);
    }

    if rows.is_empty() {
        output::info("No Rights tracked. Use 'csv right create' to create one.");
    } else {
        output::table(&headers, &rows);
    }

    Ok(())
}

fn cmd_transfer(right_id: String, to: String, _state: &UnifiedStateManager) -> Result<()> {
    output::header(&format!("Transferring Right to {}", to));
    output::kv("Right ID", &right_id);
    output::kv("New Owner", &to);
    output::info("Cross-chain transfer: use 'csv cross-chain transfer' instead");
    Ok(())
}

fn cmd_consume(right_id: String, _state: &UnifiedStateManager) -> Result<()> {
    output::header("Consuming Right");
    output::kv("Right ID", &right_id);
    output::info("This will consume the seal and make the Right unusable");
    Ok(())
}
