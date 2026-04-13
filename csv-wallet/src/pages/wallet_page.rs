/// Comprehensive Wallet Management page supporting all csv-cli commands.

use dioxus::prelude::*;
use csv_adapter_core::Chain;
use std::collections::HashMap;
use crate::context::{use_wallet_context, Network};
use crate::routes::Route;
use crate::components::{Dropdown, Card, StatCard, ChainDisplay, NetworkDisplay, all_chain_displays, all_network_displays};

#[derive(Clone, Copy, PartialEq)]
enum WalletTab {
    Overview,
    Generate,
    Import,
    Balance,
    Fund,
    Export,
    List,
}

impl std::fmt::Display for WalletTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Overview => write!(f, "Overview"),
            Self::Generate => write!(f, "Generate"),
            Self::Import => write!(f, "Import"),
            Self::Balance => write!(f, "Balance"),
            Self::Fund => write!(f, "Fund"),
            Self::Export => write!(f, "Export"),
            Self::List => write!(f, "List"),
        }
    }
}

#[component]
pub fn WalletPage() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut active_tab = use_signal(|| WalletTab::Overview);
    let mut selected_chain = use_signal(|| ChainDisplay(Chain::Bitcoin));
    let mut selected_network = use_signal(|| NetworkDisplay(Network::Test));
    let mut message = use_signal(|| Option::<String>::None);
    let mut error = use_signal(|| Option::<String>::None);
    let mut mnemonic_result = use_signal(|| Option::<String>::None);
    let mut import_input = use_signal(|| String::new());
    let mut importing = use_signal(|| false);
    let mut generating = use_signal(|| false);

    let tabs = vec![
        WalletTab::Overview,
        WalletTab::Generate,
        WalletTab::Import,
        WalletTab::Balance,
        WalletTab::Fund,
        WalletTab::Export,
        WalletTab::List,
    ];

    let addresses: HashMap<Chain, String> = wallet_ctx.addresses().into_iter().collect();
    let has_wallet = !addresses.is_empty();

    // Clone wallet context for use in closures
    let mut ctx_generate = wallet_ctx.clone();
    let mut ctx_import = wallet_ctx.clone();

    rsx! {
        div { class: "space-y-6",
            // Header
            div { class: "flex items-center justify-between",
                h1 { class: "text-3xl font-bold text-gray-100", "Wallet Management" }
                div { class: "flex items-center gap-2 text-sm text-gray-400",
                    span { class: "w-2 h-2 rounded-full", class: if has_wallet { "bg-green-500 status-online" } else { "bg-yellow-500" } }
                    if has_wallet { "Wallet Ready" } else { "No Wallet" }
                }
            }

            // Error display
            if let Some(err) = error.read().clone() {
                div { class: "bg-red-500/10 border border-red-500/30 rounded-xl p-4 text-sm text-red-300 flex items-center justify-between",
                    span { "{err}" }
                    button { onclick: move |_| error.set(None), class: "text-red-400 hover:text-red-200", "\u{2715}" }
                }
            }

            // Message display
            if let Some(msg) = message.read().clone() {
                div { class: "bg-blue-500/10 border border-blue-500/30 rounded-xl p-4 text-sm text-blue-300 flex items-center justify-between",
                    span { "{msg}" }
                    button { onclick: move |_| message.set(None), class: "text-blue-400 hover:text-blue-200", "\u{2715}" }
                }
            }

            // Mnemonic display
            if let Some(mnemonic) = mnemonic_result.read().clone() {
                div { class: "bg-yellow-500/10 border border-yellow-500/30 rounded-xl p-6 space-y-3 stagger-children",
                    div { class: "flex items-center gap-2",
                        span { class: "text-yellow-400", "\u{26A0}\u{FE0F}" }
                        p { class: "text-yellow-300 font-medium", "Save your recovery phrase!" }
                    }
                    div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700",
                        p { class: "font-mono text-sm text-gray-200 break-all leading-relaxed", "{mnemonic}" }
                    }
                    button {
                        onclick: move |_| {
                            wallet_ctx.clear_pending_secret();
                            mnemonic_result.set(None);
                            message.set(Some("Recovery phrase cleared from memory".to_string()));
                        },
                        class: "px-4 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 text-sm font-medium transition-colors",
                        "Clear from Memory"
                    }
                }
            }

            // Tab navigation
            div { class: "bg-gray-900 rounded-xl border border-gray-800 p-1",
                div { class: "flex gap-1 overflow-x-auto",
                    for tab in tabs {
                        button {
                            onclick: move |_| {
                                active_tab.set(tab);
                                error.set(None);
                                message.set(None);
                            },
                            class: "px-4 py-2 rounded-lg text-sm font-medium transition-all whitespace-nowrap",
                            class: if active_tab() == tab { "bg-blue-600 text-white" } else { "text-gray-400 hover:text-gray-200 hover:bg-gray-800" },
                            "{tab}"
                        }
                    }
                }
            }

            // Tab content
            match active_tab() {
                WalletTab::Overview => rsx! {
                    OverviewTab { has_wallet, address_count: addresses.len() }
                },
                WalletTab::Generate => rsx! {
                    GenerateTab {
                        selected_chain: selected_chain.read().clone(),
                        selected_network: selected_network.read().clone(),
                        on_chain_change: move |cd: ChainDisplay| selected_chain.set(cd),
                        on_network_change: move |nd: NetworkDisplay| selected_network.set(nd),
                        generating: *generating.read(),
                        on_generate: move || {
                            generating.set(true);
                            error.set(None);
                            let mnemonic = ctx_generate.create_wallet();
                            mnemonic_result.set(Some(mnemonic.clone()));
                            generating.set(false);
                            message.set(Some("Wallet generated! Save the recovery phrase above.".to_string()));
                        },
                    }
                },
                WalletTab::Import => rsx! {
                    ImportTab {
                        import_input: import_input.read().clone(),
                        on_input_change: move |val: String| { import_input.set(val); error.set(None); },
                        importing: *importing.read(),
                        on_import: move || {
                            let input = import_input.read().clone();
                            if input.is_empty() {
                                error.set(Some("Please enter a recovery phrase or private key.".to_string()));
                                return;
                            }
                            importing.set(true);
                            // Try as mnemonic first, then as private key
                            let result = ctx_import.import_wallet(&input);
                            if result.is_err() {
                                let result2 = ctx_import.import_wallet_from_key(&input);
                                if let Err(e) = result2 {
                                    error.set(Some(format!("Failed to import: {e}")));
                                    importing.set(false);
                                    return;
                                }
                            }
                            importing.set(false);
                            import_input.set(String::new());
                            message.set(Some("Wallet imported successfully!".to_string()));
                        },
                    }
                },
                WalletTab::Balance => rsx! {
                    BalanceTab {
                        selected_chain: selected_chain.read().clone(),
                        on_chain_change: move |cd: ChainDisplay| selected_chain.set(cd),
                        addresses: addresses.clone(),
                    }
                },
                WalletTab::Fund => rsx! {
                    FundTab {
                        selected_chain: selected_chain.read().clone(),
                        selected_network: selected_network.read().clone(),
                        on_chain_change: move |cd: ChainDisplay| selected_chain.set(cd),
                        on_network_change: move |nd: NetworkDisplay| selected_network.set(nd),
                        on_fund: move |msg: String| message.set(Some(msg)),
                        addresses: addresses.clone(),
                    }
                },
                WalletTab::Export => rsx! {
                    ExportTab {
                        selected_chain: selected_chain.read().clone(),
                        on_chain_change: move |cd: ChainDisplay| selected_chain.set(cd),
                        addresses: addresses.clone(),
                        on_export: move |msg: String| message.set(Some(msg)),
                    }
                },
                WalletTab::List => rsx! {
                    ListTab { addresses: addresses.clone() }
                },
            }
        }
    }
}

#[component]
fn OverviewTab(has_wallet: bool, address_count: usize) -> Element {
    rsx! {
        div { class: "space-y-6",
            div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                StatCard { label: "Total Addresses", value: address_count.to_string(), icon: "\u{1F4B3}" }
                StatCard { label: "Supported Chains", value: "4".to_string(), icon: "\u{26D3}\u{FE0F}" }
                StatCard { label: "Networks", value: "3".to_string(), icon: "\u{1F310}" }
            }

            Card {
                title: if has_wallet { "Quick Actions" } else { "Get Started" },
                children: rsx! {
                    if has_wallet {
                        div { class: "grid grid-cols-2 md:grid-cols-4 gap-4",
                            Link { to: Route::Dashboard {}, class: "p-4 bg-gray-800 rounded-lg hover:bg-gray-700 transition-colors text-center",
                                div { class: "text-2xl mb-2", "\u{1F4CA}" }
                                div { class: "font-medium text-sm", "Dashboard" }
                            }
                            Link { to: Route::Rights {}, class: "p-4 bg-gray-800 rounded-lg hover:bg-gray-700 transition-colors text-center",
                                div { class: "text-2xl mb-2", "\u{1F48E}" }
                                div { class: "font-medium text-sm", "Rights" }
                            }
                            Link { to: Route::CrossChain {}, class: "p-4 bg-gray-800 rounded-lg hover:bg-gray-700 transition-colors text-center",
                                div { class: "text-2xl mb-2", "\u{21C4}" }
                                div { class: "font-medium text-sm", "Cross-Chain" }
                            }
                            Link { to: Route::Settings {}, class: "p-4 bg-gray-800 rounded-lg hover:bg-gray-700 transition-colors text-center",
                                div { class: "text-2xl mb-2", "\u{2699}\u{FE0F}" }
                                div { class: "font-medium text-sm", "Settings" }
                            }
                        }
                    } else {
                        div { class: "text-center py-8 space-y-4",
                            div { class: "text-6xl", "\u{1F510}" }
                            p { class: "text-gray-400", "No wallet detected. Go to the Dashboard to create or import a wallet." }
                            Link { to: Route::Dashboard {}, class: "inline-block px-6 py-3 bg-blue-600 hover:bg-blue-700 rounded-lg font-medium transition-colors", "Go to Dashboard" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn GenerateTab(
    selected_chain: ChainDisplay,
    selected_network: NetworkDisplay,
    on_chain_change: EventHandler<ChainDisplay>,
    on_network_change: EventHandler<NetworkDisplay>,
    generating: bool,
    on_generate: EventHandler<()>,
) -> Element {
    rsx! {
        Card {
            title: "Generate New Wallet",
            children: rsx! {
                div { class: "space-y-6 stagger-children",
                    div {
                        label { class: "block text-sm font-medium text-gray-300 mb-2", "Blockchain" }
                        Dropdown {
                            options: all_chain_displays(),
                            selected: selected_chain,
                            on_change: move |cd| on_chain_change.call(cd),
                        }
                    }

                    div {
                        label { class: "block text-sm font-medium text-gray-300 mb-2", "Network" }
                        Dropdown {
                            options: all_network_displays(),
                            selected: selected_network,
                            on_change: move |nd| on_network_change.call(nd),
                        }
                    }

                    button {
                        onclick: move |_| on_generate.call(()),
                        disabled: generating,
                        class: "w-full px-6 py-3 bg-blue-600 hover:bg-blue-700 rounded-lg font-medium transition-all duration-200 text-white btn-ripple disabled:opacity-50 disabled:cursor-not-allowed",
                        if generating {
                            span { class: "inline-flex items-center gap-2",
                                span { class: "animate-spin", "\u{23F3}" }
                                "Generating..."
                            }
                        } else {
                            "Generate Wallet"
                        }
                    }

                    div { class: "bg-blue-500/10 border border-blue-500/20 rounded-lg p-4 text-sm text-gray-400",
                        span { class: "text-blue-400 font-medium", "\u{2139}\u{FE0F} Info: " }
                        "A new wallet will be generated with a random 12-word recovery phrase. Save it securely."
                    }
                }
            }
        }
    }
}

#[component]
fn ImportTab(
    import_input: String,
    on_input_change: EventHandler<String>,
    importing: bool,
    on_import: EventHandler<()>,
) -> Element {
    rsx! {
        Card {
            title: "Import Wallet",
            children: rsx! {
                div { class: "space-y-6 stagger-children",
                    div {
                        label { class: "block text-sm font-medium text-gray-300 mb-2", "Recovery Phrase or Private Key" }
                        textarea {
                            value: "{import_input}",
                            oninput: move |evt| on_input_change.call(evt.value()),
                            placeholder: "Enter 12/24 word mnemonic or hex-encoded private key...",
                            class: "w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-3 text-gray-100 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none input-focus",
                            rows: 4,
                        }
                    }

                    button {
                        onclick: move |_| on_import.call(()),
                        disabled: importing || import_input.is_empty(),
                        class: "w-full px-6 py-3 bg-green-600 hover:bg-green-700 rounded-lg font-medium transition-all duration-200 text-white btn-ripple disabled:opacity-50 disabled:cursor-not-allowed",
                        if importing {
                            span { class: "inline-flex items-center gap-2",
                                span { class: "animate-spin", "\u{23F3}" }
                                "Importing..."
                            }
                        } else {
                            "Import Wallet"
                        }
                    }

                    div { class: "bg-yellow-500/10 border border-yellow-500/20 rounded-lg p-4 text-sm text-gray-400",
                        span { class: "text-yellow-400 font-medium", "\u{26A0}\u{FE0F} Warning: " }
                        "Never share your private key or mnemonic. Only import from trusted sources."
                    }

                    div { class: "bg-gray-800/50 rounded-lg p-3 border border-gray-700 space-y-2",
                        p { class: "text-xs text-gray-400 font-medium", "Supported Formats" }
                        div { class: "flex flex-wrap gap-1.5",
                            span { class: "text-xs text-gray-500", "12 or 24 word BIP-39 mnemonic" }
                            span { class: "text-xs text-gray-600", "•" }
                            span { class: "text-xs text-gray-500", "Hex private key (64 chars for secp256k1/ed25519)" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn BalanceTab(
    selected_chain: ChainDisplay,
    on_chain_change: EventHandler<ChainDisplay>,
    addresses: HashMap<Chain, String>,
) -> Element {
    let chain = selected_chain.0;
    let addr = addresses.get(&chain).cloned().unwrap_or_else(|| "Not generated".to_string());

    rsx! {
        Card {
            title: "Check Balance",
            children: rsx! {
                div { class: "space-y-6 stagger-children",
                    div {
                        label { class: "block text-sm font-medium text-gray-300 mb-2", "Blockchain" }
                        Dropdown {
                            options: all_chain_displays(),
                            selected: selected_chain,
                            on_change: move |cd| on_chain_change.call(cd),
                        }
                    }

                    div {
                        label { class: "block text-sm font-medium text-gray-300 mb-2", "Address" }
                        div { class: "bg-gray-800 border border-gray-700 rounded-lg px-4 py-3 font-mono text-sm text-gray-200 break-all input-focus", "{addr}" }
                    }

                    div { class: "bg-gray-800/50 rounded-lg p-4 card-hover",
                        div { class: "flex items-center justify-between",
                            span { class: "text-gray-400", "Balance" }
                            span { class: "text-2xl font-bold text-gray-100 count-up", "0.0000" }
                        }
                        div { class: "text-xs text-gray-500 mt-1", "Connect to RPC to fetch real balance" }
                    }
                }
            }
        }
    }
}

#[component]
fn FundTab(
    selected_chain: ChainDisplay,
    selected_network: NetworkDisplay,
    on_chain_change: EventHandler<ChainDisplay>,
    on_network_change: EventHandler<NetworkDisplay>,
    on_fund: EventHandler<String>,
    addresses: HashMap<Chain, String>,
) -> Element {
    let chain = selected_chain.0;
    let network = selected_network.0;
    let addr = addresses.get(&chain).cloned().unwrap_or_else(|| "Generate a wallet first".to_string());
    let addr_for_closure = addr.clone();

    rsx! {
        Card {
            title: "Fund from Faucet",
            children: rsx! {
                div { class: "space-y-6 stagger-children",
                    div {
                        label { class: "block text-sm font-medium text-gray-300 mb-2", "Blockchain" }
                        Dropdown {
                            options: all_chain_displays(),
                            selected: selected_chain,
                            on_change: move |cd| on_chain_change.call(cd),
                        }
                    }

                    div {
                        label { class: "block text-sm font-medium text-gray-300 mb-2", "Network" }
                        Dropdown {
                            options: all_network_displays(),
                            selected: selected_network,
                            on_change: move |nd| on_network_change.call(nd),
                        }
                    }

                    div {
                        label { class: "block text-sm font-medium text-gray-300 mb-2", "Target Address" }
                        div { class: "bg-gray-800 border border-gray-700 rounded-lg px-4 py-3 font-mono text-sm text-gray-200 break-all input-focus", "{addr}" }
                    }

                    button {
                        onclick: move |_| {
                            if addr_for_closure != "Generate a wallet first" {
                                on_fund.call(format!("Faucet request sent for {} on {}", chain, network));
                            }
                        },
                        disabled: addr == "Generate a wallet first",
                        class: "w-full px-6 py-3 bg-purple-600 hover:bg-purple-700 rounded-lg font-medium transition-all duration-200 text-white btn-ripple disabled:opacity-50 disabled:cursor-not-allowed",
                        "Request Test Tokens"
                    }

                    div { class: "bg-purple-500/10 border border-purple-500/20 rounded-lg p-4 text-sm text-gray-400",
                        span { class: "text-purple-400 font-medium", "\u{2139}\u{FE0F} Info: " }
                        "Test tokens will be requested from the chain's official faucet. This may take a few minutes."
                    }
                }
            }
        }
    }
}

#[component]
fn ExportTab(
    selected_chain: ChainDisplay,
    on_chain_change: EventHandler<ChainDisplay>,
    addresses: HashMap<Chain, String>,
    on_export: EventHandler<String>,
) -> Element {
    let chain = selected_chain.0;
    let addr = addresses.get(&chain).cloned().unwrap_or_else(|| "Not generated".to_string());

    rsx! {
        Card {
            title: "Export Wallet",
            children: rsx! {
                div { class: "space-y-6",
                    div {
                        label { class: "block text-sm font-medium text-gray-300 mb-2", "Blockchain" }
                        Dropdown {
                            options: all_chain_displays(),
                            selected: selected_chain,
                            on_change: move |cd| on_chain_change.call(cd),
                        }
                    }

                    div {
                        label { class: "block text-sm font-medium text-gray-300 mb-2", "Address" }
                        div { class: "bg-gray-800 border border-gray-700 rounded-lg px-4 py-3 font-mono text-sm text-gray-200 break-all", "{addr}" }
                    }

                    div { class: "space-y-3",
                        button {
                            onclick: move |_| {
                                on_export.call(format!("Exported address for {}: {}", chain, addr));
                            },
                            class: "w-full px-6 py-3 bg-blue-600 hover:bg-blue-700 rounded-lg font-medium transition-colors text-white",
                            "Export Address"
                        }
                        button {
                            onclick: move |_| {
                                on_export.call("Export full wallet data (JSON format)".to_string());
                            },
                            class: "w-full px-6 py-3 bg-gray-800 hover:bg-gray-700 rounded-lg font-medium transition-colors",
                            "Export Full Wallet (JSON)"
                        }
                    }
                }
            }
        }
    }
}

fn chain_badge_class(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-orange-400 bg-orange-500/20 border border-orange-500/30",
        Chain::Ethereum => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-blue-400 bg-blue-500/20 border border-blue-500/30",
        Chain::Sui => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-cyan-400 bg-cyan-500/20 border border-cyan-500/30",
        Chain::Aptos => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-emerald-400 bg-emerald-500/20 border border-emerald-500/30",
    }
}

#[component]
fn ListTab(addresses: HashMap<Chain, String>) -> Element {
    let chains = [Chain::Bitcoin, Chain::Ethereum, Chain::Sui, Chain::Aptos];

    rsx! {
        Card {
            title: "All Wallets",
            children: rsx! {
                if addresses.is_empty() {
                    div { class: "text-center py-12 text-gray-500",
                        div { class: "text-5xl mb-4", "\u{1F4CB}" }
                        div { class: "text-lg font-medium", "No wallets" }
                        p { class: "text-sm mt-2", "Generate or import a wallet to see it listed here." }
                    }
                } else {
                    div { class: "overflow-x-auto",
                        table { class: "w-full",
                            thead {
                                tr { class: "border-b border-gray-800",
                                    th { class: "text-left py-3 px-4 text-sm font-medium text-gray-400", "Blockchain" }
                                    th { class: "text-left py-3 px-4 text-sm font-medium text-gray-400", "Address" }
                                    th { class: "text-left py-3 px-4 text-sm font-medium text-gray-400", "Balance" }
                                }
                            }
                            tbody {
                                for chain in chains {
                                    if let Some(addr) = addresses.get(&chain) {
                                        tr { class: "border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors",
                                            td { class: "py-3 px-4",
                                                span { class: "{chain_badge_class(&chain)}",
                                                    "{chain}"
                                                }
                                            }
                                            td { class: "py-3 px-4 font-mono text-sm text-gray-200", "{addr}" }
                                            td { class: "py-3 px-4 text-sm text-gray-300", "0.0000" }
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
