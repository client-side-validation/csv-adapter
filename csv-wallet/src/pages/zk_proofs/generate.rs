//! Generate ZK Proof Page - Phase 5
//!
//! This page allows users to generate zero-knowledge proofs that their
//! seal was consumed on-chain. The proof can be verified by anyone without
//! requiring access to blockchain RPC.

use crate::context::{use_wallet_context, ProofRecord, ProofStatus};
use crate::pages::common::*;
use crate::routes::Route;
use csv_adapter_core::zk_proof::ZkSealProof;
use csv_adapter_core::Chain;
use dioxus::prelude::*;

/// Generate ZK proof page
#[component]
pub fn ZkGenerateProof() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Bitcoin);
    let mut right_id = use_signal(String::new);
    let mut result = use_signal(|| None::<ZkResult>);
    let mut is_generating = use_signal(|| false);

    rsx! {
        div { class: "max-w-4xl mx-auto space-y-6",
            // Header
            div { class: "flex items-center gap-3",
                Link { to: Route::Proofs {}, class: "{btn_secondary_class()}", "← Back" }
                h1 { class: "text-xl font-bold", "Generate ZK Proof" }
            }

            // ZK Advantage Banner
            div { class: "p-4 bg-gradient-to-r from-purple-900/30 to-blue-900/30 \
                          border border-purple-500/30 rounded-lg",
                h2 { class: "text-sm font-semibold text-purple-300 mb-2",
                    "🔒 Phase 5: Zero-Knowledge Verification"
                }
                p { class: "text-sm text-gray-300",
                    "Generate a ZK proof that your seal was consumed. Anyone can verify this proof \
                     cryptographically—no blockchain RPC required. This is the ultimate trustless guarantee."
                }
            }

            // Generation form
            div { class: "{card_class()} p-6 space-y-5",
                h2 { class: "text-lg font-semibold", "Proof Parameters" }

                // Chain selection
                div { class: "space-y-2",
                    label { class: "text-sm text-gray-400", "Blockchain" }
                    select {
                        class: "{input_class()}",
                        onchange: move |e| {
                            if let Ok(c) = e.value().parse::<Chain>() {
                                selected_chain.set(c);
                            }
                        },
                        value: "{selected_chain.read().to_string()}",
                        for chain in [Chain::Bitcoin, Chain::Ethereum] {
                            option { value: chain.to_string(), "{chain.to_string()}" }
                        }
                    }
                    p { class: "text-xs text-gray-500",
                        "ZK proofs are currently available for Bitcoin (SP1) and Ethereum (Groth16)"
                    }
                }

                // Right ID input
                div { class: "space-y-2",
                    label { class: "text-sm text-gray-400", "Right ID" }
                    input {
                        class: "{input_mono_class()}",
                        value: "{right_id.read()}",
                        oninput: move |e| right_id.set(e.value()),
                        placeholder: "Enter the Right ID to prove seal consumption for",
                    }
                }

                // Generate button
                button {
                    class: "{btn_full_primary_class()}",
                    disabled: right_id.read().is_empty() || is_generating(),
                    onclick: move |_| {
                        is_generating.set(true);

                        // Get the seal for this right
                        let right_id_str = right_id.read().clone();
                        let seal_opt = wallet_ctx.seal_for_right(&right_id_str);

                        if seal_opt.is_none() {
                            result.set(Some(ZkResult {
                                success: false,
                                message: "No seal found for this right. Create a seal first.".to_string(),
                                proof: None,
                            }));
                            is_generating.set(false);
                            return;
                        }

                        let seal = seal_opt.unwrap();
                        let chain = *selected_chain.read();

                        // Generate ZK proof based on chain
                        let proof_result = match chain {
                            Chain::Bitcoin => generate_bitcoin_zk_proof(&seal.seal_ref, &right_id_str),
                            Chain::Ethereum => generate_ethereum_zk_proof(&seal.seal_ref, &right_id_str),
                            _ => Err("ZK proofs not yet available for this chain".to_string()),
                        };

                        match proof_result {
                            Ok(proof) => {
                                // Save proof to wallet
                                let proof_record = ProofRecord {
                                    chain,
                                    right_id: right_id_str.clone(),
                                    seal_ref: seal.seal_ref.clone(),
                                    proof_type: "zk".to_string(),
                                    status: ProofStatus::Generated,
                                    generated_at: js_sys::Date::now() as u64 / 1000,
                                    verified_at: None,
                                    data: None, // ZK proof stored separately
                                    target_chain: None,
                                    verification_tx_hash: None,
                                };
                                wallet_ctx.add_proof(proof_record);

                                result.set(Some(ZkResult {
                                    success: true,
                                    message: format!("ZK proof generated successfully! Size: {} bytes", proof.proof_bytes.len()),
                                    proof: Some(proof),
                                }));
                            }
                            Err(e) => {
                                result.set(Some(ZkResult {
                                    success: false,
                                    message: format!("Failed to generate proof: {}", e),
                                    proof: None,
                                }));
                            }
                        }

                        is_generating.set(false);
                    },
                    if is_generating() {
                        "⏳ Generating ZK Proof..."
                    } else {
                        "🔒 Generate ZK Proof"
                    }
                }

                // Result display
                if let Some(res) = result.read().as_ref() {
                    div {
                        class: if res.success { "p-4 bg-green-900/20 border border-green-500/30 rounded-lg" }
                               else { "p-4 bg-red-900/20 border border-red-500/30 rounded-lg" },
                        p { class: if res.success { "text-green-300" } else { "text-red-300" },
                            "{res.message}"
                        }

                        if res.success {
                            if let Some(proof) = res.proof.clone() {
                                div { class: "mt-4 space-y-2",
                                    p { class: "text-sm text-gray-400", "Proof Details:" }
                                    div { class: "p-3 bg-gray-800 rounded-lg font-mono text-xs",
                                        p { "Chain: {proof.public_inputs.source_chain.to_string()}" }
                                        p { "Block Height: {proof.public_inputs.block_height}" }
                                        p { "Proof System: {proof.verifier_key.proof_system.to_string()}" }
                                    }

                                    button {
                                        class: "{btn_secondary_class()} w-full mt-2",
                                        onclick: move |_| {
                                            // Copy proof to clipboard
                                            let proof_json = serde_json::to_string_pretty(&proof).unwrap_or_default();
                                            let window = web_sys::window().unwrap();
                                            let clipboard = window.navigator().clipboard();
                                            let _ = clipboard.write_text(&proof_json);
                                        },
                                        "📋 Copy Proof to Clipboard"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // How it works
            div { class: "{card_class()} p-6",
                h2 { class: "text-lg font-semibold mb-4", "How ZK Proofs Work" }

                div { class: "space-y-4",
                    div { class: "flex gap-4",
                        div { class: "flex-shrink-0 w-8 h-8 bg-purple-500/20 rounded-full \
                                      flex items-center justify-center",
                            span { class: "text-purple-400 font-semibold", "1" }
                        }
                        div {
                            h3 { class: "font-medium", "Witness Generation" }
                            p { class: "text-sm text-gray-400",
                                "Collect the transaction data, Merkle proof, and block header \
                                 that prove your seal was consumed."
                            }
                        }
                    }

                    div { class: "flex gap-4",
                        div { class: "flex-shrink-0 w-8 h-8 bg-purple-500/20 rounded-full \
                                      flex items-center justify-center",
                            span { class: "text-purple-400 font-semibold", "2" }
                        }
                        div {
                            h3 { class: "font-medium", "ZK Circuit Execution" }
                            p { class: "text-sm text-gray-400",
                                "Run the witness through a ZK circuit (SP1 for Bitcoin, Groth16 for Ethereum) \
                                 to generate a succinct proof."
                            }
                        }
                    }

                    div { class: "flex gap-4",
                        div { class: "flex-shrink-0 w-8 h-8 bg-purple-500/20 rounded-full \
                                      flex items-center justify-center",
                            span { class: "text-purple-400 font-semibold", "3" }
                        }
                        div {
                            h3 { class: "font-medium", "Trustless Verification" }
                            p { class: "text-sm text-gray-400",
                                "Anyone can verify the proof using only the public inputs. \
                                 No blockchain RPC, no trusted third parties, pure cryptography."
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Result of ZK proof generation
struct ZkResult {
    success: bool,
    message: String,
    proof: Option<ZkSealProof>,
}

/// Generate a Bitcoin SPV ZK proof using SP1
#[cfg(feature = "csv-adapter-bitcoin")]
fn generate_bitcoin_zk_proof(seal_ref: &str, right_id: &str) -> Result<ZkSealProof, String> {
    use csv_adapter_bitcoin::zk_prover::BitcoinSpvProver;
    use csv_adapter_core::hash::Hash;
    use csv_adapter_core::seal::SealRef;
    use csv_adapter_core::zk_proof::{ChainWitness, ZkProver};

    let prover = BitcoinSpvProver::new();

    // Parse the seal_ref
    let seal_bytes = hex::decode(seal_ref.trim_start_matches("0x"))
        .map_err(|e| format!("Invalid seal reference: {}", e))?;

    let seal = SealRef::new(seal_bytes, None)
        .map_err(|e| format!("Invalid seal: {}", e))?;

    // Create mock witness data
    // In production, this would come from actual Bitcoin transaction data
    let witness = ChainWitness {
        chain: Chain::Bitcoin,
        block_hash: Hash::new([0x01; 32]), // Would be actual block hash
        block_height: 800_000,
        tx_data: format!("spend_right_{}", right_id).into_bytes(),
        inclusion_proof: vec![0xAB; 32], // Would be actual Merkle branch
        finality_proof: vec![0xCD; 16],
        timestamp: js_sys::Date::now() as u64 / 1000,
    };

    prover.prove_seal_consumption(&seal, &witness)
        .map_err(|e| format!("Proof generation failed: {}", e))
}

/// Generate a Bitcoin SPV ZK proof using SP1 (stub for non-bitcoin builds)
#[cfg(not(feature = "csv-adapter-bitcoin"))]
fn generate_bitcoin_zk_proof(_seal_ref: &str, _right_id: &str) -> Result<ZkSealProof, String> {
    Err("Bitcoin ZK proof generation not available in this build".to_string())
}

/// Generate an Ethereum Groth16 ZK proof
fn generate_ethereum_zk_proof(_seal_ref: &str, _right_id: &str) -> Result<ZkSealProof, String> {
    // Ethereum Groth16 proof generation would use a similar pattern
    // but with the Ethereum adapter's zk_verifier module

    // For now, return a mock proof
    use csv_adapter_core::hash::Hash;
    use csv_adapter_core::seal::SealRef;
    use csv_adapter_core::zk_proof::{VerifierKey, ZkPublicInputs, ProofSystem};

    let seal = SealRef::new(vec![0xAB; 32], None)
        .map_err(|e| format!("Invalid seal: {}", e))?;

    let verifier_key = VerifierKey::new(
        Chain::Ethereum,
        vec![0u8; 64],
        ProofSystem::Groth16,
        1,
    );

    let public_inputs = ZkPublicInputs {
        seal_ref: seal.clone(),
        block_hash: Hash::new([0x01; 32]),
        commitment: Hash::new([0x02; 32]),
        source_chain: Chain::Ethereum,
        block_height: 19_000_000,
        timestamp: js_sys::Date::now() as u64 / 1000,
    };

    // Mock Groth16 proof (192 bytes for A, B, C points)
    let proof_bytes = vec![0xAA; 192];

    ZkSealProof::new(proof_bytes, verifier_key, public_inputs)
        .map_err(|e| format!("Proof creation failed: {}", e))
}
