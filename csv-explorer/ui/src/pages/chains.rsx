/// Chain status page with detailed per-chain info.

use dioxus::prelude::*;

#[component]
pub fn Chains() -> Element {
    rsx! {
        div { class: "space-y-6",
            h1 { class: "text-2xl font-bold", "Chain Status" }

            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6",
                ChainCard {
                    name: "Bitcoin",
                    id: "bitcoin",
                    network: "mainnet",
                    status: "synced",
                    latest_block: 840_000,
                    rpc_url: "https://mempool.space/api",
                    sync_lag: 0,
                }
                ChainCard {
                    name: "Ethereum",
                    id: "ethereum",
                    network: "mainnet",
                    status: "synced",
                    latest_block: 19_500_000,
                    rpc_url: "https://eth.llamarpc.com",
                    sync_lag: 2,
                }
                ChainCard {
                    name: "Sui",
                    id: "sui",
                    network: "mainnet",
                    status: "syncing",
                    latest_block: 120_000_000,
                    rpc_url: "https://fullnode.mainnet.sui.io:443",
                    sync_lag: 5,
                }
                ChainCard {
                    name: "Aptos",
                    id: "aptos",
                    network: "mainnet",
                    status: "synced",
                    latest_block: 95_000_000,
                    rpc_url: "https://fullnode.mainnet.aptoslabs.com/v1",
                    sync_lag: 1,
                }
                ChainCard {
                    name: "Solana",
                    id: "solana",
                    network: "mainnet",
                    status: "synced",
                    latest_block: 250_000_000,
                    rpc_url: "https://api.mainnet-beta.solana.com",
                    sync_lag: 0,
                }
            }
        }
    }
}

#[component]
fn ChainCard(name: String, id: String, network: String, status: String, latest_block: u64, rpc_url: String, sync_lag: u64) -> Element {
    let status_color = match status.as_str() {
        "synced" => "bg-green-500",
        "syncing" => "bg-yellow-500 animate-pulse",
        "error" => "bg-red-500",
        _ => "bg-gray-500",
    };

    let chain_icon = match id.as_str() {
        "bitcoin" => "₿",
        "ethereum" => "Ξ",
        "sui" => "S",
        "aptos" => "A",
        "solana" => "◎",
        _ => "⬡",
    };

    rsx! {
        div { class: "bg-gray-900 rounded-xl border border-gray-800 p-6",
            // Header
            div { class: "flex items-center justify-between mb-6",
                div { class: "flex items-center gap-3",
                    span { class: "text-3xl", "{chain_icon}" }
                    div {
                        h2 { class: "font-semibold", "{name}" }
                        span { class: "text-xs text-gray-500 uppercase", "{network}" }
                    }
                }
                span { class: "w-3 h-3 rounded-full {status_color}" }
            }

            // Stats
            div { class: "space-y-3 text-sm",
                div { class: "flex justify-between",
                    span { class: "text-gray-400", "Status" }
                    span { class: "text-gray-200 capitalize", "{status}" }
                }
                div { class: "flex justify-between",
                    span { class: "text-gray-400", "Latest Block" }
                    span { class: "font-mono text-gray-200", "{latest_block}" }
                }
                div { class: "flex justify-between",
                    span { class: "text-gray-400", "Sync Lag" }
                    span {
                        class: if sync_lag == 0 { "text-green-400" } else { "text-yellow-400" },
                        "{sync_lag} blocks"
                    }
                }
                div { class: "flex justify-between",
                    span { class: "text-gray-400", "RPC" }
                    span { class: "font-mono text-xs text-gray-300 truncate max-w-[200px]", "{rpc_url}" }
                }
            }

            // RPC connectivity indicator
            div { class: "mt-4 pt-4 border-t border-gray-800 flex items-center gap-2 text-xs text-gray-500",
                span { class: "w-2 h-2 rounded-full bg-green-500" }
                "RPC connected"
            }
        }
    }
}
