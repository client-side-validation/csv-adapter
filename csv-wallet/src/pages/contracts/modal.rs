//! Contract detail modal component.

use crate::context::DeployedContract;
use crate::pages::common::*;
use dioxus::prelude::*;

#[component]
pub fn ContractDetailModal(
    contract: DeployedContract,
    on_close: EventHandler<()>,
    on_use_in_transfer: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "fixed inset-0 bg-black/60 z-50 flex items-center justify-center",
            div { class: "{card_class()} max-w-lg w-full mx-4 max-h-[90vh] overflow-y-auto",
                div { class: "{card_header_class()} flex items-center justify-between",
                    h2 { class: "font-semibold", "Contract Details" }
                    button {
                        onclick: move |_| { on_close.call(()); },
                        class: "text-gray-400 hover:text-gray-200",
                        "\u{2715}"
                    }
                }
                div { class: "p-6 space-y-4",
                    div {
                        p { class: "text-xs text-gray-400 mb-1", "Address" }
                        p { class: "font-mono text-sm text-gray-200 break-all", "{contract.address}" }
                    }
                    div {
                        p { class: "text-xs text-gray-400 mb-1", "Chain" }
                        p { class: "text-sm", "{chain_name(&contract.chain)}" }
                    }
                    div {
                        p { class: "text-xs text-gray-400 mb-1", "Transaction Hash" }
                        p { class: "font-mono text-sm text-gray-300", "{truncate_address(&contract.tx_hash, 16)}" }
                    }
                    div { class: "flex gap-2 pt-4",
                        button {
                            onclick: move |_| { on_use_in_transfer.call(()); },
                            class: "{btn_primary_class()}",
                            "Use in Transfer"
                        }
                        button {
                            onclick: move |_| { on_close.call(()); },
                            class: "{btn_secondary_class()}",
                            "Close"
                        }
                    }
                }
            }
        }
    }
}
