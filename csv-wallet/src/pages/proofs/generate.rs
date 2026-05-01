//! Generate proof page.

use crate::context::{use_wallet_context, ProofRecord, ProofStatus};
use crate::pages::common::*;
use crate::routes::Route;
use csv_adapter_core::Chain;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn GenerateProof() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Bitcoin);
    let mut right_id = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);

    let proof_type = match *selected_chain.read() {
        Chain::Bitcoin => "merkle",
        Chain::Ethereum => "mpt",
        Chain::Sui => "checkpoint",
        Chain::Aptos => "ledger",
        Chain::Solana => "merkle",
        _ => "unknown",
    };

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Proofs {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Generate Proof" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Source Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
                }, *selected_chain.read()))}

                {form_field("Right ID", rsx! {
                    input {
                        value: "{right_id.read()}",
                        oninput: move |evt| { right_id.set(evt.value()); },
                        class: "{input_mono_class()}",
                        r#type: "text"
                    }
                })}

                div { class: "bg-gray-800/50 rounded-lg p-3 border border-gray-700",
                    p { class: "text-xs text-gray-400", "Proof Type: " strong { class: "text-gray-300", "{proof_type}" } }
                }

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300 font-mono text-sm break-all", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        let proof_json = serde_json::json!({
                            "chain": selected_chain.read().to_string(),
                            "right_id": right_id.read().clone(),
                            "proof_type": proof_type,
                            "data": "proof_data_value"
                        });
                        let formatted = serde_json::to_string_pretty(&proof_json).unwrap_or_default();
                        result.set(Some(formatted));
                        wallet_ctx.add_proof(ProofRecord {
                            chain: *selected_chain.read(),
                            right_id: right_id.read().clone(),
                            seal_ref: format!("seal_{}", &right_id.read()[..16.min(right_id.read().len())]),
                            proof_type: proof_type.to_string(),
                            status: ProofStatus::Generated,
                            generated_at: js_sys::Date::now() as u64 / 1000,
                            verified_at: None,
                            data: None,
                            target_chain: None,
                            verification_tx_hash: None,
                        });
                    },
                    class: "{btn_full_primary_class()}",
                    "Generate Proof"
                }
            }
        }
    }
}
