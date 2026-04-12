/// Home / landing page with stats, recent activity, and chain status.

use dioxus::prelude::*;

use crate::routes;

#[component]
pub fn Home() -> Element {
    let stats = use_resource(move || async move {
        // Fetch stats from API
        fetch_stats().await
    });

    rsx! {
        div { class: "space-y-8",
            // Hero
            div { class: "text-center py-12",
                h1 { class: "text-4xl font-bold mb-4",
                    "Cross-Chain Sealed Verifiable Rights Explorer"
                }
                p { class: "text-gray-400 text-lg max-w-2xl mx-auto",
                    "Track, search, and analyze CSV rights, transfers, and seals across Bitcoin, Ethereum, Sui, Aptos, and Solana."
                }
            }

            // Stats cards
            div { class: "grid grid-cols-1 md:grid-cols-4 gap-4",
                StatCard { label: "Total Rights", value: stats.with(|s| s.as_ref().map(|s| s.total_rights.to_string()).unwrap_or_else(|| "...".to_string())), icon: "◆" }
                StatCard { label: "Total Transfers", value: stats.with(|s| s.as_ref().map(|s| s.total_transfers.to_string()).unwrap_or_else(|| "...".to_string())), icon: "⇄" }
                StatCard { label: "Active Seals", value: stats.with(|s| s.as_ref().map(|s| s.total_seals.to_string()).unwrap_or_else(|| "...".to_string())), icon: "🔒" }
                StatCard { label: "Contracts", value: stats.with(|s| s.as_ref().map(|s| s.total_contracts.to_string()).unwrap_or_else(|| "...".to_string())), icon: "📄" }
            }

            // Chain status cards
            div {
                h2 { class: "text-xl font-semibold mb-4", "Chain Status" }
                div { class: "grid grid-cols-1 md:grid-cols-5 gap-4",
                    ChainStatusCard { chain: "Bitcoin", status: "synced", block: 840_000_u64, sync_lag: 0_u64 }
                    ChainStatusCard { chain: "Ethereum", status: "synced", block: 19_500_000_u64, sync_lag: 2_u64 }
                    ChainStatusCard { chain: "Sui", status: "syncing", block: 120_000_000_u64, sync_lag: 5_u64 }
                    ChainStatusCard { chain: "Aptos", status: "synced", block: 95_000_000_u64, sync_lag: 1_u64 }
                    ChainStatusCard { chain: "Solana", status: "synced", block: 250_000_000_u64, sync_lag: 0_u64 }
                }
            }

            // Recent activity
            div {
                div { class: "flex items-center justify-between mb-4",
                    h2 { class: "text-xl font-semibold", "Recent Activity" }
                    Link { to: routes::Route::TransfersList {},
                        span { class: "text-blue-400 hover:text-blue-300 text-sm", "View all →" }
                    }
                }
                div { class: "bg-gray-900 rounded-xl border border-gray-800 overflow-hidden",
                    // Recent activity rows would be populated from API
                    div { class: "divide-y divide-gray-800",
                        ActivityRow { action: "Right Created", chain: "Bitcoin", id: "btc-right-a1b2c3...", time: "2 min ago" }
                        ActivityRow { action: "Transfer Completed", chain: "Ethereum → Sui", id: "eth-xfer-d4e5f6...", time: "5 min ago" }
                        ActivityRow { action: "Seal Consumed", chain: "Aptos", id: "aptos-seal-g7h8i9...", time: "12 min ago" }
                        ActivityRow { action: "Right Created", chain: "Solana", id: "sol-right-j0k1l2...", time: "18 min ago" }
                        ActivityRow { action: "Transfer In Progress", chain: "Sui → Aptos", id: "sui-xfer-m3n4o5...", time: "25 min ago" }
                    }
                }
            }

            // Quick search
            div { class: "bg-gray-900 rounded-xl border border-gray-800 p-6",
                h2 { class: "text-xl font-semibold mb-4", "Quick Search" }
                p { class: "text-gray-400 mb-4", "Search by Right ID, Transfer ID, Seal ID, or Owner Address" }
                div { class: "flex gap-2",
                    input {
                        r#type: "text",
                        placeholder: "Enter ID or address...",
                        class: "flex-1 bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                    }
                    button { class: "px-6 py-2 bg-blue-600 hover:bg-blue-700 rounded-lg font-medium transition-colors",
                        "Search"
                    }
                }
            }
        }
    }
}

#[component]
fn StatCard(label: String, value: String, icon: String) -> Element {
    rsx! {
        div { class: "bg-gray-900 rounded-xl border border-gray-800 p-6",
            div { class: "flex items-center justify-between mb-2",
                span { class: "text-2xl", "{icon}" }
            }
            div { class: "text-3xl font-bold mb-1", "{value}" }
            div { class: "text-gray-400 text-sm", "{label}" }
        }
    }
}

#[component]
fn ChainStatusCard(chain: String, status: String, block: u64, sync_lag: u64) -> Element {
    let status_color = match status.as_str() {
        "synced" => "bg-green-500",
        "syncing" => "bg-yellow-500",
        "error" => "bg-red-500",
        _ => "bg-gray-500",
    };

    rsx! {
        div { class: "bg-gray-900 rounded-xl border border-gray-800 p-4",
            div { class: "flex items-center gap-2 mb-3",
                span { class: "w-2 h-2 rounded-full {status_color}" }
                span { class: "font-medium", "{chain}" }
            }
            div { class: "text-sm text-gray-400 space-y-1",
                div { class: "flex justify-between",
                    span { "Block" }
                    span { class: "text-gray-200 font-mono text-xs", "{block}" }
                }
                div { class: "flex justify-between",
                    span { "Sync Lag" }
                    span { class: "text-gray-200", "{sync_lag} blocks" }
                }
            }
        }
    }
}

#[component]
fn ActivityRow(action: String, chain: String, id: String, time: String) -> Element {
    rsx! {
        div { class: "px-6 py-4 flex items-center justify-between hover:bg-gray-800/50 transition-colors",
            div { class: "flex items-center gap-4",
                div {
                    span { class: "text-sm font-medium text-blue-400", "{action}" }
                    div { class: "text-xs text-gray-500 mt-0.5", "{chain}" }
                }
            }
            div { class: "text-right",
                div { class: "font-mono text-sm text-gray-300", "{id}" }
                div { class: "text-xs text-gray-500", "{time}" }
            }
        }
    }
}

async fn fetch_stats() -> Option<shared::ExplorerStats> {
    // In production, fetch from API
    None
}
