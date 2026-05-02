//! Wallet Management page with per-chain account management, export/import JSON.

use crate::chains::supported_wallet_chains;
use crate::components::{all_chain_displays, Card, ChainDisplay};
use crate::context::{use_wallet_context, WalletContext};
use crate::routes::Route;
use crate::wallet_core::ChainAccount;
use csv_adapter_core::Chain;
use dioxus::prelude::*;
use wasm_bindgen::prelude::*;

const EXPECTED_JSON_EXAMPLE: &str = r#"{"accounts": [{"id": "...", "chain": "bitcoin", "name": "...", "private_key": "...", "address": "..."}], "selected_account_id": null}"#;

#[derive(Clone, Copy, PartialEq)]
enum WalletTab {
    Accounts,
    AddAccount,
    Export,
    Import,
}

impl std::fmt::Display for WalletTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Accounts => write!(f, "Accounts"),
            Self::AddAccount => write!(f, "Add Account"),
            Self::Export => write!(f, "Export"),
            Self::Import => write!(f, "Import"),
        }
    }
}

#[component]
pub fn WalletPage() -> Element {
    let wallet_ctx = use_wallet_context();
    let accounts = wallet_ctx.accounts();
    let mut active_tab = use_signal(|| WalletTab::Accounts);
    let account_status_text = if !accounts.is_empty() {
        format!("{} accounts", accounts.len())
    } else {
        "No accounts".to_string()
    };

    let tabs = vec![
        WalletTab::Accounts,
        WalletTab::AddAccount,
        WalletTab::Export,
        WalletTab::Import,
    ];

    rsx! {
        div { class: "space-y-6 stagger-children",
            div { class: "flex items-center justify-between",
                h1 { class: "text-3xl font-bold text-gray-100", "Wallet Management" }
                div { class: "flex items-center gap-2 text-sm text-gray-400",
                    span { class: "w-2 h-2 rounded-full", class: if !accounts.is_empty() { "bg-green-500 status-online" } else { "bg-yellow-500" } }
                    "{account_status_text}"
                }
            }

            // Tab navigation
            div { class: "bg-gray-900 rounded-xl border border-gray-800 p-1",
                div { class: "flex gap-1 overflow-x-auto",
                    for tab in tabs {
                        button {
                            key: "tab-{tab}",
                            onclick: move |_| active_tab.set(tab),
                            class: "px-4 py-2 rounded-lg text-sm font-medium transition-all whitespace-nowrap",
                            class: if active_tab() == tab { "bg-blue-600 text-white" } else { "text-gray-400 hover:text-gray-200 hover:bg-gray-800" },
                            "{tab}"
                        }
                    }
                }
            }

            // Tab content
            match active_tab() {
                WalletTab::Accounts => rsx! { AccountsTab {} },
                WalletTab::AddAccount => rsx! { AddAccountTab {} },
                WalletTab::Export => rsx! { ExportTab {} },
                WalletTab::Import => rsx! { ImportTab {} },
            }
        }
    }
}

#[component]
fn AccountsTab() -> Element {
    let wallet_ctx = use_wallet_context();
    let accounts = wallet_ctx.accounts();

    if accounts.is_empty() {
        return rsx! {
            Card {
                title: "Accounts",
                children: rsx! {
                    div { class: "text-center py-12 space-y-3",
                        div { class: "text-5xl", "\u{1F4CB}" }
                        p { class: "text-gray-400 text-lg", "No accounts" }
                        p { class: "text-sm text-gray-500", "Add accounts per-chain or import from JSON" }
                        Link { to: Route::Dashboard {}, class: "inline-block mt-4 px-6 py-3 bg-blue-600 hover:bg-blue-700 rounded-lg text-sm font-medium transition-colors", "Go to Dashboard" }
                    }
                }
            }
        };
    }

    rsx! {
        div { class: "space-y-6",
            for chain in supported_wallet_chains() {
                ChainAccountsSection { key: "chain-{chain:?}", chain }
            }
        }
    }
}

#[component]
fn ChainAccountsSection(chain: Chain) -> Element {
    let wallet_ctx = use_wallet_context();
    let chain_accounts = wallet_ctx.accounts_for_chain(chain);

    if chain_accounts.is_empty() {
        return rsx! {};
    }

    rsx! {
        Card {
            title: format!("{chain_name} ({count})", chain_name = chain_name(&chain), count = chain_accounts.len()),
            children: rsx! {
                div { class: "space-y-2",
                    for account in chain_accounts {
                        ChainAccountRow { key: "account-{account.id}", account: account.clone(), wallet_ctx: wallet_ctx.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn ChainAccountRow(account: ChainAccount, mut wallet_ctx: WalletContext) -> Element {
    let mut show_key = use_signal(|| false);
    let _account_clone = account.clone();
    let is_bitcoin = account.chain == Chain::Bitcoin;

    rsx! {
        div { class: "flex items-center justify-between bg-gray-800/50 rounded-lg p-3",
            div { class: "flex-1 min-w-0",
                p { class: "font-mono text-sm text-gray-200 truncate", "{account.address}" }
                p { class: "text-xs text-gray-500 mt-0.5", "{account.name}" }
                if show_key() {
                    div { class: "mt-2 p-2 bg-gray-900 rounded",
                        p { class: "text-xs text-yellow-400 font-mono break-all", "[Private key stored in keystore]" }
                    }
                }
            }
            div { class: "flex gap-2",
                if is_bitcoin {
                    button {
                        onclick: {
                            let mut wallet_ctx = wallet_ctx.clone();
                            let account_id = account.id.clone();
                            move |_| {
                                if let Ok(updated) = wallet_ctx.refresh_account_address(&account_id) {
                                    if updated {
                                        web_sys::console::log_1(&"Bitcoin address refreshed to new Taproot derivation".into());
                                    }
                                }
                            }
                        },
                        class: "px-2 py-1 rounded text-xs bg-blue-900/30 text-blue-400 hover:bg-blue-900/50 transition-colors",
                        "Refresh Address"
                    }
                }
                button {
                    onclick: move |_| {
                        show_key.set(!show_key());
                    },
                    class: "px-2 py-1 rounded text-xs bg-gray-700 text-gray-300 hover:bg-gray-600 transition-colors",
                    if show_key() { "Hide Key" } else { "Show Key" }
                }
                button {
                    onclick: {
                        let mut wallet_ctx = wallet_ctx.clone();
                        let chain = account.chain;
                        let address = account.address.clone();
                        move |_| {
                            wallet_ctx.remove_account(chain, &address);
                        }
                    },
                    class: "px-2 py-1 rounded text-xs bg-red-900/30 text-red-400 hover:bg-red-900/50 transition-colors",
                    "Remove"
                }
            }
        }
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

#[component]
fn AddAccountTab() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| ChainDisplay(Chain::Bitcoin));
    let mut pk_input = use_signal(String::new);
    let mut name_input = use_signal(String::new);
    let mut message = use_signal(|| Option::<String>::None);
    let mut error = use_signal(|| Option::<String>::None);

    rsx! {
        Card {
            title: "Add Account",
            children: rsx! {
                div { class: "space-y-6 stagger-children",
                    div {
                        label { class: "block text-sm font-medium text-gray-300 mb-2", "Blockchain" }
                        div { class: "relative",
                            select {
                                class: "w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500",
                                value: "{selected_chain.read()}",
                                onchange: move |evt| {
                                    let val = evt.value();
                                    if let Ok(c) = val.parse::<Chain>() {
                                        selected_chain.set(ChainDisplay(c));
                                    }
                                },
                                for cd in all_chain_displays() {
                                    option { key: "chain-opt-{cd.0}", value: "{cd.0}", selected: cd.0 == selected_chain.read().0, "{cd.0}" }
                                }
                            }
                        }
                    }

                    div {
                        label { class: "block text-sm font-medium text-gray-300 mb-2", "Account Name (optional)" }
                        input {
                            value: "{name_input.read()}",
                            oninput: move |evt| name_input.set(evt.value()),
                            class: "w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500",
                            r#type: "text"
                        }
                    }

                    div {
                        label { class: "block text-sm font-medium text-gray-300 mb-2", "Private Key (hex)" }
                        textarea {
                            value: "{pk_input.read()}",
                            oninput: move |evt| { pk_input.set(evt.value()); error.set(None); },
                            class: "w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-3 text-sm text-gray-100 font-mono focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none",
                            rows: 3,
                        }
                        button {
                            onclick: move |_| {
                                let chain = selected_chain.read().0;
                                let key = generate_key_for_chain(chain);
                                pk_input.set(key);
                                error.set(None);
                            },
                            class: "mt-2 px-4 py-2 bg-green-600 hover:bg-green-700 rounded-lg text-sm font-medium transition-all duration-200 text-white",
                            "Generate Key"
                        }
                    }

                    if let Some(e) = error.read().clone() {
                        div { class: "bg-red-500/10 border border-red-500/30 rounded-xl p-4 text-sm text-red-300 flex items-center justify-between",
                            span { "{e}" }
                            button { onclick: move |_| error.set(None), class: "text-red-400", "\u{2715}" }
                        }
                    }

                    if let Some(msg) = message.read().clone() {
                        div { class: "bg-green-500/10 border border-green-500/30 rounded-xl p-4 text-sm text-green-300 flex items-center justify-between",
                            span { "{msg}" }
                            button { onclick: move |_| message.set(None), class: "text-green-400", "\u{2715}" }
                        }
                    }

                    button {
                        onclick: move |_| {
                            let chain = selected_chain.read().0;
                            let name = {
                                let n = name_input.read().clone();
                                if n.is_empty() { format!("{:?}", chain) } else { n }
                            };
                            let pk = pk_input.read().clone();
                            // Create account from private key via keystore
                            match wallet_ctx.import_account_from_key(chain, &name, &pk) {
                                Ok(_) => {
                                    message.set(Some(format!("Account added for {}!", chain)));
                                    pk_input.set(String::new());
                                    name_input.set(String::new());
                                }
                                Err(e) => error.set(Some(e.to_string())),
                            }
                        },
                        class: "w-full px-6 py-3 bg-blue-600 hover:bg-blue-700 rounded-lg font-medium transition-all duration-200 text-white btn-ripple",
                        "Add Account"
                    }

                    div { class: "bg-blue-500/10 border border-blue-500/20 rounded-lg p-4 text-sm text-gray-400",
                        span { class: "text-blue-400 font-medium", "\u{2139}\u{FE0F} Tip: " }
                        "You can add multiple accounts per chain. Each account needs its own private key."
                    }
                }
            }
        }
    }
}

#[component]
fn ExportTab() -> Element {
    let wallet_ctx = use_wallet_context();
    let accounts = wallet_ctx.accounts();
    let mut message = use_signal(|| Option::<String>::None);
    let mut error = use_signal(|| Option::<String>::None);

    rsx! {
        Card {
            title: "Export Wallet",
            children: rsx! {
                div { class: "space-y-6 stagger-children",
                    if let Some(e) = error.read().clone() {
                        div { class: "bg-red-500/10 border border-red-500/30 rounded-xl p-4 text-sm text-red-300", "{e}" }
                    }
                    if let Some(msg) = message.read().clone() {
                        div { class: "bg-green-500/10 border border-green-500/30 rounded-xl p-4 text-sm text-green-300", "{msg}" }
                    }

                    div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700",
                        p { class: "text-sm text-gray-400 mb-2", "Accounts to export:" }
                        p { class: "text-lg font-bold text-gray-100", "{accounts.len()}" }
                    }

                    button {
                        onclick: move |_| {
                            match wallet_ctx.export_wallet_json() {
                                Ok(json) => {
                                    // Trigger browser download
                                    trigger_download("csv-wallet-export.json", &json);
                                    message.set(Some("Wallet exported! Check your downloads folder.".to_string()));
                                }
                                Err(e) => error.set(Some(e)),
                            }
                        },
                        disabled: accounts.is_empty(),
                        class: "w-full px-6 py-3 bg-blue-600 hover:bg-blue-700 rounded-lg font-medium transition-all duration-200 text-white btn-ripple disabled:opacity-50 disabled:cursor-not-allowed",
                        "\u{1F4E4} Download JSON File"
                    }

                    div { class: "bg-yellow-500/10 border border-yellow-500/20 rounded-lg p-4 text-sm text-gray-400",
                        span { class: "text-yellow-400 font-medium", "\u{26A0}\u{FE0F} Warning: " }
                        "The exported file contains all your private keys. Store it securely and never share it."
                    }
                }
            }
        }
    }
}

#[component]
fn ImportTab() -> Element {
    let wallet_ctx = use_wallet_context();
    let message = use_signal(|| Option::<String>::None);
    let mut error = use_signal(|| Option::<String>::None);
    let mut success = use_signal(|| false);

    if *success.read() {
        return rsx! {
            Card {
                title: "Import Complete",
                children: rsx! {
                    div { class: "text-center py-8 space-y-4",
                        div { class: "text-green-400 text-4xl", "\u{2705}" }
                        p { class: "text-green-400 text-lg font-medium", "Wallet imported successfully!" }
                        p { class: "text-sm text-gray-400", "All accounts have been loaded." }
                        Link { to: Route::Dashboard {}, class: "inline-block px-6 py-3 bg-blue-600 hover:bg-blue-700 rounded-lg text-sm font-medium transition-colors", "Go to Dashboard" }
                    }
                }
            }
        };
    }

    rsx! {
        Card {
            title: "Import Wallet from JSON",
            children: rsx! {
                div { class: "space-y-6 stagger-children",
                    if let Some(e) = error.read().clone() {
                        div { class: "bg-red-500/10 border border-red-500/30 rounded-xl p-4 text-sm text-red-300", "{e}" }
                    }
                    if let Some(msg) = message.read().clone() {
                        div { class: "bg-green-500/10 border border-green-500/30 rounded-xl p-4 text-sm text-green-300", "{msg}" }
                    }

                    input {
                        r#type: "file",
                        accept: ".json",
                        id: "wallet-import-input",
                        class: "w-full text-sm text-gray-400 file:mr-4 file:py-2.5 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-medium file:bg-gray-800 file:text-gray-300 hover:file:bg-gray-700 cursor-pointer",
                        onchange: move |_| {
                            if let Some(window) = web_sys::window() {
                                if let Some(document) = window.document() {
                                    if let Some(el) = document.get_element_by_id("wallet-import-input") {
                                        if let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>() {
                                            if let Some(files) = input.files() {
                                                if files.length() > 0 {
                                                    if let Some(file) = files.get(0) {
                                                        let mut ctx = wallet_ctx.clone();
                                                        let reader = web_sys::FileReader::new().ok();
                                                        if let Some(reader) = reader {
                                                            let onload = Closure::wrap(Box::new(move |e: web_sys::ProgressEvent| {
                                                                if let Some(target) = e.target() {
                                                                    if let Some(r) = target.dyn_ref::<web_sys::FileReader>() {
                                                                        if let Ok(result) = r.result() {
                                                                            if let Some(text) = result.as_string() {
                                                                                match ctx.import_wallet_json(&text) {
                                                                                    Ok(()) => success.set(true),
                                                                                    Err(e) => error.set(Some(e)),
                                                                                }
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

                    div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700 text-sm text-gray-400",
                        p { class: "font-medium mb-1", "Expected JSON format:" }
                        pre { class: "text-xs text-gray-500 overflow-x-auto", "{EXPECTED_JSON_EXAMPLE}" }
                    }
                }
            }
        }
    }
}

fn trigger_download(filename: &str, content: &str) {
    if let Some(window) = web_sys::window() {
        let opts = web_sys::BlobPropertyBag::new();
        opts.set_type("application/json");
        let blob = web_sys::Blob::new_with_str_sequence_and_options(
            &js_sys::Array::from_iter([js_sys::JsString::from(content)]),
            &opts,
        )
        .ok();

        if let Some(blob) = blob {
            let url = web_sys::Url::create_object_url_with_blob(&blob).ok();
            if let Some(url) = url {
                let a = window.document().and_then(|d| d.create_element("a").ok());
                if let Some(a) = a {
                    if let Some(a) = a.dyn_ref::<web_sys::HtmlAnchorElement>() {
                        a.set_href(&url);
                        a.set_download(filename);
                        a.click();
                    }
                }
                let _ = web_sys::Url::revoke_object_url(&url);
            }
        }
    }
}

/// Generate a random 32-byte private key for any chain.
/// All supported chains (Bitcoin/Ethereum secp256k1, Sui/Aptos/Solana ed25519) use 32-byte keys.
fn generate_key_for_chain(_chain: Chain) -> String {
    use rand::rngs::OsRng;
    use rand::RngCore;

    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    format!("0x{}", hex::encode(key))
}
