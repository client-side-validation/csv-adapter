//! Proofs list page.

use crate::context::use_wallet_context;
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn Proofs() -> Element {
    let wallet_ctx = use_wallet_context();
    let proofs = wallet_ctx.proofs();

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold", "Proofs" }
                div { class: "flex gap-2",
                    Link { to: Route::GenerateProof {}, class: "{btn_primary_class()}", "+ Generate" }
                    Link { to: Route::VerifyProof {}, class: "{btn_secondary_class()}", "Verify" }
                }
            }

            if proofs.is_empty() {
                {empty_state("\u{1F4C4}", "No proofs generated", "Generate or verify proofs for cross-chain transfers.")}
            } else {
                div { class: "{table_class()}",
                    div { class: "{card_header_class()}",
                        h2 { class: "font-semibold text-sm", "Proof Records" }
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "text-left text-gray-400 border-b border-gray-800",
                                    th { class: "px-4 py-2 font-medium", "Chain" }
                                    th { class: "px-4 py-2 font-medium", "Right ID" }
                                    th { class: "px-4 py-2 font-medium", "Type" }
                                    th { class: "px-4 py-2 font-medium", "Verified" }
                                }
                            }
                            tbody { class: "divide-y divide-gray-800",
                                for (idx, proof) in proofs.iter().enumerate() {
                                    tr { key: "{idx}-{proof.chain}-{proof.right_id}-{proof.proof_type}", class: "hover:bg-gray-800/50 transition-colors",
                                        td { class: "px-4 py-3", span { class: "{chain_badge_class(&proof.chain)}", "{chain_icon_emoji(&proof.chain)} {chain_name(&proof.chain)}" } }
                                        td { class: "px-4 py-3 font-mono text-xs", "{truncate_address(&proof.right_id, 8)}" }
                                        td { class: "px-4 py-3 text-xs", "{proof.proof_type}" }
                                        td { class: "px-4 py-3",
                                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium",
                                                class: if proof.verified { "text-green-400 bg-green-500/20" } else { "text-yellow-400 bg-yellow-500/20" },
                                                if proof.verified { "Verified" } else { "Pending" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
