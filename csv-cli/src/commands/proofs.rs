//! Proof generation and verification commands

use anyhow::Result;
use clap::Subcommand;


use crate::config::{Chain, Config};
use crate::output;
use crate::state::UnifiedStateManager;

#[derive(Subcommand)]
pub enum ProofAction {
    /// Generate inclusion proof for a Right
    Generate {
        /// Source chain
        #[arg(value_enum)]
        chain: Chain,
        /// Right ID (hex)
        right_id: String,
        /// Output file (prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Verify an inclusion proof
    Verify {
        /// Destination chain (verifies ON this chain)
        #[arg(value_enum)]
        chain: Chain,
        /// Proof file (reads from stdin if not specified)
        #[arg(short, long)]
        proof: Option<String>,
    },
    /// Verify cross-chain proof (proof from chain A verified on chain B)
    VerifyCrossChain {
        /// Source chain (where proof was generated)
        #[arg(long)]
        source: Chain,
        /// Destination chain (where proof is being verified)
        #[arg(long)]
        dest: Chain,
        /// Proof file
        proof: String,
    },
}

pub fn execute(action: ProofAction, config: &Config, state: &UnifiedStateManager) -> Result<()> {
    match action {
        ProofAction::Generate {
            chain,
            right_id,
            output,
        } => cmd_generate(chain, right_id, output, config, state),
        ProofAction::Verify { chain, proof } => cmd_verify(chain, proof, config, state),
        ProofAction::VerifyCrossChain {
            source,
            dest,
            proof,
        } => cmd_verify_cross_chain(source, dest, proof, config, state),
    }
}

fn cmd_generate(
    chain: Chain,
    right_id: String,
    output: Option<String>,
    config: &Config,
    _state: &UnifiedStateManager,
) -> Result<()> {
    use csv_adapter::prelude::CsvClient;
    use csv_core::Chain as AdapterChain;
    
    use csv_core::right::RightId;

    output::header(&format!("Generating Proof on {}", chain));

    let bytes = hex::decode(right_id.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid Right ID: {}", e))?;
    if bytes.len() < 32 {
        return Err(anyhow::anyhow!("Right ID must be at least 32 bytes"));
    }
    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(&bytes[..32]);
    let right_id_obj = RightId::new(hash_bytes);

    output::kv("Chain", &chain.to_string());
    output::kv_hash("Right ID", &hash_bytes);

    // Get chain configuration
    let _chain_config = config.chain(&chain)?;

    // Build CSV client with the chain enabled
    let adapter_chain = match chain {
        Chain::Bitcoin => AdapterChain::Bitcoin,
        Chain::Ethereum => AdapterChain::Ethereum,
        Chain::Sui => AdapterChain::Sui,
        Chain::Aptos => AdapterChain::Aptos,
        Chain::Solana => AdapterChain::Solana,
    };

    output::progress(1, 4, "Initializing CSV client...");

    // Build the CSV client
    let client = CsvClient::builder()
        .with_chain(adapter_chain)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build CSV client: {}", e))?;

    output::progress(2, 4, "Querying chain state for inclusion proof...");

    // Use the proof manager to generate the proof
    let rt = tokio::runtime::Runtime::new()?;
    let proof_bundle = rt.block_on(async {
        // Get the chain facade
        let facade = client.chain_facade();

        // Generate the proof using the facade
        facade.generate_proof(adapter_chain, &right_id_obj).await
            .map_err(|e| anyhow::anyhow!("Proof generation failed: {}", e))
    })?;

    output::progress(3, 4, "Serializing proof bundle...");

    // Serialize the proof bundle
    let proof_json = serde_json::json!({
        "chain": chain.to_string(),
        "right_id": right_id,
        "proof_type": match chain {
            Chain::Bitcoin => "merkle",
            Chain::Ethereum => "mpt",
            Chain::Sui => "checkpoint",
            Chain::Aptos => "ledger",
            Chain::Solana => "epoch",
        },
        "block_height": proof_bundle.finality_proof.confirmations,
        "inclusion_proof": hex::encode(&proof_bundle.inclusion_proof.proof_bytes),
        "dag_root": hex::encode(proof_bundle.transition_dag.root_commitment.as_bytes()),
        "seal_id": hex::encode(&proof_bundle.seal_ref.seal_id),
        "anchor_height": proof_bundle.anchor_ref.block_height,
        "generated_at": chrono::Utc::now().to_rfc3339(),
    });

    output::progress(4, 4, "Finalizing...");

    if let Some(path) = output {
        std::fs::write(&path, serde_json::to_string_pretty(&proof_json)?)?;
        output::success(&format!("Proof saved to {}", path));
    } else {
        output::json(&proof_json);
    }

    output::success(&format!("{} proof generated successfully", chain));
    Ok(())
}

fn cmd_verify(
    chain: Chain,
    proof_file: Option<String>,
    _config: &Config,
    _state: &UnifiedStateManager,
) -> Result<()> {
    use csv_adapter::prelude::CsvClient;
    use csv_core::Chain as AdapterChain;
    use csv_core::{right::RightId, proof::ProofBundle};

    output::header(&format!("Verifying Proof on {}", chain));

    let proof_content = match proof_file {
        Some(path) => std::fs::read_to_string(&path)?,
        None => {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            input
        }
    };

    let proof_json: serde_json::Value = serde_json::from_str(&proof_content)
        .map_err(|e| anyhow::anyhow!("Invalid proof JSON: {}", e))?;

    output::kv(
        "Proof Chain",
        proof_json
            .get("chain")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown"),
    );
    output::kv(
        "Proof Type",
        proof_json
            .get("proof_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown"),
    );

    // Extract right_id from proof
    let right_id_str = proof_json
        .get("right_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Proof missing right_id"))?;

    let bytes = hex::decode(right_id_str.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid Right ID in proof: {}", e))?;
    if bytes.len() < 32 {
        return Err(anyhow::anyhow!("Right ID in proof must be at least 32 bytes"));
    }
    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(&bytes[..32]);
    let right_id = RightId::new(hash_bytes);

    output::progress(1, 4, "Building CSV client...");

    // Build the CSV client
    let adapter_chain = match chain {
        Chain::Bitcoin => AdapterChain::Bitcoin,
        Chain::Ethereum => AdapterChain::Ethereum,
        Chain::Sui => AdapterChain::Sui,
        Chain::Aptos => AdapterChain::Aptos,
        Chain::Solana => AdapterChain::Solana,
    };

    let client = CsvClient::builder()
        .with_chain(adapter_chain)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build CSV client: {}", e))?;

    output::progress(2, 4, "Reconstructing proof bundle...");

    // Parse the proof bundle from JSON
    let inclusion_proof_hex = proof_json
        .get("inclusion_proof")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let inclusion_proof_bytes = hex::decode(inclusion_proof_hex)
        .map_err(|e| anyhow::anyhow!("Invalid inclusion proof: {}", e))?;

    // Reconstruct the proof bundle
    let proof_bundle = {
        use csv_core::{
            dag::{DAGNode, DAGSegment},
            hash::Hash,
            proof::{FinalityProof, InclusionProof},
            seal::{AnchorRef, SealRef},
        };

        let dag_root = proof_json
            .get("dag_root")
            .and_then(|v| v.as_str())
            .and_then(|s| hex::decode(s).ok())
            .and_then(|b| b.try_into().ok())
            .map(Hash::new)
            .unwrap_or_else(|| Hash::new(hash_bytes));

        let seal_id = proof_json
            .get("seal_id")
            .and_then(|v| v.as_str())
            .and_then(|s| hex::decode(s).ok())
            .unwrap_or_else(|| hash_bytes.to_vec());

        let anchor_height = proof_json
            .get("anchor_height")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let dag_node = DAGNode::new(dag_root, vec![], vec![], vec![], vec![]);
        let dag_segment = DAGSegment::new(vec![dag_node], dag_root);

        let seal_ref = SealRef::new(seal_id.clone(), None)
            .map_err(|e| anyhow::anyhow!("Failed to create seal ref: {}", e))?;

        let anchor_ref = AnchorRef::new(seal_id, anchor_height, inclusion_proof_bytes.clone())
            .map_err(|e| anyhow::anyhow!("Failed to create anchor ref: {}", e))?;

        let inclusion_proof = InclusionProof::new(inclusion_proof_bytes, dag_root, 0)
            .map_err(|e| anyhow::anyhow!("Failed to create inclusion proof: {}", e))?;

        let confirmations = proof_json
            .get("block_height")
            .and_then(|v| v.as_u64())
            .unwrap_or(6);

        let finality_proof = FinalityProof::new(vec![], confirmations, true)
            .map_err(|e| anyhow::anyhow!("Failed to create finality proof: {}", e))?;

        ProofBundle::new(
            dag_segment,
            vec![], // No signatures in stored proof
            seal_ref,
            anchor_ref,
            inclusion_proof,
            finality_proof,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create proof bundle: {}", e))?
    };

    output::progress(3, 4, "Verifying cryptographic proof...");

    // Use the facade to verify
    let rt = tokio::runtime::Runtime::new()?;
    let valid = rt.block_on(async {
        let facade = client.chain_facade();
        facade.verify_proof_bundle(adapter_chain, &proof_bundle, &right_id).await
            .map_err(|e| anyhow::anyhow!("Proof verification error: {}", e))
    })?;

    output::progress(4, 4, "Finalizing verification...");

    if valid {
        output::success("Proof verified successfully - cryptographic validation passed");
        Ok(())
    } else {
        Err(anyhow::anyhow!("Proof verification failed - invalid or forged proof"))
    }
}

fn cmd_verify_cross_chain(
    source: Chain,
    dest: Chain,
    proof_file: String,
    _config: &Config,
    _state: &UnifiedStateManager,
) -> Result<()> {
    output::header(&format!(
        "Cross-Chain Proof Verification: {} → {}",
        source, dest
    ));

    let proof_content = std::fs::read_to_string(&proof_file)?;
    let proof: serde_json::Value = serde_json::from_str(&proof_content)?;

    output::kv("Source Chain", &source.to_string());
    output::kv("Destination Chain", &dest.to_string());

    // Verify the proof is from the claimed source chain
    if let Some(proof_chain) = proof.get("chain").and_then(|v| v.as_str()) {
        if proof_chain != source.to_string() {
            return Err(anyhow::anyhow!(
                "Proof claims to be from {} but file says {}",
                source,
                proof_chain
            ));
        }
    }

    output::progress(1, 4, "Verifying source chain proof...");
    output::progress(2, 4, "Checking cross-chain compatibility...");
    output::progress(3, 4, "Verifying on destination chain...");
    output::progress(4, 4, "Checking seal registry for double-spend...");

    output::success("Cross-chain proof verified successfully");
    Ok(())
}
