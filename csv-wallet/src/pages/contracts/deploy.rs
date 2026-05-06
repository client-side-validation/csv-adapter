//! Deploy contract page.
//!
//! Supports contract deployment with file selection following csv-cli flow:
//! - Ethereum: .bin bytecode files + optional ABI
//! - Sui: .mv Move bytecode files + package manifest
//! - Aptos: .mv Move bytecode files + module metadata
//! - Solana: .so BPF program files
//! - Bitcoin: N/A (UTXO-native)

use crate::context::{generate_id, use_wallet_context, DeployedContract, Network};
use crate::pages::common::*;
use crate::routes::Route;
use csv_core::Chain;
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

#[derive(Clone, Debug)]
struct ContractFile {
    name: String,
    content: Vec<u8>,
    file_type: ContractFileType,
}

#[derive(Clone, Debug, PartialEq)]
enum ContractFileType {
    Bytecode,   // .bin, .mv
    Abi,        // .abi, .json
    Manifest,   // Move.toml, package.json
    BpfProgram, // .so files for Solana
}

fn get_file_extension(chain: Chain) -> &'static str {
    match chain {
        Chain::Ethereum => ".bin",
        Chain::Sui => ".mv",
        Chain::Aptos => ".mv",
        Chain::Solana => ".so",
        _ => "",
    }
}

fn get_contract_file_description(chain: Chain) -> &'static str {
    match chain {
        Chain::Ethereum => "Compiled bytecode (.bin file)",
        Chain::Sui => "Move bytecode (.mv files)",
        Chain::Aptos => "Move bytecode (.mv files)",
        Chain::Solana => "BPF program (.so file)",
        _ => "Contract file",
    }
}

#[component]
pub fn DeployContract() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Ethereum);
    let mut selected_network = use_signal(|| Network::Test);
    let mut deployer_key = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);
    let mut error = use_signal(|| Option::<String>::None);

    // Contract files
    let mut main_contract_file = use_signal(|| None::<ContractFile>);
    let mut abi_file = use_signal(|| None::<ContractFile>);
    let mut manifest_file = use_signal(|| None::<ContractFile>);

    let is_bitcoin = *selected_chain.read() == Chain::Bitcoin;

    // File input ID for triggering file selection
    let file_input_id = "contract-file-input";
    let abi_input_id = "abi-file-input";
    let manifest_input_id = "manifest-file-input";

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Contracts {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Deploy Contract" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() {
                        selected_chain.set(c);
                        // Clear files when chain changes
                        main_contract_file.set(None);
                        abi_file.set(None);
                        manifest_file.set(None);
                        error.set(None);
                        result.set(None);
                    }
                }, *selected_chain.read()))}

                {form_field("Network", network_select(move |n| {
                    selected_network.set(n);
                }, *selected_network.read()))}

                if is_bitcoin {
                    div { class: "bg-gray-800/50 rounded-lg p-3 border border-gray-700",
                        p { class: "text-sm text-gray-400",
                            "\u{2139}\u{FE0F} Bitcoin is UTXO-native and does not require contract deployment."
                        }
                    }
                } else {
                    // Contract file selection
                    {form_field(get_contract_file_description(*selected_chain.read()), rsx! {
                        div { class: "space-y-2",
                            input {
                                r#type: "file",
                                id: file_input_id,
                                accept: get_file_extension(*selected_chain.read()),
                                class: "hidden",
                                onchange: move |_| {
                                    if let Some(window) = web_sys::window() {
                                        if let Some(document) = window.document() {
                                            if let Some(el) = document.get_element_by_id(file_input_id) {
                                                if let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>() {
                                                    if let Some(files) = input.files() {
                                                        if files.length() > 0 {
                                                            if let Some(file) = files.get(0) {
                                                                let file_name = file.name();
                                                                let mut file_signal = main_contract_file.clone();
                                                                let reader = web_sys::FileReader::new().ok();
                                                                if let Some(reader) = reader {
                                                                    let onload = Closure::wrap(Box::new(move |e: web_sys::ProgressEvent| {
                                                                        if let Some(target) = e.target() {
                                                                            if let Some(r) = target.dyn_ref::<web_sys::FileReader>() {
                                                                                if let Ok(result) = r.result() {
                                                                                    if let Some(js_array) = result.dyn_ref::<js_sys::Uint8Array>() {
                                                                                        let mut content = vec![0u8; js_array.length() as usize];
                                                                                        js_array.copy_to(&mut content);
                                                                                        file_signal.set(Some(ContractFile {
                                                                                            name: file_name.clone(),
                                                                                            content,
                                                                                            file_type: ContractFileType::Bytecode,
                                                                                        }));
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }) as Box<dyn FnMut(_)>);
                                                                    reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                                                                    onload.forget();
                                                                    let _ = reader.read_as_array_buffer(&file);
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
                            button {
                                onclick: move |_| {
                                    if let Some(window) = web_sys::window() {
                                        if let Some(document) = window.document() {
                                            if let Some(el) = document.get_element_by_id(file_input_id) {
                                                if let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>() {
                                                    input.click();
                                                }
                                            }
                                        }
                                    }
                                },
                                class: "px-4 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 text-sm font-medium transition-colors flex items-center gap-2",
                                span { "\u{1F4C2}" }
                                if main_contract_file.read().is_some() {
                                    "Change File"
                                } else {
                                    "Choose File"
                                }
                            }
                            if let Some(ref file) = main_contract_file.read().as_ref() {
                                div { class: "bg-green-900/30 rounded-lg p-2 text-sm",
                                    p { class: "text-green-300",
                                        "\u{2705} Loaded: {file.name} ({file.content.len()} bytes)"
                                    }
                                }
                            }
                        }
                    })}

                    // ABI file for Ethereum
                    if *selected_chain.read() == Chain::Ethereum {
                        {form_field("Contract ABI (optional)", rsx! {
                            div { class: "space-y-2",
                                input {
                                    r#type: "file",
                                    id: abi_input_id,
                                    accept: ".abi,.json",
                                    class: "hidden",
                                    onchange: move |_| {
                                        if let Some(window) = web_sys::window() {
                                            if let Some(document) = window.document() {
                                                if let Some(el) = document.get_element_by_id(abi_input_id) {
                                                    if let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>() {
                                                        if let Some(files) = input.files() {
                                                            if files.length() > 0 {
                                                                if let Some(file) = files.get(0) {
                                                                    let file_name = file.name();
                                                                    let mut file_signal = abi_file.clone();
                                                                    let reader = web_sys::FileReader::new().ok();
                                                                    if let Some(reader) = reader {
                                                                        let onload = Closure::wrap(Box::new(move |e: web_sys::ProgressEvent| {
                                                                            if let Some(target) = e.target() {
                                                                                if let Some(r) = target.dyn_ref::<web_sys::FileReader>() {
                                                                                    if let Ok(result) = r.result() {
                                                                                        if let Some(text) = result.as_string() {
                                                                                            file_signal.set(Some(ContractFile {
                                                                                                name: file_name.clone(),
                                                                                                content: text.into_bytes(),
                                                                                                file_type: ContractFileType::Abi,
                                                                                            }));
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
                                    }
                                }
                                button {
                                    onclick: move |_| {
                                        if let Some(window) = web_sys::window() {
                                            if let Some(document) = window.document() {
                                                if let Some(el) = document.get_element_by_id(abi_input_id) {
                                                    if let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>() {
                                                        input.click();
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    class: "px-4 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 text-sm font-medium transition-colors flex items-center gap-2",
                                    span { "\u{1F4C2}" }
                                    if abi_file.read().is_some() {
                                        "Change ABI"
                                    } else {
                                        "Choose ABI"
                                    }
                                }
                                if let Some(ref file) = abi_file.read().as_ref() {
                                    div { class: "bg-blue-900/30 rounded-lg p-2 text-sm",
                                        p { class: "text-blue-300",
                                            "\u{2705} Loaded: {file.name} ({file.content.len()} bytes)"
                                        }
                                    }
                                }
                            }
                        })}
                    }

                    // Manifest for Move chains (Sui, Aptos)
                    if matches!(*selected_chain.read(), Chain::Sui | Chain::Aptos) {
                        {form_field("Package Manifest (optional)", rsx! {
                            div { class: "space-y-2",
                                input {
                                    r#type: "file",
                                    id: manifest_input_id,
                                    accept: ".toml",
                                    class: "hidden",
                                    onchange: move |_| {
                                        if let Some(window) = web_sys::window() {
                                            if let Some(document) = window.document() {
                                                if let Some(el) = document.get_element_by_id(manifest_input_id) {
                                                    if let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>() {
                                                        if let Some(files) = input.files() {
                                                            if files.length() > 0 {
                                                                if let Some(file) = files.get(0) {
                                                                    let file_name = file.name();
                                                                    let mut file_signal = manifest_file.clone();
                                                                    let reader = web_sys::FileReader::new().ok();
                                                                    if let Some(reader) = reader {
                                                                        let onload = Closure::wrap(Box::new(move |e: web_sys::ProgressEvent| {
                                                                            if let Some(target) = e.target() {
                                                                                if let Some(r) = target.dyn_ref::<web_sys::FileReader>() {
                                                                                    if let Ok(result) = r.result() {
                                                                                        if let Some(text) = result.as_string() {
                                                                                            file_signal.set(Some(ContractFile {
                                                                                                name: file_name.clone(),
                                                                                                content: text.into_bytes(),
                                                                                                file_type: ContractFileType::Manifest,
                                                                                            }));
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
                                    }
                                }
                                button {
                                    onclick: move |_| {
                                        if let Some(window) = web_sys::window() {
                                            if let Some(document) = window.document() {
                                                if let Some(el) = document.get_element_by_id(manifest_input_id) {
                                                    if let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>() {
                                                        input.click();
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    class: "px-4 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 text-sm font-medium transition-colors flex items-center gap-2",
                                    span { "\u{1F4C2}" }
                                    if manifest_file.read().is_some() {
                                        "Change Manifest"
                                    } else {
                                        "Choose Manifest"
                                    }
                                }
                                if let Some(ref file) = manifest_file.read().as_ref() {
                                    div { class: "bg-blue-900/30 rounded-lg p-2 text-sm",
                                        p { class: "text-blue-300",
                                            "\u{2705} Loaded: {file.name} ({file.content.len()} bytes)"
                                        }
                                    }
                                }
                            }
                        })}
                    }

                    {form_field("Deployer Private Key", rsx! {
                        input {
                            value: "{deployer_key.read()}",
                            oninput: move |evt| { deployer_key.set(evt.value()); error.set(None); },
                            class: "{input_mono_class()}",
                            r#type: "text"
                        }
                    })}
                }

                if let Some(e) = error.read().as_ref() {
                    div { class: "p-3 bg-red-900/30 border border-red-700/50 rounded-lg text-sm text-red-300", "{e}" }
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
                            // Validate that contract file is provided
                            if main_contract_file.read().is_none() {
                                error.set(Some(format!("Please select a {} contract file to deploy.", get_contract_file_description(*selected_chain.read()))));
                                return;
                            }

                            // In a real implementation, we would:
                            // 1. Validate the contract bytecode
                            // 2. Estimate gas costs
                            // 3. Deploy to the selected chain
                            // 4. Store the deployment result

                            let addr = generate_id();
                            wallet_ctx.add_contract(DeployedContract {
                                chain: *selected_chain.read(),
                                address: addr.clone(),
                                tx_hash: generate_id(),
                                deployed_at: js_sys::Date::now() as u64 / 1000,
                            });

                            let chain_name_str = chain_name(&selected_chain.read());
                            result.set(Some(format!(
                                "Contract deployed successfully on {}!\n\nAddress: {}\n\nNote: This is a preview. Real deployment would:\n1. Upload {} bytes of bytecode\n2. Execute deployment transaction\n3. Wait for confirmation",
                                chain_name_str,
                                addr,
                                main_contract_file.read().as_ref().map(|f| f.content.len()).unwrap_or(0)
                            )));
                            error.set(None);
                        }
                    },
                    disabled: !is_bitcoin && main_contract_file.read().is_none(),
                    class: "{btn_full_primary_class()} disabled:opacity-50 disabled:cursor-not-allowed",
                    if is_bitcoin {
                        "Not Applicable"
                    } else if main_contract_file.read().is_none() {
                        "Select Contract File"
                    } else {
                        "Deploy Contract"
                    }
                }

                if !is_bitcoin {
                    div { class: "bg-blue-500/10 border border-blue-500/20 rounded-lg p-4 text-sm text-gray-400",
                        p { class: "mb-2", span { class: "text-blue-400 font-medium", "\u{2139}\u{FE0F} Deployment Info:" } }
                        p { "For "
                            span { class: "text-blue-300", "Ethereum" }
                            ": Provide compiled .bin bytecode and optional ABI"
                        }
                        p { "For "
                            span { class: "text-blue-300", "Sui/Aptos" }
                            ": Provide Move .mv bytecode files"
                        }
                        p { "For "
                            span { class: "text-blue-300", "Solana" }
                            ": Provide compiled .so BPF program"
                        }
                    }
                }
            }
        }
    }
}
