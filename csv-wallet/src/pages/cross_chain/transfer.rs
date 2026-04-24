//! Cross-chain transfer page.

use crate::context::{use_wallet_context, RightStatus, TrackedRight, TrackedTransfer, TransferStatus};
use crate::pages::common::*;
use crate::routes::Route;
use csv_adapter_core::Chain;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn CrossChainTransfer() -> Element {
    let wallet_ctx = use_wallet_context();
    let mut from_chain = use_signal(|| Chain::Bitcoin);
    let mut to_chain = use_signal(|| Chain::Sui);
    let mut selected_right_index = use_signal(|| 0usize);
    let mut dest_owner = use_signal(String::new);
    let mut step = use_signal(|| 0);
    let result = use_signal(|| Option::<String>::None);
    let mut error = use_signal(|| Option::<String>::None);
    let mut executing = use_signal(|| false);
    let mut selected_account_index = use_signal(|| 0usize);
    let mut selected_target_contract_index = use_signal(|| 0usize);

    // Get rights for the source chain (filtered to active only)
    let from_chain_val = *from_chain.read();
    let rights_for_source: Vec<_> = wallet_ctx.rights_for_chain(from_chain_val)
        .into_iter()
        .filter(|r| r.status == RightStatus::Active)
        .collect();
    let has_rights = !rights_for_source.is_empty();
    
    // Reset right selection when chain changes
    use_effect(move || {
        selected_right_index.set(0);
    });
    
    // Clone for use in memo
    let rights_for_memo = rights_for_source.clone();
    // Get the selected right ID
    let right_id = use_memo(move || {
        rights_for_memo.get(*selected_right_index.read())
            .map(|r| r.id.clone())
            .unwrap_or_default()
    });

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

    // Check for globally selected contract and pre-populate if it matches target chain
    use_effect({
        let target_contracts = target_contracts.clone();
        let selected = wallet_ctx.selected_contract();
        move || {
            if let Some(ref contract) = selected {
                // Find the contract in target contracts list
                if let Some(index) = target_contracts.iter().position(|c| {
                    c.chain == contract.chain && c.address == contract.address
                }) {
                    selected_target_contract_index.set(index);
                }
            }
        }
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

        if !has_rights {
            error.set(Some(format!("No active rights available for {:?}. Create a right first.", *from_chain.read())));
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
                use crate::services::blockchain::{BlockchainConfig, BlockchainService, NativeWallet};
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
                        contracts.insert(to, crate::services::blockchain::ContractDeployment {
                            chain: to,
                            contract_address: contract.address.clone(),
                            tx_hash: contract.tx_hash.clone(),
                            deployed_at: contract.deployed_at,
                            contract_type: crate::services::blockchain::ContractType::Lock,
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
                                option { key: "{account.id}", value: idx.to_string(), selected: idx == *selected_account_index.read(),
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
                                for (idx, contract) in source_contracts.iter().enumerate() {
                                    p { key: "{idx}", class: "text-xs text-green-400 font-mono",
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
                                            option { key: "{idx}", value: idx.to_string(), selected: idx == *selected_target_contract_index.read(),
                                                {format!("{}...", &contract.address[..12.min(contract.address.len())])}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                {form_field("Available Rights", rsx! {
                    if rights_for_source.is_empty() {
                        p { class: "text-sm text-red-400", 
                            {format!("No active rights available for {:?}. Create a right on this chain first.", from_chain_val)} 
                        }
                    } else {
                        select {
                            class: "{input_mono_class()}",
                            onchange: move |evt| {
                                if let Ok(idx) = evt.value().parse::<usize>() {
                                    selected_right_index.set(idx);
                                }
                            },
                            for (idx, right) in rights_for_source.iter().enumerate() {
                                option { key: "{idx}", value: idx.to_string(), selected: idx == *selected_right_index.read(),
                                    {format!("{}... - Value: {} - {}",
                                        &right.id[..16.min(right.id.len())],
                                        right.value,
                                        right.status
                                    )}
                                }
                            }
                        }
                    }
                })}
                
                // Show selected right details
                if let Some(right) = rights_for_source.get(*selected_right_index.read()) {
                    div { class: "bg-gray-800/50 rounded-lg p-3 border border-gray-700",
                        p { class: "text-xs text-gray-400 mb-2", "Selected Right Details:" }
                        div { class: "grid grid-cols-2 gap-2 text-xs",
                            div { span { class: "text-gray-500", "Full ID: " }, span { class: "font-mono text-gray-300 break-all", "{&right.id}" } }
                            div { span { class: "text-gray-500", "Value: " }, span { class: "font-mono text-gray-300", "{right.value}" } }
                            div { span { class: "text-gray-500", "Status: " }, span { class: "{right_status_class(&right.status)}", "{right.status}" } }
                            div { span { class: "text-gray-500", "Owner: " }, span { class: "font-mono text-gray-300", "{truncate_address(&right.owner, 8)}" } }
                        }
                    }
                }

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
                            div { key: "{i}", class: "flex items-center gap-2",
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
                        || !has_rights
                        || !has_account
                        || !has_target_contract,
                    class: "{btn_full_primary_class()}",
                    if *executing.read() {
                        "Executing..."
                    } else if !has_account {
                        "Add Source Account First"
                    } else if !has_rights {
                        "No Rights Available"
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
                if !has_rights {
                    p { class: "text-xs text-red-500 mt-2",
                        {format!("Note: Create a Right on {:?} source chain first", *from_chain.read())}
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
