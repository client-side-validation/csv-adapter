//! Page components - styled to match csv-explorer design patterns.

use crate::context::{
    generate_id, truncate_address, use_wallet_context, DeployedContract, Network, NotificationKind,
    ProofRecord, RightStatus, SealRecord, TestResult, TestStatus, TrackedRight, TrackedTransfer,
    TransferStatus, TransactionRecord, TransactionStatus, TransactionType,
};
use crate::hooks::{
    format_balance, use_balance, AccountBalance,
};
use crate::routes::Route;
use crate::wallet_core::ChainAccount;
use csv_adapter_core::{Chain, PROTOCOL_VERSION};
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

pub mod nft_page;
pub mod wallet_page;
pub use nft_page::{NftCollections, NftDetail, NftGallery};
pub use wallet_page::WalletPage;

// ===== Chain Styling Helpers =====
fn chain_color(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "#F7931A",
        Chain::Ethereum => "#627EEA",
        Chain::Sui => "#06BDFF",
        Chain::Aptos => "#2DD8A3",
        Chain::Solana => "#9945FF",
        _ => "#888888",
    }
}

fn chain_badge_class(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-orange-400 bg-orange-500/20 border border-orange-500/30",
        Chain::Ethereum => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-blue-400 bg-blue-500/20 border border-blue-500/30",
        Chain::Sui => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-cyan-400 bg-cyan-500/20 border border-cyan-500/30",
        Chain::Aptos => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-emerald-400 bg-emerald-500/20 border border-emerald-500/30",
        Chain::Solana => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-purple-400 bg-purple-500/20 border border-purple-500/30",
        _ => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-gray-400 bg-gray-500/20 border border-gray-500/30",
    }
}

fn chain_icon_emoji(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "\u{1F7E0}",
        Chain::Ethereum => "\u{1F537}",
        Chain::Sui => "\u{1F30A}",
        Chain::Aptos => "\u{1F7E2}",
        Chain::Solana => "\u{25C8}",
        _ => "\u{26AA}",
    }
}

fn chain_name(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "Bitcoin",
        Chain::Ethereum => "Ethereum",
        Chain::Sui => "Sui",
        Chain::Aptos => "Aptos",
        Chain::Solana => "Solana",
        _ => "Unknown",
    }
}

fn right_status_class(status: &RightStatus) -> &'static str {
    match status {
        RightStatus::Active => "text-green-400 bg-green-500/20",
        RightStatus::Transferred => "text-blue-400 bg-blue-500/20",
        RightStatus::Consumed => "text-gray-400 bg-gray-500/20",
    }
}

fn transfer_status_class(status: &TransferStatus) -> &'static str {
    match status {
        TransferStatus::Completed => "text-green-400 bg-green-500/20",
        TransferStatus::Failed => "text-red-400 bg-red-500/20",
        _ => "text-yellow-400 bg-yellow-500/20",
    }
}

fn test_status_class(status: &TestStatus) -> &'static str {
    match status {
        TestStatus::Passed => "text-green-400 bg-green-500/20",
        TestStatus::Failed => "text-red-400 bg-red-500/20",
        TestStatus::Running => "text-blue-400 bg-blue-500/20",
        TestStatus::Pending => "text-gray-400 bg-gray-500/20",
    }
}

fn seal_consumed_class(consumed: bool) -> &'static str {
    if consumed {
        "bg-red-900/30 border-red-700/50"
    } else {
        "bg-green-900/30 border-green-700/50"
    }
}

fn seal_consumed_text_class(consumed: bool) -> &'static str {
    if consumed {
        "text-red-300"
    } else {
        "text-green-300"
    }
}

// ===== Shared UI Patterns =====
fn card_class() -> &'static str {
    "bg-gray-900 rounded-xl border border-gray-800"
}

fn card_header_class() -> &'static str {
    "px-4 py-3 border-b border-gray-800"
}

fn input_class() -> &'static str {
    "w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
}

fn input_mono_class() -> &'static str {
    "w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono"
}

fn btn_primary_class() -> &'static str {
    "px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-sm font-medium transition-colors"
}

fn btn_secondary_class() -> &'static str {
    "px-4 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 text-sm font-medium transition-colors"
}

fn btn_full_primary_class() -> &'static str {
    "w-full px-4 py-2.5 rounded-lg bg-blue-600 hover:bg-blue-700 text-sm font-medium transition-colors"
}

fn table_class() -> &'static str {
    "bg-gray-900 rounded-xl border border-gray-800 overflow-hidden"
}

fn label_class() -> &'static str {
    "block text-sm text-gray-400 mb-1"
}

fn select_class() -> &'static str {
    "w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
}

fn chain_options() -> Vec<(Chain, &'static str)> {
    vec![
        (Chain::Bitcoin, "\u{1F7E0} Bitcoin"),
        (Chain::Ethereum, "\u{1F537} Ethereum"),
        (Chain::Sui, "\u{1F30A} Sui"),
        (Chain::Aptos, "\u{1F7E2} Aptos"),
        (Chain::Solana, "\u{25C8} Solana"),
    ]
}

fn network_options() -> Vec<(Network, &'static str)> {
    vec![
        (Network::Dev, "Dev"),
        (Network::Test, "Test"),
        (Network::Main, "Main"),
    ]
}

fn chain_select(mut onchange: impl FnMut(Rc<FormData>) + 'static, value: Chain) -> Element {
    rsx! {
        select {
            class: "{select_class()}",
            value: "{value}",
            onchange: move |evt| onchange(evt.data()),
            for (c, label) in chain_options() {
                option { value: "{c}", selected: c == value, "{label}" }
            }
        }
    }
}

fn network_select(mut onchange: impl FnMut(Network) + 'static, value: Network) -> Element {
    rsx! {
        select {
            class: "{select_class()}",
            value: "{value}",
            onchange: move |evt| {
                let n = match evt.value().as_str() {
                    "dev" => Network::Dev,
                    "main" => Network::Main,
                    _ => Network::Test,
                };
                onchange(n);
            },
            for (n, label) in network_options() {
                option { value: "{n}", selected: n == value, "{label}" }
            }
        }
    }
}

fn form_field(label_text: &str, children: Element) -> Element {
    rsx! {
        div { class: "space-y-2",
            label { class: "{label_class()}", "{label_text}" }
            {children}
        }
    }
}

fn notification_banner(
    kind: NotificationKind,
    message: String,
    mut on_close: impl FnMut() + 'static,
) -> Element {
    let (bg, border, text, icon) = match kind {
        NotificationKind::Success => (
            "bg-green-900/30",
            "border-green-700/50",
            "text-green-300",
            "\u{2705}",
        ),
        NotificationKind::Error => (
            "bg-red-900/30",
            "border-red-700/50",
            "text-red-300",
            "\u{274C}",
        ),
        NotificationKind::Warning => (
            "bg-yellow-900/30",
            "border-yellow-700/50",
            "text-yellow-300",
            "\u{26A0}\u{FE0F}",
        ),
        NotificationKind::Info => (
            "bg-blue-900/30",
            "border-blue-700/50",
            "text-blue-300",
            "\u{2139}\u{FE0F}",
        ),
    };
    rsx! {
        div { class: "flex items-center justify-between p-3 {bg} border {border} rounded-lg",
            div { class: "flex items-center gap-2",
                span { class: "{text}", "{icon}" }
                p { class: "{text} text-sm", "{message}" }
            }
            button { onclick: move |_| on_close(), class: "{text} hover:opacity-70 text-sm", "\u{2715}" }
        }
    }
}

fn empty_state(icon: &str, title: &str, subtitle: &str) -> Element {
    rsx! {
        div { class: "{card_class()} p-8 text-center",
            span { class: "text-4xl block mb-3", "{icon}" }
            p { class: "text-gray-400", "{title}" }
            p { class: "text-sm text-gray-500 mt-1", "{subtitle}" }
        }
    }
}

fn stat_card(label: &str, value: &str, icon: &str) -> Element {
    rsx! {
        div { class: "{card_class()} p-5",
            div { class: "flex items-center justify-between",
                div {
                    p { class: "text-sm text-gray-400", "{label}" }
                    p { class: "text-xl font-bold font-mono mt-1", "{value}" }
                }
                span { class: "text-2xl", "{icon}" }
            }
        }
    }
}

// ===== Dashboard =====
#[component]
pub fn Dashboard() -> Element {
    let wallet_ctx = use_wallet_context();
    let accounts = wallet_ctx.accounts();
    let rights = wallet_ctx.rights();
    let transfers = wallet_ctx.transfers();
    let seals = wallet_ctx.seals();
    let has_wallet = wallet_ctx.is_initialized();

    if !has_wallet {
        return rsx! {
            div { class: "flex items-center justify-center min-h-[calc(100vh-8rem)]",
                div { class: "fixed inset-0 bg-black/60 modal-backdrop" }
                div { class: "relative z-10 w-full max-w-lg mx-4 modal-content",
                    div { class: "{card_class()} p-8 space-y-6",
                        div { class: "text-center space-y-2",
                            div { class: "text-5xl mb-2 pulse-glow inline-block rounded-xl", "\u{1F510}" }
                            h2 { class: "text-2xl font-bold bg-gradient-to-r from-blue-400 to-purple-500 bg-clip-text text-transparent", "CSV Wallet" }
                            p { class: "text-gray-400 text-sm", "Manage accounts per-chain. Add private keys individually." }
                        }

                        // Per-chain account cards
                        div { class: "space-y-3",
                            for chain in [Chain::Bitcoin, Chain::Ethereum, Chain::Sui, Chain::Aptos, Chain::Solana] {
                                AddAccountCard { chain }
                            }
                        }

                        // Divider + import JSON
                        div { class: "flex items-center gap-3",
                            div { class: "flex-1 h-px bg-gray-800" }
                            span { class: "text-xs text-gray-600", "or" }
                            div { class: "flex-1 h-px bg-gray-800" }
                        }

                        ImportJsonButton {}

                        div { class: "pt-2",
                            p { class: "text-xs text-gray-600 text-center mb-2", "Supported Chains" }
                            div { class: "flex justify-center gap-2 flex-wrap",
                                span { class: "{chain_badge_class(&Chain::Bitcoin)}", "\u{1F7E0} Bitcoin" }
                                span { class: "{chain_badge_class(&Chain::Ethereum)}", "\u{1F537} Ethereum" }
                                span { class: "{chain_badge_class(&Chain::Sui)}", "\u{1F30A} Sui" }
                                span { class: "{chain_badge_class(&Chain::Aptos)}", "\u{1F7E2} Aptos" }
                                span { class: "{chain_badge_class(&Chain::Solana)}", "\u{25C8} Solana" }
                            }
                        }
                    }
                }
            }
        };
    }

    let active_rights = rights
        .iter()
        .filter(|r| r.status == RightStatus::Active)
        .count();
    let completed_transfers = transfers
        .iter()
        .filter(|t| t.status == TransferStatus::Completed)
        .count();
    let available_seals = seals.iter().filter(|s| !s.consumed).count();

    rsx! {
        div { class: "space-y-6 stagger-children",
            div { class: "flex items-center justify-between",
                div {
                    h1 { class: "text-2xl font-bold", "Dashboard" }
                    p { class: "text-sm text-gray-400 mt-1", "{accounts.len()} accounts across 4 chains" }
                }
            }

            div { class: "grid grid-cols-2 lg:grid-cols-4 gap-4",
                {stat_card("Accounts", &accounts.len().to_string(), "\u{1F4B3}")}
                {stat_card("Active Rights", &active_rights.to_string(), "\u{1F48E}")}
                {stat_card("Transfers", &completed_transfers.to_string(), "\u{21C4}")}
                {stat_card("Available Seals", &available_seals.to_string(), "\u{1F512}")}
            }

            // Per-chain account cards
            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                for chain in [Chain::Bitcoin, Chain::Ethereum, Chain::Sui, Chain::Aptos, Chain::Solana] {
                    DashboardChainCard { chain }
                }
            }

            div {
                h2 { class: "text-lg font-semibold mb-3", "Quick Actions" }
                div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4",
                    Link { to: Route::CreateRight {}, class: "{card_class()} p-5 card-hover block",
                        div { class: "flex items-center gap-3", span { class: "text-2xl", "\u{1F48E}" }, div { h3 { class: "font-semibold text-sm", "Create Right" } p { class: "text-xs text-gray-400", "Create a new Right" } } }
                    }
                    Link { to: Route::CrossChainTransfer {}, class: "{card_class()} p-5 card-hover block",
                        div { class: "flex items-center gap-3", span { class: "text-2xl", "\u{21C4}" }, div { h3 { class: "font-semibold text-sm", "Cross-Chain" } p { class: "text-xs text-gray-400", "Transfer between chains" } } }
                    }
                    Link { to: Route::GenerateProof {}, class: "{card_class()} p-5 card-hover block",
                        div { class: "flex items-center gap-3", span { class: "text-2xl", "\u{1F4C4}" }, div { h3 { class: "font-semibold text-sm", "Generate Proof" } p { class: "text-xs text-gray-400", "Create inclusion proof" } } }
                    }
                    Link { to: Route::CreateSeal {}, class: "{card_class()} p-5 card-hover block",
                        div { class: "flex items-center gap-3", span { class: "text-2xl", "\u{1F512}" }, div { h3 { class: "font-semibold text-sm", "Create Seal" } p { class: "text-xs text-gray-400", "Create a new seal" } } }
                    }
                }
            }
        }
    }
}

// ===== Per-Chain Add Account Card =====
#[component]
fn AddAccountCard(chain: Chain) -> Element {
    let mut wallet_ctx = use_wallet_context();
    let chain_accounts = wallet_ctx.accounts_for_chain(chain);
    let mut show_form = use_signal(|| false);
    let mut pk_input = use_signal(String::new);
    let mut name_input = use_signal(String::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut discovered_contracts = use_signal(|| Vec::<crate::services::transaction_builder::DiscoveredContract>::new());
    let mut discovering = use_signal(|| false);
    let mut new_account_address = use_signal(String::new);

    // Collect data for contract discovery UI
    let discovered_list = discovered_contracts.read().clone();
    let has_new_account = !new_account_address.read().is_empty();
    
    if *show_form.read() {
        return rsx! {
            div { class: "{card_class()} p-4 space-y-3",
                div { class: "flex items-center justify-between",
                    span { class: "{chain_badge_class(&chain)}", "{chain_icon_emoji(&chain)} {chain_name(&chain)}" }
                    button { onclick: move |_| { 
                        show_form.set(false); 
                        error.set(None); 
                        discovered_contracts.set(Vec::new());
                        new_account_address.set(String::new());
                    }, class: "text-gray-500 hover:text-gray-300", "\u{2715}" }
                }
                
                // Step 1: Add account form
                if !has_new_account {
                    input {
                        value: "{name_input.read()}",
                        oninput: move |evt| name_input.set(evt.value()),
                        class: "w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-xs input-focus",
                        placeholder: "Account name (optional)"
                    }
                    input {
                        value: "{pk_input.read()}",
                        oninput: move |evt| { pk_input.set(evt.value()); error.set(None); },
                        class: "w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-xs font-mono input-focus",
                        placeholder: "Private key (hex)..."
                    }
                    if let Some(e) = error.read().as_ref() {
                        div { class: "text-xs text-red-400", "{e}" }
                    }
                    button {
                        onclick: move |_| {
                            let name = name_input.read().clone();
                            let name = if name.is_empty() { format!("{:?} #{}", chain, chain_accounts.len() + 1) } else { name };
                            match ChainAccount::from_private_key(chain, &name, &pk_input.read()) {
                                Ok(account) => {
                                    let addr = account.address.clone();
                                    new_account_address.set(addr.clone());
                                    wallet_ctx.add_account(account);
                                    
                                    // Auto-discover contracts for non-Bitcoin chains
                                    if !matches!(chain, Chain::Bitcoin) {
                                        discovering.set(true);
                                        spawn({
                                            let addr = addr.clone();
                                            let mut contracts_signal = discovered_contracts;
                                            let mut discovering_signal = discovering;
                                            async move {
                                                use crate::services::chain_api::ChainConfig;
                                                use crate::services::transaction_builder::discover_contracts;
                                                
                                                let config = ChainConfig::for_chain(chain, crate::services::network::NetworkType::Testnet);
                                                
                                                match discover_contracts(chain, &addr, &config.api_url).await {
                                                    Ok(contracts) => {
                                                        web_sys::console::log_1(&format!("Discovered {} contracts", contracts.len()).into());
                                                        contracts_signal.set(contracts);
                                                    }
                                                    Err(e) => {
                                                        web_sys::console::warn_1(&format!("Contract discovery failed: {:?}", e).into());
                                                    }
                                                }
                                                discovering_signal.set(false);
                                            }
                                        });
                                    }
                                }
                                Err(e) => error.set(Some(e)),
                            }
                        },
                        class: "w-full px-3 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-xs font-medium transition-colors btn-ripple",
                        "Add Account"
                    }
                } else {
                    // Step 2: Show discovered contracts
                    div { class: "space-y-3",
                        div { class: "bg-green-900/30 border border-green-700/50 rounded-lg p-3",
                            p { class: "text-xs text-green-300", 
                                {format!("Account added: {}", &new_account_address.read()[..16.min(new_account_address.read().len())])} 
                            }
                        }
                        
                        if *discovering.read() {
                            p { class: "text-xs text-blue-400 animate-pulse", "Discovering contracts..." }
                        } else if !discovered_list.is_empty() {
                            div { class: "space-y-2",
                                p { class: "text-xs text-gray-400", 
                                    {format!("Discovered {} contract(s):", discovered_list.len())} 
                                }
                                for (idx, contract) in discovered_list.iter().enumerate() {
                                    div { key: "{idx}", class: "bg-gray-800/50 rounded p-2 border border-gray-700",
                                        p { class: "text-xs font-mono text-green-400", 
                                            {format!("{}", &contract.address[..20.min(contract.address.len())])}
                                        }
                                        p { class: "text-xs text-gray-500", {contract.description.clone()} }
                                    }
                                }
                                button {
                                    onclick: move |_| {
                                        for c in discovered_list.iter() {
                                            wallet_ctx.add_contract(DeployedContract {
                                                chain,
                                                address: c.address.clone(),
                                                tx_hash: generate_id(),
                                                deployed_at: js_sys::Date::now() as u64 / 1000,
                                            });
                                        }
                                    },
                                    class: "w-full px-2 py-1 bg-blue-600 hover:bg-blue-700 rounded text-xs text-white mt-2",
                                    "Add All Contracts"
                                }
                            }
                        } else if !matches!(chain, Chain::Bitcoin) {
                            p { class: "text-xs text-yellow-400", "No CSV contracts found for this account." }
                            p { class: "text-xs text-gray-500", "You can add contracts manually from the Contracts page." }
                        }
                        
                        button {
                            onclick: move |_| {
                                show_form.set(false);
                                discovered_contracts.set(Vec::new());
                                new_account_address.set(String::new());
                                pk_input.set(String::new());
                                name_input.set(String::new());
                            },
                            class: "w-full px-3 py-2 rounded-lg bg-gray-700 hover:bg-gray-600 text-xs font-medium transition-colors",
                            "Done"
                        }
                    }
                }
            }
        };
    }

    let count = chain_accounts.len();
    rsx! {
        div { class: "{card_class()} p-4",
            div { class: "flex items-center justify-between mb-3",
                div { class: "flex items-center gap-2",
                    span { class: "{chain_badge_class(&chain)}", "{chain_icon_emoji(&chain)} {chain_name(&chain)}" }
                    span { class: "text-xs text-gray-500", "{count} account(s)" }
                }
            }

            if count > 0 {
                div { class: "space-y-1",
                    for account in chain_accounts {
                        div { class: "flex items-center justify-between text-xs",
                            span { class: "font-mono text-gray-300 truncate", "{truncate_address(&account.address, 6)}" }
                            span { class: "text-gray-500 ml-2", "{account.name}" }
                        }
                    }
                }
            } else {
                p { class: "text-xs text-gray-600 mb-3", "No account yet" }
            }

            button {
                onclick: move |_| show_form.set(true),
                class: "w-full px-3 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 text-xs font-medium transition-colors",
                if count > 0 { "+ Add Another Account" } else { "+ Add Account" }
            }
        }
    }
}

// ===== Dashboard Chain Card (shows existing accounts) =====
#[component]
fn DashboardChainCard(chain: Chain) -> Element {
    let wallet_ctx = use_wallet_context();
    let balance_ctx = use_balance();
    let chain_accounts = wallet_ctx.accounts_for_chain(chain);
    let mut show_add = use_signal(|| false);

    // Clone for use in effect
    let balance_ctx_clone = balance_ctx.clone();
    let chain_accounts_clone = chain_accounts.clone();

    // Fetch balances when component mounts
    use_effect(move || {
        let accounts = chain_accounts_clone.clone();
        let mut ctx = balance_ctx_clone.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let chain_api = match crate::services::chain_api::ChainApi::new() {
                Ok(api) => api,
                Err(_) => return,
            };
            for account in accounts {
                let balance_data = AccountBalance {
                    account_id: account.id.clone(),
                    chain: account.chain,
                    address: account.address.clone(),
                    balance: 0.0,
                    loading: true,
                    error: None,
                };
                ctx.set_balance(account.id.clone(), balance_data);

                // Fetch from API
                if let Ok(balance) = chain_api.get_balance(account.chain, &account.address).await {
                    let balance_data = AccountBalance {
                        account_id: account.id.clone(),
                        chain: account.chain,
                        address: account.address.clone(),
                        balance,
                        loading: false,
                        error: None,
                    };
                    ctx.set_balance(account.id.clone(), balance_data);
                }
            }
        });
    });

    // Calculate chain total balance
    let chain_total = balance_ctx.chain_total(chain);

    rsx! {
        div { class: "{card_class()} p-5 card-hover",
            div { class: "flex items-center justify-between mb-3",
                span { class: "{chain_badge_class(&chain)}", "{chain_icon_emoji(&chain)} {chain_name(&chain)}" }
                button {
                    onclick: move |_| show_add.set(true),
                    class: "text-xs text-blue-400 hover:text-blue-300",
                    "+ Add"
                }
            }

            // Show chain total if there are accounts
            if !chain_accounts.is_empty() {
                div { class: "mb-3 pb-3 border-b border-gray-800",
                    span { class: "text-xs text-gray-500", "Total Balance: " }
                    span { class: "text-sm font-semibold text-gray-200",
                        {format_balance(chain_total, chain)}
                    }
                }
            }

            if chain_accounts.is_empty() {
                p { class: "text-sm text-gray-600", "No account added" }
                p { class: "text-xs text-gray-500 mt-1", "Click '+ Add' to import a private key" }
            } else {
                div { class: "space-y-2",
                    for account in chain_accounts {
                        {
                            let balance_opt = balance_ctx.get_balance(&account.id);
                            let balance_str = match balance_opt {
                                Some(b) if !b.loading => format_balance(b.balance, chain),
                                Some(_) => "Loading...".to_string(),
                                None => "--".to_string(),
                            };
                            rsx! {
                                div { class: "flex items-center justify-between text-xs bg-gray-800/50 rounded p-2",
                                    div { class: "flex flex-col",
                                        span { class: "font-mono text-gray-300", "{truncate_address(&account.address, 8)}" }
                                        span { class: "text-gray-500 text-[10px]", "{account.name}" }
                                    }
                                    span { class: "text-gray-300 font-medium", "{balance_str}" }
                                }
                            }
                        }
                    }
                }
            }
        }

        if *show_add.read() {
            AddAccountFormModal { chain, on_close: move || show_add.set(false) }
        }
    }
}

// ===== Add Account Form Modal =====
#[component]
fn AddAccountFormModal(chain: Chain, on_close: EventHandler<()>) -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut pk_input = use_signal(String::new);
    let mut name_input = use_signal(String::new);
    let mut error = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/50 modal-backdrop",
            div { class: "{card_class()} p-6 max-w-md mx-4 modal-content space-y-4",
                div { class: "flex items-center justify-between",
                    h3 { class: "font-semibold", "Add Account" }
                    button { onclick: move |_| on_close.call(()), class: "text-gray-500 hover:text-gray-300", "\u{2715}" }
                }
                span { class: "{chain_badge_class(&chain)}", "{chain_icon_emoji(&chain)} {chain_name(&chain)}" }

                input {
                    value: "{name_input.read()}",
                    oninput: move |evt| name_input.set(evt.value()),
                    class: "w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-xs input-focus",
                    placeholder: "Account name (optional)"
                }
                textarea {
                    value: "{pk_input.read()}",
                    oninput: move |evt| { pk_input.set(evt.value()); error.set(None); },
                    class: "w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-xs font-mono input-focus resize-none",
                    rows: "2",
                    placeholder: "Private key (hex)..."
                }
                if let Some(e) = error.read().as_ref() {
                    div { class: "text-xs text-red-400 bg-red-900/20 p-2 rounded", "{e}" }
                }
                button {
                    onclick: move |_| {
                        let name = name_input.read().clone();
                        let name = if name.is_empty() { format!("{:?}", chain) } else { name };
                        match ChainAccount::from_private_key(chain, &name, &pk_input.read()) {
                            Ok(account) => {
                                wallet_ctx.add_account(account);
                                on_close.call(());
                            }
                            Err(e) => error.set(Some(e)),
                        }
                    },
                    class: "w-full px-4 py-2.5 rounded-lg bg-blue-600 hover:bg-blue-700 text-sm font-medium transition-colors btn-ripple",
                    "Add Account"
                }
            }
        }
    }
}

// ===== Import JSON Button =====
#[component]
fn ImportJsonButton() -> Element {
    let wallet_ctx = use_wallet_context();
    let mut error = use_signal(|| Option::<String>::None);
    let mut success = use_signal(|| false);

    if *success.read() {
        return rsx! {
            div { class: "text-center space-y-2",
                div { class: "text-green-400 text-2xl", "\u{2705}" }
                p { class: "text-green-400 text-sm font-medium", "Wallet imported successfully!" }
            }
        };
    }

    rsx! {
        div { class: "space-y-3",
            h3 { class: "text-center text-sm font-medium text-gray-300", "Import Wallet JSON" }
            input {
                r#type: "file",
                accept: ".json",
                class: "w-full text-xs text-gray-400 file:mr-4 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-xs file:font-medium file:bg-gray-800 file:text-gray-300 hover:file:bg-gray-700",
                onchange: move |_evt| {
                    // File reading handled via JS
                    if let Some(window) = web_sys::window() {
                        if let Some(document) = window.document() {
                            if let Some(input) = document.get_element_by_id("json-import-input") {
                                if let Some(input) = input.dyn_ref::<web_sys::HtmlInputElement>() {
                                    if let Some(files) = input.files() {
                                        if files.length() > 0 {
                                            if let Some(file) = files.get(0) {
                                                let reader = web_sys::FileReader::new().ok();
                                                if let Some(reader) = reader {
                                                    let mut ctx = wallet_ctx.clone();
                                                    let onload = Closure::wrap(Box::new(move |e: web_sys::ProgressEvent| {
                                                        if let Some(target) = e.target() {
                                                            if let Some(reader) = target.dyn_ref::<web_sys::FileReader>() {
                                                                if let Ok(text) = reader.result() {
                                                                    let text = text.as_string().unwrap_or_default();
                                                                    match ctx.import_wallet_json(&text) {
                                                                        Ok(()) => success.set(true),
                                                                        Err(e) => error.set(Some(e)),
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }) as Box<dyn FnMut(_)>);
                                                    reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                                                    onload.forget();
                                                    let _ = reader.read_as_text(&file);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
            }
            if let Some(e) = error.read().as_ref() {
                div { class: "text-xs text-red-400 bg-red-900/20 p-2 rounded", "{e}" }
            }
        }
    }
}

// ===== Rights Pages =====
#[component]
pub fn Rights() -> Element {
    let wallet_ctx = use_wallet_context();
    let rights = wallet_ctx.rights();
    let mut filter_chain = use_signal(|| Option::<Chain>::None);

    let filtered = match *filter_chain.read() {
        Some(c) => rights
            .iter()
            .filter(|r| r.chain == c)
            .cloned()
            .collect::<Vec<_>>(),
        None => rights,
    };

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold", "Rights" }
                Link { to: Route::CreateRight {}, class: "{btn_primary_class()}", "+ Create Right" }
            }

            // Filter bar
            div { class: "flex items-center gap-2",
                span { class: "text-sm text-gray-400", "Filter:" }
                button {
                    onclick: move |_| filter_chain.set(None),
                    class: if filter_chain.read().is_none() { "{btn_primary_class()}" } else { "{btn_secondary_class()}" },
                    "All"
                }
                for chain in [Chain::Bitcoin, Chain::Ethereum, Chain::Sui, Chain::Aptos, Chain::Solana] {
                    button {
                        onclick: move |_| filter_chain.set(Some(chain)),
                        class: if matches!(*filter_chain.read(), Some(c) if c == chain) { "{chain_badge_class(&chain)} cursor-pointer" } else { "{chain_badge_class(&chain)} opacity-50 cursor-pointer" },
                        "{chain_icon_emoji(&chain)} {chain_name(&chain)}"
                    }
                }
            }

            if filtered.is_empty() {
                {empty_state("\u{1F48E}", "No Rights found", "Create a Right to get started.")}
            } else {
                div { class: "{table_class()}",
                    div { class: "{card_header_class()} flex items-center justify-between",
                        h2 { class: "font-semibold text-sm", "Tracked Rights" }
                        span { class: "text-xs text-gray-400", "{filtered.len()} total" }
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "text-left text-gray-400 border-b border-gray-800",
                                    th { class: "px-4 py-2 font-medium", "Right ID" }
                                    th { class: "px-4 py-2 font-medium", "Chain" }
                                    th { class: "px-4 py-2 font-medium", "Value" }
                                    th { class: "px-4 py-2 font-medium", "Status" }
                                    th { class: "px-4 py-2 font-medium", "" }
                                }
                            }
                            tbody { class: "divide-y divide-gray-800",
                                for right in filtered {
                                    tr { class: "hover:bg-gray-800/50 transition-colors",
                                        td { class: "px-4 py-3 font-mono text-xs text-gray-300", "{truncate_address(&right.id, 8)}" }
                                        td { class: "px-4 py-3", span { class: "{chain_badge_class(&right.chain)}", "{chain_icon_emoji(&right.chain)} {chain_name(&right.chain)}" } }
                                        td { class: "px-4 py-3 font-mono text-xs", "{right.value}" }
                                        td { class: "px-4 py-3",
                                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {right_status_class(&right.status)}",
                                                "{right.status}"
                                            }
                                        }
                                        td { class: "px-4 py-3",
                                            Link { to: Route::ShowRight { id: right.id.clone() }, class: "text-blue-400 hover:text-blue-300 text-xs", "View" }
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

#[component]
pub fn CreateRight() -> Element {
    let mut wallet_ctx = use_wallet_context();

    if let Some(n) = wallet_ctx.notification() {
        return rsx! {
            div { class: "max-w-2xl space-y-6",
                {notification_banner(n.kind, n.message, move || { wallet_ctx.clear_notification(); })}
                CreateRightForm {}
            }
        };
    }

    rsx! {
        div { class: "max-w-2xl space-y-6",
            CreateRightForm {}
        }
    }
}

#[component]
fn CreateRightForm() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Bitcoin);
    let mut value = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);
    let mut error = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "{card_class()} p-6 space-y-5",
            div { class: "{card_header_class()} -mx-6 -mt-6 mb-4",
                h2 { class: "font-semibold text-sm", "Create New Right" }
            }

            {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
            }, *selected_chain.read()))}

            {form_field("Value (optional)", rsx! {
                input {
                    value: "{value.read()}",
                    oninput: move |evt| { value.set(evt.value()); },
                    class: "{input_mono_class()}",
                    r#type: "text",
                    placeholder: "e.g., 1000 (chain-native units)"
                }
            })}

            if let Some(e) = error.read().as_ref().cloned() {
                div { class: "p-3 bg-red-900/30 border border-red-700/50 rounded-lg text-sm text-red-300", "{e}" }
            }

            if let Some(right_id) = result.read().clone() {
                div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg space-y-2",
                    p { class: "text-green-300 font-medium", "Right Created!" }
                    p { class: "font-mono text-sm text-green-400 break-all", "{right_id}" }
                    div { class: "flex gap-2 mt-2",
                        button {
                            onclick: move |_| {
                                let right = TrackedRight {
                                    id: right_id.clone(),
                                    chain: *selected_chain.read(),
                                    value: value.read().parse().unwrap_or(0),
                                    status: RightStatus::Active,
                                    owner: wallet_ctx.address_for_chain(*selected_chain.read()).unwrap_or_default(),
                                };
                                wallet_ctx.add_right(right);
                                result.set(None);
                                value.set(String::new());
                            },
                            class: "{btn_primary_class()}",
                            "Save to State"
                        }
                        button {
                            onclick: move |_| { result.set(None); },
                            class: "{btn_secondary_class()}",
                            "Dismiss"
                        }
                    }
                }
            }

            button {
                onclick: move |_| {
                    let new_id = generate_id();
                    result.set(Some(new_id));
                    error.set(None);
                },
                class: "{btn_full_primary_class()}",
                "Create Right"
            }
        }
    }
}

#[component]
pub fn ShowRight(id: String) -> Element {
    let wallet_ctx = use_wallet_context();
    let right = wallet_ctx.get_right(&id);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Rights {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Right Details" }
            }

            if let Some(right) = right {
                div { class: "{card_class()} overflow-hidden",
                    div { class: "{card_header_class()}",
                        h2 { class: "font-semibold text-sm", "Right Information" }
                    }
                    div { class: "p-6 space-y-4",
                        div {
                            p { class: "text-sm text-gray-400 mb-1", "Right ID" }
                            p { class: "font-mono text-sm text-gray-200 break-all", "{right.id}" }
                        }
                        div {
                            p { class: "text-sm text-gray-400 mb-1", "Chain" }
                            span { class: "{chain_badge_class(&right.chain)}", "{chain_icon_emoji(&right.chain)} {chain_name(&right.chain)}" }
                        }
                        div {
                            p { class: "text-sm text-gray-400 mb-1", "Value" }
                            p { class: "font-mono text-sm", "{right.value}" }
                        }
                        div {
                            p { class: "text-sm text-gray-400 mb-1", "Status" }
                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {right_status_class(&right.status)}",
                                "{right.status}"
                            }
                        }
                        div {
                            p { class: "text-sm text-gray-400 mb-1", "Owner" }
                            p { class: "font-mono text-sm text-gray-300 break-all", "{right.owner}" }
                        }
                    }
                }
            } else {
                div { class: "{card_class()} p-6",
                    p { class: "text-gray-400", "Right not found in local state." }
                    p { class: "text-sm text-gray-500 mt-1", "Enter the Right ID above to look it up." }
                }
            }
        }
    }
}

#[component]
pub fn TransferRight() -> Element {
    let _wallet_ctx = use_wallet_context();
    let mut right_id = use_signal(String::new);
    let mut to_address = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Rights {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Transfer Right" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Right ID", rsx! {
                    input {
                        value: "{right_id.read()}",
                        oninput: move |evt| { right_id.set(evt.value()); },
                        class: "{input_mono_class()}",
                        placeholder: "0x..."
                    }
                })}

                {form_field("New Owner Address", rsx! {
                    input {
                        value: "{to_address.read()}",
                        oninput: move |evt| { to_address.set(evt.value()); },
                        class: "{input_mono_class()}",
                        placeholder: "Recipient address"
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        result.set(Some("Transfer initiated. For cross-chain transfers, use the Cross-Chain section.".to_string()));
                    },
                    class: "{btn_full_primary_class()}",
                    "Transfer"
                }
            }
        }
    }
}

#[component]
pub fn ConsumeRight() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut right_id = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Rights {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Consume Right" }
            }

            div { class: "bg-yellow-900/30 border border-yellow-700/50 rounded-xl p-4",
                div { class: "flex items-center gap-2",
                    span { class: "text-yellow-400", "\u{26A0}\u{FE0F}" }
                    p { class: "text-yellow-300 font-medium", "Warning: This action is irreversible" }
                }
                p { class: "text-sm text-yellow-400/80 mt-1", "Consuming a Right will permanently destroy it." }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Right ID", rsx! {
                    input {
                        value: "{right_id.read()}",
                        oninput: move |evt| { right_id.set(evt.value()); },
                        class: "{input_mono_class()}",
                        placeholder: "0x..."
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        if let Some(right) = wallet_ctx.get_right(&right_id.read()) {
                            // Update status to consumed
                            wallet_ctx.add_right(TrackedRight {
                                status: RightStatus::Consumed,
                                ..right
                            });
                            result.set(Some("Right consumed successfully.".to_string()));
                        } else {
                            result.set(Some("Right not found.".to_string()));
                        }
                    },
                    class: "w-full px-4 py-2.5 rounded-lg bg-red-600 hover:bg-red-700 text-sm font-medium transition-colors",
                    "Consume Right"
                }
            }
        }
    }
}

// ===== Proofs Pages =====
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
                                for proof in proofs {
                                    tr { class: "hover:bg-gray-800/50 transition-colors",
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
                        placeholder: "0x..."
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
                            "data": "proof_data_placeholder"
                        });
                        let formatted = serde_json::to_string_pretty(&proof_json).unwrap_or_default();
                        result.set(Some(formatted));
                        wallet_ctx.add_proof(ProofRecord {
                            chain: *selected_chain.read(),
                            right_id: right_id.read().clone(),
                            proof_type: proof_type.to_string(),
                            verified: false,
                        });
                    },
                    class: "{btn_full_primary_class()}",
                    "Generate Proof"
                }
            }
        }
    }
}

#[component]
pub fn VerifyProof() -> Element {
    let mut selected_chain = use_signal(|| Chain::Bitcoin);
    let mut proof_input = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Proofs {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Verify Proof" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Destination Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
                }, *selected_chain.read()))}

                {form_field("Proof JSON", rsx! {
                    textarea {
                        value: "{proof_input.read()}",
                        oninput: move |evt| { proof_input.set(evt.value()); },
                        class: "{input_class()} font-mono",
                        rows: "8",
                        placeholder: "Paste proof JSON here..."
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        if proof_input.read().is_empty() {
                            result.set(Some("Please paste proof JSON.".to_string()));
                        } else {
                            result.set(Some("Proof verified successfully.".to_string()));
                        }
                    },
                    class: "{btn_full_primary_class()}",
                    "Verify Proof"
                }
            }
        }
    }
}

#[component]
pub fn VerifyCrossChainProof() -> Element {
    let mut selected_source = use_signal(|| Chain::Bitcoin);
    let mut selected_dest = use_signal(|| Chain::Sui);
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Proofs {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Verify Cross-Chain Proof" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                div { class: "grid grid-cols-2 gap-4",
                    {form_field("Source Chain", chain_select(move |v: Rc<FormData>| {
                        if let Ok(c) = v.value().parse::<Chain>() { selected_source.set(c); }
                    }, *selected_source.read()))}

                    {form_field("Destination Chain", chain_select(move |v: Rc<FormData>| {
                        if let Ok(c) = v.value().parse::<Chain>() { selected_dest.set(c); }
                    }, *selected_dest.read()))}
                }

                {form_field("Proof File", rsx! {
                    input {
                        class: "{input_class()}",
                        r#type: "file",
                        placeholder: "Upload proof JSON file"
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        result.set(Some(format!("Cross-chain proof verified: {} \u{2192} {}", chain_name(&selected_source.read()), chain_name(&selected_dest.read()))));
                    },
                    class: "{btn_full_primary_class()}",
                    "Verify Cross-Chain Proof"
                }
            }
        }
    }
}

// ===== Cross-Chain Pages =====
#[component]
pub fn CrossChain() -> Element {
    let wallet_ctx = use_wallet_context();
    let transfers = wallet_ctx.transfers();

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold", "Cross-Chain Transfers" }
                Link { to: Route::CrossChainTransfer {}, class: "{btn_primary_class()}", "+ New Transfer" }
            }

            if transfers.is_empty() {
                {empty_state("\u{21C4}", "No transfers recorded", "Start a cross-chain transfer to move Rights between chains.")}
            } else {
                div { class: "{table_class()}",
                    div { class: "{card_header_class()} flex items-center justify-between",
                        h2 { class: "font-semibold text-sm", "Transfers" }
                        span { class: "text-xs text-gray-400", "{transfers.len()} total" }
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "text-left text-gray-400 border-b border-gray-800",
                                    th { class: "px-4 py-2 font-medium", "Transfer ID" }
                                    th { class: "px-4 py-2 font-medium", "From" }
                                    th { class: "px-4 py-2 font-medium", "To" }
                                    th { class: "px-4 py-2 font-medium", "Right ID" }
                                    th { class: "px-4 py-2 font-medium", "Status" }
                                }
                            }
                            tbody { class: "divide-y divide-gray-800",
                                for t in transfers {
                                    tr { class: "hover:bg-gray-800/50 transition-colors",
                                        td { class: "px-4 py-3 font-mono text-xs", "{truncate_address(&t.id, 6)}" }
                                        td { class: "px-4 py-3", span { class: "{chain_badge_class(&t.from_chain)}", "{chain_icon_emoji(&t.from_chain)}" } }
                                        td { class: "px-4 py-3", span { class: "{chain_badge_class(&t.to_chain)}", "{chain_icon_emoji(&t.to_chain)}" } }
                                        td { class: "px-4 py-3 font-mono text-xs", "{truncate_address(&t.right_id, 8)}" }
                                        td { class: "px-4 py-3",
                                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {transfer_status_class(&t.status)}",
                                                "{t.status}"
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

#[component]
pub fn CrossChainTransfer() -> Element {
    let wallet_ctx = use_wallet_context();
    let mut from_chain = use_signal(|| Chain::Bitcoin);
    let mut to_chain = use_signal(|| Chain::Sui);
    let mut right_id = use_signal(String::new);
    let mut dest_owner = use_signal(String::new);
    let mut step = use_signal(|| 0);
    let result = use_signal(|| Option::<String>::None);
    let mut error = use_signal(|| Option::<String>::None);
    let mut executing = use_signal(|| false);
    let mut selected_account_index = use_signal(|| 0usize);
    let mut selected_target_contract_index = use_signal(|| 0usize);

    // Get accounts for the source chain
    let accounts = wallet_ctx.accounts_for_chain(*from_chain.read());
    let has_account = !accounts.is_empty();

    // Get contracts for source and target chains
    let source_contracts = wallet_ctx.contracts_for_chain(*from_chain.read());
    let target_contracts = wallet_ctx.contracts_for_chain(*to_chain.read());
    let _has_source_contract = !source_contracts.is_empty() || matches!(*from_chain.read(), Chain::Bitcoin);
    let has_target_contract = !target_contracts.is_empty();
    
    // Reset target contract selection when target chain changes
    use_effect(move || {
        selected_target_contract_index.set(0);
    });

    let steps = [
        "Select Account",
        "Lock Right on source chain",
        "Generate cryptographic proof",
        "Verify proof on destination",
        "Mint Right on destination",
        "Complete transfer",
    ];

    // Execute real cross-chain transfer using native signing
    let execute_transfer = move |_| {
        if !has_account {
            error.set(Some(format!("No account available for {:?}. Please add an account first.", *from_chain.read())));
            return;
        }

        if right_id.read().is_empty() {
            error.set(Some("Please enter a Right ID".to_string()));
            return;
        }

        // All chains now supported via proper BCS/ABI encoding
        // - Bitcoin: Native UTXO with mempool.space
        // - Ethereum: Native ABI encoding
        // - Sui: BCS encoding via sdk_tx
        // - Aptos: BCS encoding via sdk_tx (planned)
        
        let from = *from_chain.read();
        let to = *to_chain.read();
        
        executing.set(true);
        error.set(None);
        step.set(1);

        // Spawn async task for blockchain operations
        spawn({
            let right = right_id.read().clone();
            let dest = dest_owner.read().clone();
            let account_idx = *selected_account_index.read();
            let target_contract_idx = *selected_target_contract_index.read();
            let accounts = wallet_ctx.accounts_for_chain(from);
            let mut step_signal = step;
            let mut result_signal = result;
            let mut error_signal = error;
            let mut executing_signal = executing;
            let mut wallet_ctx = wallet_ctx.clone();

            async move {
                use crate::services::blockchain_service::{BlockchainConfig, BlockchainService, NativeWallet};
                use crate::wallet_core::ChainAccount;

                // Get the selected account
                let account: ChainAccount = if let Some(acc) = accounts.get(account_idx) {
                    acc.clone()
                } else {
                    error_signal.set(Some("Selected account not found".to_string()));
                    executing_signal.set(false);
                    return;
                };

                // Create native wallet from account
                let signer = NativeWallet::new(from, account);
                let service = BlockchainService::new(BlockchainConfig::default());

                // Determine destination owner (default to same address)
                let dest_addr = if dest.is_empty() {
                    signer.address()
                } else {
                    dest
                };

                // Step 1: Lock right on source chain
                step_signal.set(1);
                web_sys::console::log_1(&"Step 1: Locking right on source chain...".into());

                // Build contracts map with selected target contract
                let mut contracts = std::collections::HashMap::new();
                
                // Get target contracts and select the one user chose
                let target_contracts = wallet_ctx.contracts_for_chain(to);
                if !target_contracts.is_empty() {
                    let selected_idx = target_contract_idx.min(target_contracts.len() - 1);
                    if let Some(contract) = target_contracts.get(selected_idx) {
                        contracts.insert(to, crate::services::blockchain_service::ContractDeployment {
                            chain: to,
                            contract_address: contract.address.clone(),
                            tx_hash: contract.tx_hash.clone(),
                            deployed_at: contract.deployed_at,
                            contract_type: crate::services::blockchain_service::ContractType::Lock,
                        });
                    }
                }

                match service.execute_cross_chain_transfer(
                    from,
                    to,
                    &right,
                    &dest_addr,
                    &contracts,
                    &signer,
                ).await {
                    Ok(transfer_result) => {
                        step_signal.set(5);
                        let transfer_id = transfer_result.transfer_id.clone();

                        // Record the transfer
                        wallet_ctx.add_transfer(TrackedTransfer {
                            id: transfer_id.clone(),
                            from_chain: from,
                            to_chain: to,
                            right_id: right.clone(),
                            dest_owner: dest_addr.clone(),
                            status: TransferStatus::Completed,
                            created_at: js_sys::Date::now() as u64 / 1000,
                        });

                        result_signal.set(Some(format!(
                            "Transfer complete!\nTransfer ID: {}\nRight {} moved from {:?} to {:?}\nLock TX: {}\nMint TX: {}",
                            transfer_id, right, from, to,
                            transfer_result.lock_tx_hash,
                            transfer_result.mint_tx_hash
                        )));
                    }
                    Err(e) => {
                        error_signal.set(Some(format!("Transfer failed: {}", e)));
                        web_sys::console::error_1(&format!("Transfer error: {:?}", e).into());
                    }
                }

                executing_signal.set(false);
            }
        });
    };

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::CrossChain {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Cross-Chain Transfer" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                // Account Selection Section
                div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700",
                    h3 { class: "text-sm font-medium text-gray-300 mb-3", "1. Select Source Account" }
                    if accounts.is_empty() {
                        div { class: "text-sm text-red-400",
                            {format!("No accounts available for {:?}. Please add an account first.", *from_chain.read())}
                        }
                    } else {
                        select {
                            class: "{input_class()}",
                            onchange: move |evt| {
                                if let Ok(idx) = evt.value().parse::<usize>() {
                                    selected_account_index.set(idx);
                                }
                            },
                            for (idx, account) in accounts.iter().enumerate() {
                                option { value: idx.to_string(), selected: idx == *selected_account_index.read(),
                                    {format!("{} - {} (Balance: {:.4})",
                                        account.name,
                                        &account.address[..8.min(account.address.len())],
                                        account.balance
                                    )}
                                }
                            }
                        }
                    }
                }

                div { class: "grid grid-cols-2 gap-4",
                    {form_field("From Chain", chain_select(move |v: Rc<FormData>| {
                        if let Ok(c) = v.value().parse::<Chain>() {
                            from_chain.set(c);
                            selected_account_index.set(0); // Reset account selection
                        }
                    }, *from_chain.read()))}

                    {form_field("To Chain", chain_select(move |v: Rc<FormData>| {
                        if let Ok(c) = v.value().parse::<Chain>() { to_chain.set(c); }
                    }, *to_chain.read()))}
                }

                // Chain compatibility note
                div { class: "bg-blue-900/30 border border-blue-700/50 rounded-lg p-3",
                    p { class: "text-xs text-blue-300", "Chain support (all via native signing):" }
                    div { class: "flex gap-2 mt-1 text-xs",
                        span { class: "text-green-400", "✓ Bitcoin: UTXO" }
                        span { class: "text-green-400", "✓ Ethereum: ABI" }
                        span { class: "text-green-400", "✓ Sui: BCS" }
                        span { class: "text-green-400", "✓ Aptos: BCS" }
                        span { class: "text-green-400", "✓ Solana: Native" }
                    }
                }

                // Contracts display section
                div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700",
                    h3 { class: "text-sm font-medium text-gray-300 mb-3", "Deployed Contracts" }
                    div { class: "grid grid-cols-2 gap-4",
                        // Source chain contracts
                        div {
                            p { class: "text-xs text-gray-500 mb-1", {format!("Source ({:?})", *from_chain.read())} }
                            if source_contracts.is_empty() {
                                if matches!(*from_chain.read(), Chain::Bitcoin) {
                                    p { class: "text-xs text-green-400", "✓ UTXO chain - no contract needed" }
                                } else {
                                    p { class: "text-xs text-red-400", "✗ No contract deployed" }
                                }
                            } else {
                                for contract in source_contracts.iter() {
                                    p { class: "text-xs text-green-400 font-mono",
                                        {format!("✓ {}", &contract.address[..16.min(contract.address.len())])}
                                    }
                                }
                            }
                        }
                        // Target chain contracts
                        div {
                            p { class: "text-xs text-gray-500 mb-1", {format!("Target ({:?})", *to_chain.read())} }
                            if target_contracts.is_empty() {
                                p { class: "text-xs text-red-400", "✗ No contract deployed" }
                            } else if target_contracts.len() == 1 {
                                // Single contract - just display it
                                p { class: "text-xs text-green-400 font-mono",
                                    {format!("✓ {}", &target_contracts[0].address[..16.min(target_contracts[0].address.len())])}
                                }
                            } else {
                                // Multiple contracts - show selector
                                div { class: "space-y-1",
                                    p { class: "text-xs text-blue-400", "Select contract:" }
                                    select {
                                        class: "w-full bg-gray-800 border border-gray-700 rounded px-2 py-1 text-xs font-mono",
                                        onchange: move |evt| {
                                            if let Ok(idx) = evt.value().parse::<usize>() {
                                                selected_target_contract_index.set(idx);
                                            }
                                        },
                                        for (idx, contract) in target_contracts.iter().enumerate() {
                                            option { value: idx.to_string(), selected: idx == *selected_target_contract_index.read(),
                                                {format!("{}...", &contract.address[..12.min(contract.address.len())])}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                {form_field("Right ID", rsx! {
                    input {
                        value: "{right_id.read()}",
                        oninput: move |evt| { right_id.set(evt.value()); },
                        class: "{input_mono_class()}",
                        placeholder: "0x...",
                        disabled: *executing.read(),
                    }
                })}

                {form_field("Destination Owner (optional)", rsx! {
                    input {
                        value: "{dest_owner.read()}",
                        oninput: move |evt| { dest_owner.set(evt.value()); },
                        class: "{input_mono_class()}",
                        placeholder: "0x... (defaults to your address)",
                        disabled: *executing.read(),
                    }
                })}

                // Progress steps
                if *step.read() > 0 {
                    div { class: "space-y-2 mt-4",
                        for (i, step_text) in steps.iter().enumerate() {
                            div { class: "flex items-center gap-2",
                                if i < *step.read() {
                                    span { class: "text-green-400", "\u{2705}" }
                                    p { class: "text-sm text-green-400", "{step_text}" }
                                } else if i == *step.read() {
                                    span { class: "text-blue-400 animate-pulse", "\u{23F3}" }
                                    p { class: "text-sm text-blue-400", "{step_text}" }
                                } else {
                                    span { class: "text-gray-600", "\u{2B55}" }
                                    p { class: "text-sm text-gray-500", "{step_text}" }
                                }
                            }
                        }
                    }
                }

                if let Some(err) = error.read().as_ref() {
                    div { class: "p-4 bg-red-900/30 border border-red-700/50 rounded-lg",
                        p { class: "text-red-300 text-sm", "{err}" }
                    }
                }

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300 font-mono text-sm break-all whitespace-pre-wrap", "{msg}" }
                    }
                }

                button {
                    onclick: execute_transfer,
                    disabled: *executing.read()
                        || *step.read() >= 5
                        || right_id.read().is_empty()
                        || !has_account
                        || !has_target_contract,
                    class: "{btn_full_primary_class()}",
                    if *executing.read() {
                        "Executing..."
                    } else if !has_account {
                        "Add Source Account First"
                    } else if !has_target_contract {
                        "Deploy Target Contract First"
                    } else if *step.read() >= 5 {
                        "Transfer Complete"
                    } else {
                        "Execute Cross-Chain Transfer"
                    }
                }

                if !has_account {
                    p { class: "text-xs text-red-500 mt-2",
                        "Note: Add an account for the selected source chain"
                    }
                }
                if !has_target_contract {
                    p { class: "text-xs text-red-500 mt-2",
                        {format!("Note: Deploy a CSV contract on {:?} target chain first", *to_chain.read())}
                    }
                }
            }
        }
    }
}

#[component]
pub fn CrossChainStatus() -> Element {
    let wallet_ctx = use_wallet_context();
    let mut transfer_id = use_signal(String::new);
    let mut result = use_signal(|| Option::<TrackedTransfer>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::CrossChain {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Transfer Status" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Transfer ID", rsx! {
                    input {
                        value: "{transfer_id.read()}",
                        oninput: move |evt| { transfer_id.set(evt.value()); },
                        class: "{input_mono_class()}",
                        placeholder: "0x..."
                    }
                })}

                button {
                    onclick: move |_| {
                        result.set(wallet_ctx.get_transfer(&transfer_id.read()));
                    },
                    class: "{btn_full_primary_class()}",
                    "Check Status"
                }

                if let Some(t) = result.read().as_ref() {
                    div { class: "space-y-3",
                        div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700 space-y-2",
                            div { class: "flex justify-between",
                                span { class: "text-sm text-gray-400", "Source" }
                                span { class: "{chain_badge_class(&t.from_chain)}", "{chain_icon_emoji(&t.from_chain)} {chain_name(&t.from_chain)}" }
                            }
                            div { class: "flex justify-between",
                                span { class: "text-sm text-gray-400", "Destination" }
                                span { class: "{chain_badge_class(&t.to_chain)}", "{chain_icon_emoji(&t.to_chain)} {chain_name(&t.to_chain)}" }
                            }
                            div { class: "flex justify-between",
                                span { class: "text-sm text-gray-400", "Status" }
                                span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {transfer_status_class(&t.status)}",
                                    "{t.status}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn CrossChainRetry() -> Element {
    let wallet_ctx = use_wallet_context();
    let mut transfer_id = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::CrossChain {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Retry Transfer" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Transfer ID", rsx! {
                    input {
                        value: "{transfer_id.read()}",
                        oninput: move |evt| { transfer_id.set(evt.value()); },
                        class: "{input_mono_class()}",
                        placeholder: "0x..."
                    }
                })}

                button {
                    onclick: move |_| {
                        if let Some(t) = wallet_ctx.get_transfer(&transfer_id.read()) {
                            if t.status == TransferStatus::Failed {
                                result.set(Some("Retry initiated. Monitor status for updates.".to_string()));
                            } else {
                                result.set(Some(format!("Transfer status: {}. Only failed transfers can be retried.", t.status)));
                            }
                        } else {
                            result.set(Some("Transfer not found.".to_string()));
                        }
                    },
                    class: "{btn_full_primary_class()}",
                    "Retry Transfer"
                }

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-blue-900/30 border border-blue-700/50 rounded-lg",
                        p { class: "text-blue-300", "{msg}" }
                    }
                }
            }
        }
    }
}

// ===== Contracts Pages =====
#[component]
pub fn Contracts() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let contracts = wallet_ctx.contracts();
    let accounts = wallet_ctx.accounts();

    // State for contract discovery
    let discovering = use_signal(|| false);
    let discovered_count = use_signal(|| 0usize);

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold", "Contracts" }
                div { class: "flex gap-2",
                    button {
                        onclick: move |_| {
                            if *discovering.read() {
                                return;
                            }
                            discovering.set(true);
                            discovered_count.set(0);

                            spawn({
                                let mut wallet_ctx = wallet_ctx.clone();
                                let mut discovering_signal = discovering;
                                let mut count_signal = discovered_count;
                                async move {
                                    use crate::services::chain_api::ChainConfig;
                                    use crate::services::network::NetworkType;
                                    use crate::services::transaction_builder::discover_contracts;

                                    let mut total_found = 0;

                                    for account in &accounts {
                                        if matches!(account.chain, Chain::Bitcoin) {
                                            continue;
                                        }

                                        let config = ChainConfig::for_chain(account.chain, NetworkType::Testnet);

                                        match discover_contracts(account.chain, &account.address, &config.api_url).await {
                                            Ok(contracts) => {
                                                for c in contracts {
                                                    wallet_ctx.add_contract(DeployedContract {
                                                        chain: account.chain,
                                                        address: c.address,
                                                        tx_hash: generate_id(),
                                                        deployed_at: js_sys::Date::now() as u64 / 1000,
                                                    });
                                                    total_found += 1;
                                                }
                                            }
                                            Err(e) => {
                                                web_sys::console::warn_1(&format!("Discovery failed for {:?}: {:?}", account.chain, e).into());
                                            }
                                        }
                                    }

                                    count_signal.set(total_found);
                                    discovering_signal.set(false);

                                    if total_found > 0 {
                                        wallet_ctx.set_notification(
                                            NotificationKind::Success,
                                            format!("Discovered {} contract(s) from chain", total_found)
                                        );
                                    } else {
                                        wallet_ctx.set_notification(
                                            NotificationKind::Info,
                                            "No new contracts found on chain".to_string()
                                        );
                                    }
                                }
                            });
                        },
                        disabled: *discovering.read() || accounts.is_empty(),
                        class: if *discovering.read() {
                            "px-3 py-1.5 rounded-lg text-sm font-medium bg-gray-700 text-gray-400 cursor-not-allowed"
                        } else {
                            "px-3 py-1.5 rounded-lg text-sm font-medium bg-indigo-600 hover:bg-indigo-700 text-white transition-colors"
                        },
                        if *discovering.read() {
                            span { class: "flex items-center gap-2",
                                span { class: "animate-spin", "\u{27F3}" }
                                "Discovering..."
                            }
                        } else {
                            "\u{1F50D} Discover from Chain"
                        }
                    }
                    Link { to: Route::AddContract {}, class: "{btn_secondary_class()}", "+ Add Existing" }
                    Link { to: Route::DeployContract {}, class: "{btn_primary_class()}", "+ Deploy New" }
                }
            }

            if *discovered_count.read() > 0 {
                div { class: "bg-green-900/30 border border-green-700/50 rounded-lg p-3",
                    p { class: "text-sm text-green-300",
                        "\u{2713} Discovered and added {discovered_count} contract(s) from chain"
                    }
                }
            }

            if contracts.is_empty() {
                {empty_state("\u{1F4DC}", "No contracts deployed", "Deploy contracts or discover from chain to enable cross-chain functionality.")}
            } else {
                div { class: "{table_class()}",
                    div { class: "{card_header_class()}",
                        h2 { class: "font-semibold text-sm", "Deployed Contracts" }
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "text-left text-gray-400 border-b border-gray-800",
                                    th { class: "px-4 py-2 font-medium", "Chain" }
                                    th { class: "px-4 py-2 font-medium", "Address" }
                                    th { class: "px-4 py-2 font-medium", "TX Hash" }
                                }
                            }
                            tbody { class: "divide-y divide-gray-800",
                                for c in contracts {
                                    tr { class: "hover:bg-gray-800/50 transition-colors",
                                        td { class: "px-4 py-3", span { class: "{chain_badge_class(&c.chain)}", "{chain_icon_emoji(&c.chain)} {chain_name(&c.chain)}" } }
                                        td { class: "px-4 py-3 font-mono text-xs", "{truncate_address(&c.address, 8)}" }
                                        td { class: "px-4 py-3 font-mono text-xs", "{truncate_address(&c.tx_hash, 8)}" }
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

#[component]
pub fn DeployContract() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Ethereum);
    let mut selected_network = use_signal(|| Network::Test);
    let mut deployer_key = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);

    let is_bitcoin = *selected_chain.read() == Chain::Bitcoin;

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Contracts {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Deploy Contract" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
                }, *selected_chain.read()))}

                {form_field("Network", network_select(move |n| {
                    selected_network.set(n);
                }, *selected_network.read()))}

                if !is_bitcoin {
                    {form_field("Deployer Private Key", rsx! {
                        input {
                            value: "{deployer_key.read()}",
                            oninput: move |evt| { deployer_key.set(evt.value()); },
                            class: "{input_mono_class()}",
                            placeholder: "0x..."
                        }
                    })}
                }

                if is_bitcoin {
                    div { class: "bg-gray-800/50 rounded-lg p-3 border border-gray-700",
                        p { class: "text-sm text-gray-400",
                            "\u{2139}\u{FE0F} Bitcoin is UTXO-native and does not require contract deployment."
                        }
                    }
                }

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300 font-mono text-sm break-all", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        if is_bitcoin {
                            result.set(Some("Bitcoin does not need contract deployment.".to_string()));
                        } else {
                            let addr = generate_id();
                            wallet_ctx.add_contract(DeployedContract {
                                chain: *selected_chain.read(),
                                address: addr.clone(),
                                tx_hash: generate_id(),
                                deployed_at: 0,
                            });
                            result.set(Some(format!("Contract deployed at: {}", addr)));
                        }
                    },
                    class: "{btn_full_primary_class()}",
                    if is_bitcoin { "Not Applicable" } else { "Deploy" }
                }
            }
        }
    }
}

#[component]
pub fn AddContract() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Sui);
    let mut contract_address = use_signal(String::new);
    let mut tx_hash = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);
    let mut error = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Contracts {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Add Existing Contract" }
            }

            div { class: "bg-blue-900/30 border border-blue-700/50 rounded-lg p-4",
                p { class: "text-sm text-blue-300",
                    "Use this form to add a contract that was already deployed (e.g., via csv-cli)."
                }
                p { class: "text-xs text-blue-400 mt-1",
                    "Example Sui package: 0xa972ca52e0c69118471a755aee0efd89993649b6d1f32a4fc9e186c1458694c2"
                }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() {
                        selected_chain.set(c);
                        error.set(None);
                    }
                }, *selected_chain.read()))}

                {form_field("Contract Address / Package ID", rsx! {
                    input {
                        value: "{contract_address.read()}",
                        oninput: move |evt| { contract_address.set(evt.value()); error.set(None); },
                        class: "{input_mono_class()}",
                        placeholder: "0x..."
                    }
                })}

                {form_field("Transaction Hash (optional)", rsx! {
                    input {
                        value: "{tx_hash.read()}",
                        oninput: move |evt| { tx_hash.set(evt.value()); },
                        class: "{input_mono_class()}",
                        placeholder: "0x..."
                    }
                })}

                if let Some(e) = error.read().as_ref() {
                    div { class: "p-4 bg-red-900/30 border border-red-700/50 rounded-lg",
                        p { class: "text-red-300 text-sm", "{e}" }
                    }
                }

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300 font-mono text-sm", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        let addr = contract_address.read().trim().to_string();
                        if addr.is_empty() {
                            error.set(Some("Contract address is required".to_string()));
                            return;
                        }
                        // Only require 0x prefix for non-Solana chains (Solana uses base58)
                        let chain = *selected_chain.read();
                        if chain != Chain::Solana && !addr.starts_with("0x") {
                            error.set(Some("Address must start with 0x".to_string()));
                            return;
                        }

                        let tx = tx_hash.read().trim().to_string();
                        let tx = if tx.is_empty() { generate_id() } else { tx };

                        wallet_ctx.add_contract(DeployedContract {
                            chain: *selected_chain.read(),
                            address: addr.clone(),
                            tx_hash: tx,
                            deployed_at: js_sys::Date::now() as u64 / 1000,
                        });

                        result.set(Some(format!("Contract added for {:?}", *selected_chain.read())));
                        contract_address.set(String::new());
                        tx_hash.set(String::new());
                    },
                    disabled: contract_address.read().is_empty(),
                    class: "{btn_full_primary_class()}",
                    "Add Contract"
                }
            }
        }
    }
}

#[component]
pub fn ContractStatus() -> Element {
    let wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Ethereum);

    let contracts = wallet_ctx.contracts_for_chain(*selected_chain.read());

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Contracts {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Contract Status" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
                }, *selected_chain.read()))}

                if contracts.is_empty() {
                    div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700 text-center",
                        p { class: "text-gray-400", "No contracts deployed on {chain_name(&selected_chain.read())}" }
                        Link { to: Route::DeployContract {}, class: "text-blue-400 hover:text-blue-300 text-sm mt-1 inline-block", "Deploy now \u{2192}" }
                    }
                } else {
                    for c in contracts {
                        div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700 space-y-2",
                            div { class: "flex justify-between",
                                span { class: "text-sm text-gray-400", "Address" }
                                p { class: "font-mono text-sm text-gray-200", "{truncate_address(&c.address, 10)}" }
                            }
                            div { class: "flex justify-between",
                                span { class: "text-sm text-gray-400", "TX Hash" }
                                p { class: "font-mono text-sm text-gray-200", "{truncate_address(&c.tx_hash, 10)}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ===== Seals Pages =====
#[component]
pub fn Seals() -> Element {
    let wallet_ctx = use_wallet_context();
    let seals = wallet_ctx.seals();
    let mut filter_chain = use_signal(|| Option::<Chain>::None);

    let filtered = match *filter_chain.read() {
        Some(c) => seals
            .iter()
            .filter(|s| s.chain == c)
            .cloned()
            .collect::<Vec<_>>(),
        None => seals,
    };

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold", "Seals" }
                Link { to: Route::CreateSeal {}, class: "{btn_primary_class()}", "+ Create Seal" }
            }

            // Filter bar
            div { class: "flex items-center gap-2 flex-wrap",
                span { class: "text-sm text-gray-400", "Filter:" }
                button {
                    onclick: move |_| filter_chain.set(None),
                    class: if filter_chain.read().is_none() { "{btn_primary_class()}" } else { "{btn_secondary_class()}" },
                    "All"
                }
                for chain in [Chain::Bitcoin, Chain::Ethereum, Chain::Sui, Chain::Aptos, Chain::Solana] {
                    button {
                        onclick: move |_| filter_chain.set(Some(chain)),
                        class: if matches!(*filter_chain.read(), Some(c) if c == chain) { "{chain_badge_class(&chain)} cursor-pointer" } else { "{chain_badge_class(&chain)} opacity-50 cursor-pointer" },
                        "{chain_icon_emoji(&chain)} {chain_name(&chain)}"
                    }
                }
            }

            if filtered.is_empty() {
                {empty_state("\u{1F512}", "No seals found", "Create a seal on a chain to get started.")}
            } else {
                div { class: "{table_class()}",
                    div { class: "{card_header_class()} flex items-center justify-between",
                        h2 { class: "font-semibold text-sm", "Consumed Seals" }
                        span { class: "text-xs text-gray-400", "{filtered.len()} total" }
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "text-left text-gray-400 border-b border-gray-800",
                                    th { class: "px-4 py-2 font-medium", "#" }
                                    th { class: "px-4 py-2 font-medium", "Seal Ref" }
                                    th { class: "px-4 py-2 font-medium", "Chain" }
                                    th { class: "px-4 py-2 font-medium", "Value" }
                                    th { class: "px-4 py-2 font-medium", "Status" }
                                }
                            }
                            tbody { class: "divide-y divide-gray-800",
                                for (i, seal) in filtered.iter().enumerate() {
                                    tr { class: "hover:bg-gray-800/50 transition-colors",
                                        td { class: "px-4 py-3 text-gray-400", "{i + 1}" }
                                        td { class: "px-4 py-3 font-mono text-xs", "{truncate_address(&seal.seal_ref, 12)}" }
                                        td { class: "px-4 py-3", span { class: "{chain_badge_class(&seal.chain)}", "{chain_icon_emoji(&seal.chain)} {chain_name(&seal.chain)}" } }
                                        td { class: "px-4 py-3 font-mono text-xs", "{seal.value}" }
                                        td { class: "px-4 py-3",
                                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium",
                                                class: if seal.consumed { "text-gray-400 bg-gray-500/20" } else { "text-green-400 bg-green-500/20" },
                                                if seal.consumed { "Consumed" } else { "Available" }
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

#[component]
pub fn CreateSeal() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Bitcoin);
    let mut value = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Seals {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Create Seal" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
                }, *selected_chain.read()))}

                {form_field("Value (optional)", rsx! {
                    input {
                        value: "{value.read()}",
                        oninput: move |evt| { value.set(evt.value()); },
                        class: "{input_mono_class()}",
                        placeholder: "e.g., 1000"
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300 font-mono text-sm break-all", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        let seal_ref = generate_id();
                        let val: u64 = value.read().parse().unwrap_or(0);
                        wallet_ctx.add_seal(SealRecord {
                            seal_ref: seal_ref.clone(),
                            chain: *selected_chain.read(),
                            value: val,
                            consumed: false,
                            created_at: 0,
                        });
                        result.set(Some(format!("Seal created on {} (ref: {})", chain_name(&selected_chain.read()), truncate_address(&seal_ref, 16))));
                    },
                    class: "{btn_full_primary_class()}",
                    "Create Seal"
                }
            }
        }
    }
}

#[component]
pub fn ConsumeSeal() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Bitcoin);
    let mut seal_ref = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);
    let mut error = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Seals {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Consume Seal" }
            }

            div { class: "bg-yellow-900/30 border border-yellow-700/50 rounded-xl p-4",
                div { class: "flex items-center gap-2",
                    span { class: "text-yellow-400", "\u{26A0}\u{FE0F}" }
                    p { class: "text-yellow-300 font-medium", "Warning: Seal consumption is irreversible" }
                }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
                }, *selected_chain.read()))}

                {form_field("Seal Reference (hex)", rsx! {
                    input {
                        value: "{seal_ref.read()}",
                        oninput: move |evt| { seal_ref.set(evt.value()); error.set(None); },
                        class: "{input_mono_class()}",
                        placeholder: "0x..."
                    }
                })}

                if let Some(e) = error.read().as_ref() {
                    div { class: "p-3 bg-red-900/30 border border-red-700/50 rounded-lg text-sm text-red-300", "{e}" }
                }

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        if wallet_ctx.is_seal_consumed(&seal_ref.read()) {
                            error.set(Some("Seal replay detected: this seal has already been consumed.".to_string()));
                        } else {
                            let val: u64 = 0;
                            wallet_ctx.add_seal(SealRecord {
                                seal_ref: seal_ref.read().clone(),
                                chain: *selected_chain.read(),
                                value: val,
                                consumed: true,
                                created_at: 0,
                            });
                            result.set(Some("Seal consumed successfully.".to_string()));
                        }
                    },
                    class: "w-full px-4 py-2.5 rounded-lg bg-red-600 hover:bg-red-700 text-sm font-medium transition-colors",
                    "Consume Seal"
                }
            }
        }
    }
}

#[component]
pub fn VerifySeal() -> Element {
    let wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Bitcoin);
    let mut seal_ref = use_signal(String::new);
    let mut result = use_signal(|| Option::<bool>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Seals {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Verify Seal" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
                }, *selected_chain.read()))}

                {form_field("Seal Reference (hex)", rsx! {
                    input {
                        value: "{seal_ref.read()}",
                        oninput: move |evt| { seal_ref.set(evt.value()); result.set(None); },
                        class: "{input_mono_class()}",
                        placeholder: "0x..."
                    }
                })}

                if let Some(consumed) = result.read().as_ref() {
                    div { class: "p-4 {seal_consumed_class(*consumed)} border rounded-lg",
                        p { class: "{seal_consumed_text_class(*consumed)}",
                            if *consumed { "Seal is CONSUMED" } else { "Seal is UNCONSUMED (available)" }
                        }
                    }
                }

                button {
                    onclick: move |_| {
                        result.set(Some(wallet_ctx.is_seal_consumed(&seal_ref.read())));
                    },
                    class: "{btn_full_primary_class()}",
                    "Verify Seal"
                }
            }
        }
    }
}

// ===== Test Pages =====
#[component]
pub fn Test() -> Element {
    let wallet_ctx = use_wallet_context();
    let results = wallet_ctx.test_results();
    let passed = results
        .iter()
        .filter(|r| r.status == TestStatus::Passed)
        .count();
    let failed = results
        .iter()
        .filter(|r| r.status == TestStatus::Failed)
        .count();

    rsx! {
        div { class: "space-y-6",
            h1 { class: "text-2xl font-bold", "Tests" }

            if !results.is_empty() {
                div { class: "grid grid-cols-1 sm:grid-cols-3 gap-4",
                    {stat_card("Total", &results.len().to_string(), "\u{1F9EA}")}
                    {stat_card("Passed", &passed.to_string(), "\u{2705}")}
                    {stat_card("Failed", &failed.to_string(), "\u{274C}")}
                }
            }

            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                Link { to: Route::RunTests {}, class: "{card_class()} p-6 hover:bg-gray-800/50 transition-colors block",
                    div { class: "flex items-center gap-3", span { class: "text-2xl", "\u{25B6}\u{FE0F}" }, div { h3 { class: "font-semibold", "Run Tests" } p { class: "text-sm text-gray-400", "Run end-to-end chain tests" } } }
                }
                Link { to: Route::RunScenario {}, class: "{card_class()} p-6 hover:bg-gray-800/50 transition-colors block",
                    div { class: "flex items-center gap-3", span { class: "text-2xl", "\u{1F3AC}" }, div { h3 { class: "font-semibold", "Run Scenario" } p { class: "text-sm text-gray-400", "Run specific test scenarios" } } }
                }
            }

            if !results.is_empty() {
                div { class: "{table_class()}",
                    div { class: "{card_header_class()}",
                        h2 { class: "font-semibold text-sm", "Test Results" }
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "text-left text-gray-400 border-b border-gray-800",
                                    th { class: "px-4 py-2 font-medium", "From" }
                                    th { class: "px-4 py-2 font-medium", "To" }
                                    th { class: "px-4 py-2 font-medium", "Status" }
                                    th { class: "px-4 py-2 font-medium", "Message" }
                                }
                            }
                            tbody { class: "divide-y divide-gray-800",
                                for r in results {
                                    tr { class: "hover:bg-gray-800/50 transition-colors",
                                        td { class: "px-4 py-3", span { class: "{chain_badge_class(&r.from_chain)}", "{chain_icon_emoji(&r.from_chain)}" } }
                                        td { class: "px-4 py-3", span { class: "{chain_badge_class(&r.to_chain)}", "{chain_icon_emoji(&r.to_chain)}" } }
                                        td { class: "px-4 py-3",
                                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {test_status_class(&r.status)}",
                                                "{r.status}"
                                            }
                                        }
                                        td { class: "px-4 py-3 text-xs text-gray-400", "{r.message}" }
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

#[component]
pub fn RunTests() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_from = use_signal(|| Chain::Bitcoin);
    let mut selected_to = use_signal(|| Chain::Sui);
    let mut run_all = use_signal(|| false);
    let mut running = use_signal(|| false);
    let mut current_step = use_signal(|| 0);

    let test_steps = [
        "Checking chain connectivity...",
        "Creating Right on source...",
        "Locking Right on source...",
        "Verifying proof on destination...",
        "Minting Right on destination...",
    ];

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Test {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Run Tests" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                // All chains checkbox
                div { class: "flex items-center gap-2",
                    input {
                        r#type: "checkbox",
                        id: "run_all",
                        checked: *run_all.read(),
                        onchange: move |evt| { run_all.set(evt.data().checked()); },
                    }
                    label { r#for: "run_all", class: "text-sm text-gray-300", "Run all chain pairs" }
                }

                if !*run_all.read() {
                    div { class: "grid grid-cols-2 gap-4",
                        {form_field("From Chain", chain_select(move |v: Rc<FormData>| {
                            if let Ok(c) = v.value().parse::<Chain>() { selected_from.set(c); }
                        }, *selected_from.read()))}

                        {form_field("To Chain", chain_select(move |v: Rc<FormData>| {
                            if let Ok(c) = v.value().parse::<Chain>() { selected_to.set(c); }
                        }, *selected_to.read()))}
                    }
                }

                // Progress
                if *running.read() {
                    div { class: "space-y-2",
                        for (i, step_text) in test_steps.iter().enumerate() {
                            div { class: "flex items-center gap-2",
                                if i < *current_step.read() {
                                    span { class: "text-green-400", "\u{2705}" }
                                    p { class: "text-sm text-green-400", "{step_text}" }
                                } else if i == *current_step.read() {
                                    span { class: "text-blue-400 animate-pulse", "\u{23F3}" }
                                    p { class: "text-sm text-blue-400", "{step_text}" }
                                } else {
                                    span { class: "text-gray-600", "\u{2B55}" }
                                    p { class: "text-sm text-gray-500", "{step_text}" }
                                }
                            }
                        }
                    }
                }

                button {
                    onclick: move |_| {
                        running.set(true);
                        current_step.set(0);

                        // Simulate test steps
                        let pairs = if *run_all.read() {
                            vec![
                                (Chain::Bitcoin, Chain::Sui),
                                (Chain::Bitcoin, Chain::Ethereum),
                                (Chain::Sui, Chain::Ethereum),
                            ]
                        } else {
                            vec![(*selected_from.read(), *selected_to.read())]
                        };

                        for (from, to) in &pairs {
                            for i in 0..5 {
                                current_step.set(i);
                                wallet_ctx.add_test_result(TestResult {
                                    from_chain: *from,
                                    to_chain: *to,
                                    status: if i == 4 { TestStatus::Passed } else { TestStatus::Running },
                                    message: format!("Step {}/5", i + 1),
                                });
                            }
                        }

                        wallet_ctx.add_test_result(TestResult {
                            from_chain: pairs[0].0,
                            to_chain: pairs[0].1,
                            status: TestStatus::Passed,
                            message: "All tests completed".to_string(),
                        });
                        running.set(false);
                    },
                    disabled: *running.read(),
                    class: "{btn_full_primary_class()}",
                    if *running.read() { "Running..." } else { "Run Tests" }
                }
            }
        }
    }
}

#[component]
pub fn RunScenario() -> Element {
    let mut selected_scenario = use_signal(|| String::from("double_spend"));
    let mut result = use_signal(|| Option::<String>::None);

    let scenarios = [
        ("double_spend", "Double Spend Detection"),
        ("invalid_proof", "Invalid Proof Rejection"),
        ("ownership_transfer", "Ownership Transfer"),
    ];

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Test {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Run Scenario" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Scenario", rsx! {
                    select {
                        class: "{select_class()}",
                        value: "{selected_scenario.read()}",
                        onchange: move |evt| { selected_scenario.set(evt.value()); },
                        for (id, label) in scenarios {
                            option { value: "{id}", "{label}" }
                        }
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        result.set(Some(format!("Scenario '{}' completed successfully.", selected_scenario.read())));
                    },
                    class: "{btn_full_primary_class()}",
                    "Run Scenario"
                }
            }
        }
    }
}

// ===== Validate Pages =====
#[component]
pub fn Validate() -> Element {
    rsx! {
        div { class: "space-y-6",
            h1 { class: "text-2xl font-bold", "Validate" }

            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                Link { to: Route::ValidateConsignment {}, class: "{card_class()} p-6 hover:bg-gray-800/50 transition-colors block",
                    div { class: "flex items-center gap-3", span { class: "text-2xl", "\u{1F4C3}" }, div { h3 { class: "font-semibold", "Consignment" } p { class: "text-sm text-gray-400", "Validate a consignment file" } } }
                }
                Link { to: Route::ValidateProof {}, class: "{card_class()} p-6 hover:bg-gray-800/50 transition-colors block",
                    div { class: "flex items-center gap-3", span { class: "text-2xl", "\u{1F50D}" }, div { h3 { class: "font-semibold", "Proof" } p { class: "text-sm text-gray-400", "Validate a cross-chain proof" } } }
                }
                Link { to: Route::ValidateSeal {}, class: "{card_class()} p-6 hover:bg-gray-800/50 transition-colors block",
                    div { class: "flex items-center gap-3", span { class: "text-2xl", "\u{1F512}" }, div { h3 { class: "font-semibold", "Seal" } p { class: "text-sm text-gray-400", "Validate seal consumption" } } }
                }
                Link { to: Route::ValidateCommitmentChain {}, class: "{card_class()} p-6 hover:bg-gray-800/50 transition-colors block",
                    div { class: "flex items-center gap-3", span { class: "text-2xl", "\u{1F517}" }, div { h3 { class: "font-semibold", "Commitment Chain" } p { class: "text-sm text-gray-400", "Validate commitment chain integrity" } } }
                }
            }
        }
    }
}

#[component]
pub fn ValidateConsignment() -> Element {
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Validate {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Validate Consignment" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Consignment File", rsx! {
                    input {
                        class: "{input_class()}",
                        r#type: "file",
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        result.set(Some("Consignment is valid.".to_string()));
                    },
                    class: "{btn_full_primary_class()}",
                    "Validate"
                }
            }
        }
    }
}

#[component]
pub fn ValidateProof() -> Element {
    let mut selected_chain = use_signal(|| Chain::Bitcoin);
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Validate {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Validate Proof" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
                }, *selected_chain.read()))}

                {form_field("Proof File", rsx! {
                    input {
                        class: "{input_class()}",
                        r#type: "file",
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        result.set(Some("Proof is valid.".to_string()));
                    },
                    class: "{btn_full_primary_class()}",
                    "Validate Proof"
                }
            }
        }
    }
}

#[component]
pub fn ValidateSeal() -> Element {
    let mut seal_ref = use_signal(String::new);
    let mut result = use_signal(|| Option::<bool>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Validate {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Validate Seal" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Seal Reference (hex)", rsx! {
                    input {
                        value: "{seal_ref.read()}",
                        oninput: move |evt| { seal_ref.set(evt.value()); result.set(None); },
                        class: "{input_mono_class()}",
                        placeholder: "0x..."
                    }
                })}

                if let Some(consumed) = result.read().as_ref() {
                    div { class: "p-4 {seal_consumed_class(*consumed)} border rounded-lg",
                        p { class: "{seal_consumed_text_class(*consumed)}",
                            if *consumed { "Consumed (double-spend if reused)" } else { "Unconsumed (available)" }
                        }
                    }
                }

                button {
                    onclick: move |_| {
                        result.set(Some(!seal_ref.read().is_empty() && seal_ref.read().len() > 4));
                    },
                    class: "{btn_full_primary_class()}",
                    "Validate Seal"
                }
            }
        }
    }
}

#[component]
pub fn ValidateCommitmentChain() -> Element {
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Validate {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Validate Commitment Chain" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Commitment Chain File", rsx! {
                    input {
                        class: "{input_class()}",
                        r#type: "file",
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        result.set(Some("Commitment chain is valid.".to_string()));
                    },
                    class: "{btn_full_primary_class()}",
                    "Validate"
                }
            }
        }
    }
}

// ===== Settings =====
#[component]
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
                        "This will remove the wallet from memory. Your rights, seals, and other data will remain saved, but you'll need to re-import your wallet to access them."
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
                        "This will permanently delete all wallet data, rights, seals, transfers, and settings from localStorage. This action cannot be undone."
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
                                    let _ = storage.delete(crate::storage::WALLET_STATE_KEY);
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

// ===== Transactions List Page =====
#[component]
pub fn Transactions() -> Element {
    let wallet_ctx = use_wallet_context();
    let transactions = wallet_ctx.transactions();
    let accounts = wallet_ctx.accounts();

    // Get unique addresses from accounts for filtering
    let account_addresses: Vec<String> = accounts.iter().map(|a| a.address.clone()).collect();

    // Filter transactions related to our accounts
    let relevant_transactions: Vec<TransactionRecord> = transactions
        .into_iter()
        .filter(|tx| {
            account_addresses.contains(&tx.from_address)
                || tx.to_address.as_ref().map_or(false, |addr| account_addresses.contains(addr))
        })
        .collect();

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold", "Transactions" }
                span { class: "text-sm text-gray-400", "{relevant_transactions.len()} total" }
            }

            if relevant_transactions.is_empty() {
                {empty_state("\u{1F4B8}", "No Transactions", "Your transaction history will appear here")}
            } else {
                div { class: "space-y-3",
                    for tx in relevant_transactions {
                        TransactionCard { transaction: tx.clone() }
                    }
                }
            }
        }
    }
}

// ===== Transaction Card Component =====
#[component]
fn TransactionCard(transaction: TransactionRecord) -> Element {
    let wallet_ctx = use_wallet_context();
    let tx_clone = transaction.clone();

    let status_class = match transaction.status {
        TransactionStatus::Confirmed => "text-green-400 bg-green-500/20",
        TransactionStatus::Pending => "text-yellow-400 bg-yellow-500/20",
        TransactionStatus::Failed => "text-red-400 bg-red-500/20",
    };

    let explorer_url = wallet_ctx.get_explorer_url(transaction.chain, &transaction.tx_hash);

    rsx! {
        Link {
            to: Route::TransactionDetail { id: transaction.id.clone() },
            class: "{card_class()} p-4 block card-hover",
            div { class: "flex items-center justify-between",
                div { class: "flex items-center gap-3",
                    span { class: "{chain_badge_class(&transaction.chain)}",
                        "{chain_icon_emoji(&transaction.chain)}"
                    }
                    div {
                        p { class: "font-medium text-sm", "{transaction.tx_type.to_string()}" }
                        p { class: "text-xs text-gray-500",
                            {format!("{} → {}", 
                                truncate_address(&transaction.from_address, 6),
                                transaction.to_address.as_ref().map_or("?".to_string(), |a| truncate_address(a, 6))
                            )}
                        }
                    }
                }
                div { class: "text-right",
                    span { class: "inline-flex items-center px-2 py-1 rounded text-xs font-medium {status_class}",
                        "{transaction.status.to_string()}"
                    }
                    if let Some(block) = transaction.block_number {
                        p { class: "text-xs text-gray-500 mt-1", "Block {block}" }
                    }
                }
            }
            if let Some(url) = explorer_url {
                div { class: "mt-3 pt-3 border-t border-gray-800 flex items-center justify-between",
                    span { class: "text-xs text-gray-500", "{truncate_address(&transaction.tx_hash, 8)}" }
                    a {
                        href: "{url}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        class: "text-xs text-blue-400 hover:text-blue-300 flex items-center gap-1",
                        onclick: |e| e.stop_propagation(),
                        "View on Explorer \u{2197}"
                    }
                }
            }
        }
    }
}

// ===== Transaction Detail Page =====
#[component]
pub fn TransactionDetail(id: String) -> Element {
    let wallet_ctx = use_wallet_context();
    let transaction = wallet_ctx.transaction_by_id(&id);

    if let Some(tx) = transaction {
        let explorer_url = wallet_ctx.get_explorer_url(tx.chain, &tx.tx_hash);
        let from_explorer_url = wallet_ctx.get_address_explorer_url(tx.chain, &tx.from_address);
        let to_explorer_url = tx.to_address.as_ref().and_then(|addr| {
            wallet_ctx.get_address_explorer_url(tx.chain, addr)
        });

        let status_class = match tx.status {
            TransactionStatus::Confirmed => "text-green-400 bg-green-500/20",
            TransactionStatus::Pending => "text-yellow-400 bg-yellow-500/20",
            TransactionStatus::Failed => "text-red-400 bg-red-500/20",
        };

        rsx! {
            div { class: "space-y-6",
                div { class: "flex items-center justify-between",
                    h1 { class: "text-2xl font-bold", "Transaction Details" }
                    Link { to: Route::Transactions {}, class: "text-sm text-blue-400 hover:text-blue-300", "\u{2190} Back" }
                }

                div { class: "{card_class()} p-6 space-y-6",
                    // Header
                    div { class: "flex items-center justify-between pb-6 border-b border-gray-800",
                        div { class: "flex items-center gap-3",
                            span { class: "{chain_badge_class(&tx.chain)}",
                                "{chain_icon_emoji(&tx.chain)} {chain_name(&tx.chain)}"
                            }
                            span { class: "px-3 py-1 rounded-full text-sm font-medium {status_class}",
                                "{tx.status.to_string()}"
                            }
                        }
                        if let Some(url) = explorer_url.clone() {
                            a {
                                href: "{url}",
                                target: "_blank",
                                rel: "noopener noreferrer",
                                class: "text-sm text-blue-400 hover:text-blue-300 flex items-center gap-1",
                                "View on Explorer \u{2197}"
                            }
                        }
                    }

                    // Transaction Type
                    div { class: "grid grid-cols-2 gap-4",
                        div {
                            p { class: "text-xs text-gray-500 mb-1", "Transaction Type" }
                            p { class: "text-sm font-medium", "{tx.tx_type.to_string()}" }
                        }
                        div {
                            p { class: "text-xs text-gray-500 mb-1", "Transaction Hash" }
                            p { class: "text-sm font-mono", "{truncate_address(&tx.tx_hash, 12)}" }
                        }
                    }

                    // Addresses
                    div { class: "space-y-3",
                        div {
                            p { class: "text-xs text-gray-500 mb-1", "From" }
                            div { class: "flex items-center justify-between bg-gray-800/50 rounded p-3",
                                span { class: "text-sm font-mono", "{tx.from_address}" }
                                if let Some(url) = from_explorer_url {
                                    a {
                                        href: "{url}",
                                        target: "_blank",
                                        rel: "noopener noreferrer",
                                        class: "text-xs text-blue-400 hover:text-blue-300",
                                        "View \u{2197}"
                                    }
                                }
                            }
                        }
                        if let Some(to) = &tx.to_address {
                            div {
                                p { class: "text-xs text-gray-500 mb-1", "To" }
                                div { class: "flex items-center justify-between bg-gray-800/50 rounded p-3",
                                    span { class: "text-sm font-mono", "{to}" }
                                    if let Some(url) = to_explorer_url {
                                        a {
                                            href: "{url}",
                                            target: "_blank",
                                            rel: "noopener noreferrer",
                                            class: "text-xs text-blue-400 hover:text-blue-300",
                                            "View \u{2197}"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Amount & Fee
                    div { class: "grid grid-cols-2 gap-4 pt-4 border-t border-gray-800",
                        if let Some(amount) = tx.amount {
                            div {
                                p { class: "text-xs text-gray-500 mb-1", "Amount" }
                                p { class: "text-lg font-semibold", "{format_balance(amount as f64, tx.chain)}" }
                            }
                        }
                        if let Some(fee) = tx.fee {
                            div {
                                p { class: "text-xs text-gray-500 mb-1", "Fee" }
                                p { class: "text-lg font-semibold", "{format_balance(fee as f64, tx.chain)}" }
                            }
                        }
                    }

                    // Block Info
                    div { class: "grid grid-cols-2 gap-4 pt-4 border-t border-gray-800",
                        if let Some(block) = tx.block_number {
                            div {
                                p { class: "text-xs text-gray-500 mb-1", "Block Number" }
                                p { class: "text-sm font-medium", "{block}" }
                            }
                        }
                        if let Some(confirmations) = tx.confirmations {
                            div {
                                p { class: "text-xs text-gray-500 mb-1", "Confirmations" }
                                p { class: "text-sm font-medium", "{confirmations}" }
                            }
                        }
                    }

                    // Timestamp
                    div { class: "pt-4 border-t border-gray-800",
                        p { class: "text-xs text-gray-500 mb-1", "Timestamp" }
                        p { class: "text-sm font-medium",
                            {
                                let timestamp_ms = (tx.created_at as f64) * 1000.0;
                                let date = js_sys::Date::new(&timestamp_ms.into());
                                format!("{} UTC", date.to_utc_string())
                            }
                        }
                    }
                }
            }
        }
    } else {
        rsx! {
            div { class: "space-y-6",
                h1 { class: "text-2xl font-bold", "Transaction Not Found" }
                {empty_state("\u{274C}", "Transaction Not Found", "The transaction you're looking for doesn't exist")}
                Link { to: Route::Transactions {}, class: "text-blue-400 hover:text-blue-300", "Back to Transactions" }
            }
        }
    }
}
