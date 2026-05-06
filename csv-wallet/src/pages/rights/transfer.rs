//! Transfer sanad page.

use crate::context::{use_wallet_context, SanadStatus};
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn TransferSanad() -> Element {
    let wallet_ctx = use_wallet_context();
    let sanads = wallet_ctx.sanads();
    // Filter only active sanads
    let active_sanads: Vec<_> = sanads
        .into_iter()
        .filter(|r| r.status == SanadStatus::Active)
        .collect();
    let has_active_sanads = !active_sanads.is_empty();
    // Clone for use in closure
    let active_sanads_for_closure = active_sanads.clone();

    let mut selected_sanad_index = use_signal(|| 0usize);
    let mut to_address = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| false);
    let _blockchain = crate::services::blockchain::BlockchainService::new(Default::default());
    let wallet_ctx = use_wallet_context();

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Sanads {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Transfer Sanad" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Available Sanads", rsx! {
                    if active_sanads.is_empty() {
                        p { class: "text-sm text-red-400", "No active sanads available. Create a sanad first." }
                    } else {
                        select {
                            class: "{input_mono_class()}",
                            onchange: move |evt| {
                                if let Ok(idx) = evt.value().parse::<usize>() {
                                    selected_sanad_index.set(idx);
                                }
                            },
                            for (idx, sanad) in active_sanads.iter().enumerate() {
                                option { key: "transfer-sanad-{idx}", value: idx.to_string(), selected: idx == *selected_sanad_index.read(),
                                    {format!("{} - {} - Value: {} - {}",
                                        &sanad.id[..12.min(sanad.id.len())],
                                        chain_name(&sanad.chain),
                                        sanad.value,
                                        sanad.status
                                    )}
                                }
                            }
                        }
                    }
                })}

                // Show selected sanad details
                if let Some(sanad) = active_sanads.get(*selected_sanad_index.read()) {
                    div { class: "bg-gray-800/50 rounded-lg p-3 border border-gray-700",
                        p { class: "text-xs text-gray-400 mb-2", "Selected Sanad Details:" }
                        div { class: "grid grid-cols-2 gap-2 text-xs",
                            div { span { class: "text-gray-500", "ID: " }, span { class: "font-mono text-gray-300", "{truncate_address(&sanad.id, 10)}" } }
                            div { span { class: "text-gray-500", "Chain: " }, span { class: "{chain_badge_class(&sanad.chain)}", "{chain_icon_emoji(&sanad.chain)} {chain_name(&sanad.chain)}" } }
                            div { span { class: "text-gray-500", "Value: " }, span { class: "font-mono text-gray-300", "{sanad.value}" } }
                            div { span { class: "text-gray-500", "Status: " }, span { class: "{sanad_status_class(&sanad.status)}", "{sanad.status}" } }
                            div { span { class: "text-gray-500", "Owner: " }, span { class: "font-mono text-gray-300", "{truncate_address(&sanad.owner, 8)}" } }
                        }
                    }
                }

                {form_field("New Owner Address", rsx! {
                    input {
                        value: "{to_address.read()}",
                        oninput: move |evt| { to_address.set(evt.value()); },
                        class: "{input_mono_class()}",
                        r#type: "text"
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        if let Some(sanad) = active_sanads_for_closure.get(*selected_sanad_index.read()) {
                            let sanad_id = sanad.id.clone();
                            let chain = sanad.chain;
                            let to_addr = to_address.read().clone();
                            let mut wallet_ctx = wallet_ctx.clone();
                            let blockchain = crate::services::blockchain::BlockchainService::new(Default::default());
                            let signer = wallet_ctx.get_signer_for_chain(chain).unwrap();

                            spawn(async move {
                                loading.set(true);
                                result.set(None);

                                match blockchain.transfer_sanad_local(chain, &sanad_id, &to_addr, &std::collections::HashMap::new(), &signer).await {
                                    Ok(tx_hash) => {
                                        result.set(Some(format!("✅ Transfer successful! Transaction: {}", truncate_address(&tx_hash, 12))));
                                        wallet_ctx.refresh_sanads().await;
                                    },
                                    Err(e) => {
                                        result.set(Some(format!("❌ Transfer failed: {}", e)));
                                    }
                                }

                                loading.set(false);
                            });
                        }
                    },
                    disabled: !has_active_sanads || *loading.read() || to_address.read().is_empty(),
                    class: if !has_active_sanads || *loading.read() { "{btn_full_primary_class()} opacity-50 cursor-not-allowed" } else { "{btn_full_primary_class()}" },
                    if *loading.read() { "Processing Transfer..." }
                    else if !has_active_sanads { "No Sanads Available" }
                    else { "Transfer" }
                }
            }
        }
    }
}
