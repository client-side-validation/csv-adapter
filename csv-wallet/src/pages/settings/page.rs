//! Settings page.

use crate::context::use_wallet_context;
use crate::pages::common::*;
use csv_core::PROTOCOL_VERSION;
use dioxus::prelude::*;

pub fn Settings() -> Element {
    let wallet_ctx = use_wallet_context();
    let mut show_lock_confirm = use_signal(|| false);
    let mut show_clear_data = use_signal(|| false);
    let is_initialized = wallet_ctx.is_initialized();
    let has_wallet = is_initialized;

    // Clone for closures
    let mut ctx_lock = wallet_ctx.clone();
    let mut ctx_clear = wallet_ctx.clone();

    rsx! {
        div { class: "max-w-2xl space-y-6 stagger-children",
            h1 { class: "text-2xl font-bold", "Settings" }

            // Wallet section
            div { class: "{card_class()} overflow-hidden",
                div { class: "{card_header_class()}",
                    h3 { class: "font-semibold text-sm", "Wallet" }
                }
                div { class: "p-6 space-y-4",
                    // Status
                    div { class: "flex items-center justify-between",
                        span { class: "text-sm text-gray-400", "Status" }
                        div { class: "flex items-center gap-2",
                            span { class: "w-2 h-2 rounded-full", class: if has_wallet { "bg-green-500 status-online" } else { "bg-gray-500" } }
                            span { class: "text-sm", if has_wallet { "Unlocked" } else { "Locked" } }
                        }
                    }

                    div { class: "flex items-center justify-between",
                        span { class: "text-sm text-gray-400", "Initialized" }
                        span { class: "text-sm", if is_initialized { "Yes" } else { "No" } }
                    }

                    div { class: "flex gap-3 pt-2",
                        button {
                            onclick: move |_| show_lock_confirm.set(true),
                            disabled: !has_wallet,
                            class: "{btn_secondary_class()} disabled:opacity-50 disabled:cursor-not-allowed",
                            "\u{1F512} Lock Wallet"
                        }
                        button {
                            onclick: move |_| show_clear_data.set(true),
                            class: "px-4 py-2 rounded-lg bg-red-900/30 hover:bg-red-900/50 border border-red-700/50 text-sm font-medium transition-colors text-red-300",
                            "\u{1F5D1}\u{FE0F} Clear Data"
                        }
                    }
                }
            }

            // About
            div { class: "{card_class()} overflow-hidden",
                div { class: "{card_header_class()}",
                    h3 { class: "font-semibold text-sm", "About" }
                }
                div { class: "p-6 space-y-3",
                    div { class: "flex justify-between",
                        span { class: "text-sm text-gray-400", "Version" }
                        span { class: "text-sm font-mono", "{PROTOCOL_VERSION}" }
                    }
                    div { class: "flex justify-between",
                        span { class: "text-sm text-gray-400", "Chains" }
                        span { class: "text-sm", "Bitcoin, Ethereum, Sui, Aptos" }
                    }
                    div { class: "flex justify-between",
                        span { class: "text-sm text-gray-400", "Framework" }
                        span { class: "text-sm font-mono", "Dioxus 0.7" }
                    }
                    div { class: "flex justify-between",
                        span { class: "text-sm text-gray-400", "Storage" }
                        span { class: "text-sm", "localStorage (persistent)" }
                    }
                }
            }
        }

        // Lock confirmation modal
        if *show_lock_confirm.read() {
            div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/50 modal-backdrop",
                div { class: "{card_class()} p-6 max-w-sm mx-4 modal-content",
                    div { class: "flex items-center gap-2 mb-4",
                        span { class: "text-yellow-400 text-xl", "\u{26A0}\u{FE0F}" }
                        h3 { class: "font-semibold", "Lock Wallet?" }
                    }
                    p { class: "text-sm text-gray-400 mb-4",
                        "This will remove the wallet from memory. Your sanads, seals, and other data will remain saved, but you'll need to re-import your wallet to access them."
                    }
                    div { class: "flex gap-3",
                        button {
                            onclick: move |_| show_lock_confirm.set(false),
                            class: "flex-1 {btn_secondary_class()}",
                            "Cancel"
                        }
                        button {
                            onclick: move |_| {
                                ctx_lock.lock();
                                show_lock_confirm.set(false);
                            },
                            class: "flex-1 px-4 py-2 rounded-lg bg-red-600 hover:bg-red-700 text-sm font-medium transition-colors",
                            "Lock"
                        }
                    }
                }
            }
        }

        // Clear data confirmation modal
        if *show_clear_data.read() {
            div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/50 modal-backdrop",
                div { class: "{card_class()} p-6 max-w-sm mx-4 modal-content",
                    div { class: "flex items-center gap-2 mb-4",
                        span { class: "text-red-400 text-xl", "\u{26A0}\u{FE0F}" }
                        h3 { class: "font-semibold text-red-300", "Clear All Data?" }
                    }
                    p { class: "text-sm text-gray-400 mb-4",
                        "This will permanently delete all wallet data, sanads, seals, transfers, and settings from localStorage. This action cannot be undone."
                    }
                    div { class: "flex gap-3",
                        button {
                            onclick: move |_| show_clear_data.set(false),
                            class: "flex-1 {btn_secondary_class()}",
                            "Cancel"
                        }
                        button {
                            onclick: move |_| {
                                // Clear all localStorage
                                if let Ok(storage) = crate::storage::wallet_storage() {
                                    let _ = storage.delete(crate::storage::UNIFIED_STORAGE_KEY);
                                    let _ = storage.delete(crate::storage::WALLET_MNEMONIC_KEY);
                                }
                                ctx_clear.lock();
                                show_clear_data.set(false);
                            },
                            class: "flex-1 px-4 py-2 rounded-lg bg-red-600 hover:bg-red-700 text-sm font-medium transition-colors",
                            "Clear All"
                        }
                    }
                }
            }
        }
    }
}
