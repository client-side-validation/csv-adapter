//! Proofs list page.

use crate::context::{use_wallet_context, ProofRecord};
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn Proofs() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let proofs = wallet_ctx.proofs();
    let mut selected_proof = use_signal(|| None::<ProofRecord>);
    let mut show_delete_confirm = use_signal(|| None::<ProofRecord>);

    // Collect proofs into owned vector for use in closures
    let proofs_owned: Vec<_> = proofs.into_iter().collect();

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold", "Proofs" }
                div { class: "flex gap-2",
                    Link { to: Route::GenerateProof {}, class: "{btn_primary_class()}", "+ Generate" }
                    Link { to: Route::VerifyProof {}, class: "{btn_secondary_class()}", "Verify" }
                }
            }

            if proofs_owned.is_empty() {
                {empty_state("\u{1F4C4}", "No proofs generated", "Generate or verify proofs for cross-chain transfers.")}
            } else {
                div { class: "{table_class()}",
                    div { class: "{card_header_class()} flex items-center justify-between",
                        h2 { class: "font-semibold text-sm", "Proof Records" }
                        span { class: "text-xs text-gray-400", "{proofs_owned.len()} total" }
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "text-left text-gray-400 border-b border-gray-800",
                                    th { class: "px-4 py-2 font-medium", "Chain" }
                                    th { class: "px-4 py-2 font-medium", "Right ID" }
                                    th { class: "px-4 py-2 font-medium", "Type" }
                                    th { class: "px-4 py-2 font-medium", "Verified" }
                                    th { class: "px-4 py-2 font-medium", "Actions" }
                                }
                            }
                            tbody { class: "divide-y divide-gray-800",
                                for (idx, proof) in proofs_owned.iter().enumerate() {
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
                                        td { class: "px-4 py-3",
                                            div { class: "flex gap-2",
                                                {
                                                    let proof_for_view = proof.clone();
                                                    rsx! {
                                                        button {
                                                            onclick: move |_| selected_proof.set(Some(proof_for_view.clone())),
                                                            class: "px-2 py-1 rounded text-xs bg-blue-900/30 text-blue-400 hover:bg-blue-900/50 transition-colors",
                                                            "View"
                                                        }
                                                    }
                                                }
                                                {
                                                    let proof_for_delete = proof.clone();
                                                    rsx! {
                                                        button {
                                                            onclick: move |_| show_delete_confirm.set(Some(proof_for_delete.clone())),
                                                            class: "px-2 py-1 rounded text-xs bg-red-900/30 text-red-400 hover:bg-red-900/50 transition-colors",
                                                            "Delete"
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

            // Proof Detail Modal
            {
                let proof_opt = selected_proof.read().clone();
                let mut close_modal = selected_proof.clone();
                match proof_opt {
                    Some(proof) => rsx! {
                        div { class: "fixed inset-0 bg-black/50 flex items-center justify-center z-50",
                            div { class: "{card_class()} max-w-lg w-full mx-4",
                                div { class: "{card_header_class()} flex items-center justify-between",
                                    h3 { class: "font-semibold", "Proof Details" }
                                    button { onclick: move |_| close_modal.set(None), class: "text-gray-400 hover:text-gray-200", "\u{2715}" }
                                }
                                div { class: "p-4 space-y-4",
                                    div { class: "space-y-2",
                                        p { class: "text-sm text-gray-400", "Chain" }
                                        p { class: "text-sm", span { class: "{chain_badge_class(&proof.chain)}", "{chain_icon_emoji(&proof.chain)} {chain_name(&proof.chain)}" } }
                                    }
                                    div { class: "space-y-2",
                                        p { class: "text-sm text-gray-400", "Right ID" }
                                        p { class: "text-sm font-mono break-all", "{proof.right_id}" }
                                    }
                                    div { class: "space-y-2",
                                        p { class: "text-sm text-gray-400", "Proof Type" }
                                        p { class: "text-sm", "{proof.proof_type}" }
                                    }
                                    div { class: "space-y-2",
                                        p { class: "text-sm text-gray-400", "Status" }
                                        p { class: "text-sm",
                                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium",
                                                class: if proof.verified { "text-green-400 bg-green-500/20" } else { "text-yellow-400 bg-yellow-500/20" },
                                                if proof.verified { "Verified" } else { "Pending" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    None => rsx! {}
                }
            }

            // Delete Confirmation Modal
            {
                let proof_opt = show_delete_confirm.read().clone();
                let mut close_modal = show_delete_confirm.clone();
                let mut ctx = wallet_ctx.clone();
                match proof_opt {
                    Some(proof) => rsx! {
                        div { class: "fixed inset-0 bg-black/50 flex items-center justify-center z-50",
                            div { class: "{card_class()} max-w-md w-full mx-4",
                                div { class: "p-6 space-y-4",
                                    div { class: "flex items-center gap-3",
                                        span { class: "text-2xl", "\u{26A0}\u{FE0F}" }
                                        h3 { class: "font-semibold text-lg", "Delete Proof?" }
                                    }
                                    p { class: "text-sm text-gray-400",
                                        "Are you sure you want to delete this proof? This action cannot be undone."
                                    }
                                    div { class: "bg-gray-800/50 rounded-lg p-3",
                                        p { class: "text-xs text-gray-500", "Right ID: {truncate_address(&proof.right_id, 20)}" }
                                        p { class: "text-xs text-gray-500", "Chain: {chain_name(&proof.chain)}" }
                                    }
                                    div { class: "flex gap-3",
                                        button {
                                            onclick: move |_| close_modal.set(None),
                                            class: "flex-1 px-4 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 text-sm font-medium transition-colors",
                                            "Cancel"
                                        }
                                        button {
                                            onclick: move |_| {
                                                ctx.remove_proof(&proof.right_id, &proof.proof_type);
                                                close_modal.set(None);
                                            },
                                            class: "flex-1 px-4 py-2 rounded-lg bg-red-600 hover:bg-red-700 text-sm font-medium transition-colors",
                                            "Delete"
                                        }
                                    }
                                }
                            }
                        }
                    },
                    None => rsx! {}
                }
            }
        }
    }
}
