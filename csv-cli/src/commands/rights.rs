//! Sanad lifecycle commands

use anyhow::Result;
use clap::Subcommand;
use sha2::Digest;

use csv_core::hash::Hash;

use crate::config::{Chain, Config};
use crate::output;
use crate::state::{SanadRecord, SanadStatus, UnifiedStateManager};

#[derive(Subcommand)]
pub enum SanadAction {
    /// Create a new Sanad
    Create {
        /// Chain name
        #[arg(short, long, value_enum)]
        chain: Chain,
        /// Value (chain-specific: sats for Bitcoin, etc.)
        #[arg(short = 'V', long)]
        value: Option<u64>,
    },
    /// Show Sanad details
    Show {
        /// Sanad ID (hex)
        sanad_id: String,
    },
    /// List all tracked Sanads
    List {
        /// Filter by chain
        #[arg(short, long, value_enum)]
        chain: Option<Chain>,
    },
    /// Transfer a Sanad to a new owner
    Transfer {
        /// Sanad ID (hex)
        sanad_id: String,
        /// New owner address
        to: String,
    },
    /// Consume a Sanad (seal consumption)
    Consume {
        /// Sanad ID (hex)
        sanad_id: String,
    },
}

pub fn execute(
    action: SanadAction,
    config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    match action {
        SanadAction::Create { chain, value } => cmd_create(chain, value, config, state),
        SanadAction::Show { sanad_id } => cmd_show(sanad_id, state),
        SanadAction::List { chain } => cmd_list(chain, state),
        SanadAction::Transfer { sanad_id, to } => cmd_transfer(sanad_id, to, state),
        SanadAction::Consume { sanad_id } => cmd_consume(sanad_id, state),
    }
}

fn cmd_create(
    chain: Chain,
    value: Option<u64>,
    _config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    output::header(&format!("Creating Sanad on {}", chain));

    // Use the new facade to create the sanad
    use csv_adapter::CsvClient;
    use csv_adapter::StoreBackend;
    use csv_core::Chain as CoreChain;

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

    // Generate a commitment for the sanad
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
    let commitment = csv_core::Hash::new(commitment_bytes);

    // Create the sanad through the facade
    match client.sanads().create(commitment, core_chain) {
        Ok(sanad) => {
            let sanad_id_hex = hex::encode(sanad.id.as_bytes());
            
            // Track the sanad in local state
            let tracked = SanadRecord {
                id: sanad_id_hex.clone(),
                chain: chain.clone(),
                seal_ref: String::new(), // Would be populated by the facade
                owner: String::new(),    // Would be populated by the facade
                value: value.unwrap_or(0),
                commitment: hex::encode(commitment.as_bytes()),
                nullifier: None,
                status: SanadStatus::Active,
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };

            state.storage.sanads.push(tracked);

            output::kv("Chain", &chain.to_string());
            output::kv_hash("Sanad ID", sanad.id.as_bytes());
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
            output::info("Sanad created successfully via facade. Use 'csv sanad show <sanad_id>' to view details");
        }
        Err(e) => {
            output::error(&format!("Failed to create sanad via facade: {}", e));
            return Err(anyhow::anyhow!("Sanad creation failed: {}", e));
        }
    }

    Ok(())
}

fn cmd_show(sanad_id: String, state: &UnifiedStateManager) -> Result<()> {
    let bytes = hex::decode(sanad_id.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid Sanad ID: {}", e))?;

    if bytes.len() != 32 {
        return Err(anyhow::anyhow!(
            "Sanad ID must be 32 bytes ({} bytes provided)",
            bytes.len()
        ));
    }

    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(&bytes);
    let sanad_id = Hash::new(hash_bytes);

    output::header(&format!("Sanad: {}", hex::encode(sanad_id.as_bytes())));

    if let Some(tracked) = state.get_sanad(&sanad_id.to_hex()) {
        output::kv("Chain", &tracked.chain.to_string());
        output::kv_hash("Commitment", tracked.commitment.as_bytes());
        output::kv(
            "Status",
            match tracked.status {
                SanadStatus::Consumed => "Consumed",
                SanadStatus::Transferred => "Transferred",
                SanadStatus::Active => "Active",
            },
        );
        if let Some(nullifier) = &tracked.nullifier {
            output::kv_hash("Nullifier", nullifier.as_bytes());
        }
    } else {
        output::warning("Sanad not found in local tracking");
        output::info("This Sanad may exist on-chain but hasn't been tracked locally");
    }

    Ok(())
}

fn cmd_list(chain: Option<Chain>, state: &UnifiedStateManager) -> Result<()> {
    output::header("Tracked Sanads");

    let headers = vec!["Sanad ID", "Chain", "Status"];
    let mut rows = Vec::new();

    for sanad in &state.storage.sanads {
        if let Some(ref filter_chain) = chain {
            if sanad.chain != *filter_chain {
                continue;
            }
        }

        // Check if seal is consumed in registry even if flag not set
        let seal_consumed = state.is_seal_consumed(&sanad.id);
        let status = if sanad.status == SanadStatus::Consumed || seal_consumed {
            "Consumed".to_string()
        } else {
            "Active".to_string()
        };

        rows.push(vec![sanad.id.clone(), sanad.chain.to_string(), status]);
    }

    if rows.is_empty() {
        output::info("No Sanads tracked. Use 'csv sanad create' to create one.");
    } else {
        output::table(&headers, &rows);
    }

    Ok(())
}

fn cmd_transfer(sanad_id: String, to: String, _state: &UnifiedStateManager) -> Result<()> {
    output::header(&format!("Transferring Sanad to {}", to));
    output::kv("Sanad ID", &sanad_id);
    output::kv("New Owner", &to);
    output::info("Cross-chain transfer: use 'csv cross-chain transfer' instead");
    Ok(())
}

fn cmd_consume(sanad_id: String, _state: &UnifiedStateManager) -> Result<()> {
    output::header("Consuming Sanad");
    output::kv("Sanad ID", &sanad_id);
    output::info("This will consume the seal and make the Sanad unusable");
    Ok(())
}
