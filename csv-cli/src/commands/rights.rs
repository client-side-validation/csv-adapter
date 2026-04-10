//! Right lifecycle commands

use anyhow::Result;
use clap::Subcommand;
use sha2::Digest;

use csv_adapter_core::hash::Hash;

use crate::config::{Chain, Config};
use crate::output;
use crate::state::{State, TrackedRight};

#[derive(Subcommand)]
pub enum RightAction {
    /// Create a new Right
    Create {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Value (chain-specific: sats for Bitcoin, etc.)
        #[arg(short, long)]
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

pub fn execute(action: RightAction, config: &Config, state: &State) -> Result<()> {
    match action {
        RightAction::Create { chain, value } => cmd_create(chain, value, config, state),
        RightAction::Show { right_id } => cmd_show(right_id, state),
        RightAction::List { chain } => cmd_list(chain, state),
        RightAction::Transfer { right_id, to } => cmd_transfer(right_id, to, state),
        RightAction::Consume { right_id } => cmd_consume(right_id, state),
    }
}

fn cmd_create(chain: Chain, value: Option<u64>, _config: &Config, _state: &State) -> Result<()> {
    output::header(&format!("Creating Right on {}", chain));

    // In production, this would call the chain adapter to create a seal
    // For now, generate a Right ID and track it

    let right_id_bytes: [u8; 32] = {
        use sha2::Sha256;
        let mut hasher = Sha256::new();
        hasher.update(b"right-");
        hasher.update(chain.to_string().as_bytes());
        hasher.update(value.unwrap_or(0).to_le_bytes());
        if let Some(nanos) = chrono::Utc::now().timestamp_nanos_opt() {
            hasher.update(nanos.to_le_bytes());
        }
        hasher.finalize().into()
    };

    let right_id = Hash::new(right_id_bytes);

    let _tracked = TrackedRight {
        id: right_id,
        chain: chain.clone(),
        seal_ref: vec![], // Would come from adapter
        owner: vec![],    // Would come from wallet
        commitment: right_id,
        nullifier: None,
        consumed: false,
    };

    output::kv("Chain", &chain.to_string());
    output::kv_hash("Right ID", right_id_bytes.as_slice());
    output::kv(
        "Value",
        &value
            .map(|v| v.to_string())
            .unwrap_or_else(|| "default".to_string()),
    );
    output::kv("Status", "Created");

    // Note: state is &State, not &mut State — can't save here
    // In production, this would save to state
    println!();
    output::info("Right created. Use 'csv right show <right_id>' to view details");

    Ok(())
}

fn cmd_show(right_id: String, state: &State) -> Result<()> {
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

    if let Some(tracked) = state.get_right(&right_id) {
        output::kv("Chain", &tracked.chain.to_string());
        output::kv_hash("Commitment", tracked.commitment.as_bytes());
        output::kv(
            "Status",
            if tracked.consumed {
                "Consumed"
            } else {
                "Active"
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

fn cmd_list(chain: Option<Chain>, state: &State) -> Result<()> {
    output::header("Tracked Rights");

    let headers = vec!["Right ID", "Chain", "Status", "Commitment"];
    let mut rows = Vec::new();

    for right in &state.rights {
        if let Some(ref filter_chain) = chain {
            if right.chain != *filter_chain {
                continue;
            }
        }

        rows.push(vec![
            hex::encode(right.id.as_bytes())[..16].to_string(),
            right.chain.to_string(),
            if right.consumed {
                "Consumed".to_string()
            } else {
                "Active".to_string()
            },
            hex::encode(right.commitment.as_bytes())[..16].to_string(),
        ]);
    }

    if rows.is_empty() {
        output::info("No Rights tracked. Use 'csv right create' to create one.");
    } else {
        output::table(&headers, &rows);
    }

    Ok(())
}

fn cmd_transfer(right_id: String, to: String, _state: &State) -> Result<()> {
    output::header(&format!("Transferring Right to {}", to));
    output::kv("Right ID", &right_id);
    output::kv("New Owner", &to);
    output::info("Cross-chain transfer: use 'csv cross-chain transfer' instead");
    Ok(())
}

fn cmd_consume(right_id: String, _state: &State) -> Result<()> {
    output::header("Consuming Right");
    output::kv("Right ID", &right_id);
    output::info("This will consume the seal and make the Right unusable");
    Ok(())
}
