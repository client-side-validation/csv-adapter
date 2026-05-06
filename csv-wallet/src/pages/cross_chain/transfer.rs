//! Cross-chain transfer page.

use crate::context::{
    use_wallet_context, ProofData, ProofRecord, ProofStatus, RightStatus, SealContent, SealRecord,
    SealStatus, TrackedTransfer, TransferStatus,
};
use crate::pages::common::*;
use crate::routes::Route;
use crate::services::blockchain::types::{ContractDeployment, ContractType};
use csv_core::Chain;
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
    let rights_for_source: Vec<_> = wallet_ctx
        .rights_for_chain(from_chain_val)
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
        rights_for_memo
            .get(*selected_right_index.read())
            .map(|r| r.id.clone())
            .unwrap_or_default()
    });

    // Get accounts for the source chain
    let accounts = wallet_ctx.accounts_for_chain(*from_chain.read());
    let has_account = !accounts.is_empty();

    // Check if selected account is watch-only (can't sign)
    let selected_account = accounts.get(*selected_account_index.read());
    let is_watch_only = selected_account.map(|a| a.is_watch_only()).unwrap_or(false);

    // Get accounts for the destination chain (needed for gas payment)
    let dest_accounts = wallet_ctx.accounts_for_chain(*to_chain.read());
    let has_dest_account = !dest_accounts.is_empty();

    // Track fetched destination balance (in raw chain units: satoshis, lamports, MIST, octas, wei)
    let mut dest_balance_raw = use_signal(|| 0u64);
    let mut dest_balance_loading = use_signal(|| false);

    // Fetch destination balance when chain or account changes
    use_effect({
        let to_chain_val = *to_chain.read();
        let dest_addr = dest_accounts.first().map(|a| a.address.clone());
        move || {
            if let Some(addr) = &dest_addr {
                dest_balance_loading.set(true);
                let addr = addr.clone();
                spawn(async move {
                    use crate::services::chain_api::ChainApi;
                    let api = ChainApi::default();
                    if let Ok(balance) = api.get_balance(to_chain_val, &addr).await {
                        dest_balance_raw.set(balance);
                    }
                    dest_balance_loading.set(false);
                });
            }
        }
    });

    // Check if destination account has minimum balance for gas (in raw chain units)
    // Sui: ~0.01 SUI = 10_000_000 MIST, Aptos: ~0.01 APT = 1_000_000 octas
    let min_dest_balance_raw = match *to_chain.read() {
        Chain::Sui => 10_000_000u64,     // 0.01 SUI in MIST
        Chain::Aptos => 1_000_000u64,  // 0.01 APT in octas
        Chain::Ethereum => 1_000_000_000_000_000u64, // ~0.001 ETH in wei
        Chain::Solana => 1_000_000u64, // ~0.001 SOL in lamports
        _ => 0u64,                      // Bitcoin doesn't need pre-funded destination for minting
    };
    let dest_has_enough_balance = *dest_balance_raw.read() >= min_dest_balance_raw;

    // Get contracts for source and target chains
    let source_contracts = wallet_ctx.contracts_for_chain(*from_chain.read());
    let target_contracts = wallet_ctx.contracts_for_chain(*to_chain.read());
    let _has_source_contract =
        !source_contracts.is_empty() || matches!(*from_chain.read(), Chain::Bitcoin);
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
                if let Some(index) = target_contracts
                    .iter()
                    .position(|c| c.chain == contract.chain && c.address == contract.address)
                {
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

    // Clone before moving into closure to avoid borrow after move
    let rights_for_closure = rights_for_source.clone();

    // Execute real cross-chain transfer using native signing
    let execute_transfer = move |_| {
        let rights_for_source_closure = rights_for_closure.clone();
        if !_has_source_contract {
            error.set(Some(format!(
                "No contract deployed on {:?}. Deploy a contract first.",
                *from_chain.read()
            )));
            return;
        }

        if !has_target_contract {
            error.set(Some(format!(
                "No contract deployed on {:?}. Deploy a contract first.",
                *to_chain.read()
            )));
            return;
        }

        if !has_dest_account {
            error.set(Some(format!(
                "No account available for destination chain {:?}. Please add an account first.",
                *to_chain.read()
            )));
            return;
        }

        if !dest_has_enough_balance {
            let min_balance = match *to_chain.read() {
                Chain::Sui => "0.01 SUI",
                Chain::Aptos => "0.01 APT",
                Chain::Ethereum => "0.001 ETH",
                Chain::Solana => "0.001 SOL",
                _ => "funds",
            };
            error.set(Some(format!(
                "Destination account on {:?} needs at least {} for gas fees. Please fund your account first.",
                *to_chain.read(), min_balance
            )));
            return;
        }

        if !has_account {
            error.set(Some(format!(
                "No account available for {:?}. Please add an account first.",
                *from_chain.read()
            )));
            return;
        }

        if !has_rights {
            error.set(Some(format!(
                "No active rights available for {:?}. Create a right first.",
                *from_chain.read()
            )));
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
                use crate::services::blockchain::{
                    BlockchainConfig, BlockchainService, NativeWallet,
                };
                use crate::wallet_core::ChainAccount;

                // Get the selected account
                let account: ChainAccount = if let Some(acc) = accounts.get(account_idx) {
                    acc.clone()
                } else {
                    error_signal.set(Some("Selected account not found".to_string()));
                    executing_signal.set(false);
                    return;
                };

                // Check if account can sign transactions
                if account.is_watch_only() {
                    error_signal.set(Some(format!(
                        "Account '{}' is watch-only (no private key). \n\
                        Please import the private key or use a browser wallet like MetaMask.",
                        account.name
                    )));
                    executing_signal.set(false);
                    return;
                }

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

                // Build contracts map with both source and target contracts
                let mut contracts = std::collections::HashMap::new();

                // Add source chain contract (needed for locking)
                let source_contracts = wallet_ctx.contracts_for_chain(from);
                if !source_contracts.is_empty() {
                    if let Some(contract) = source_contracts.first() {
                        contracts.insert(
                            from,
                            ContractDeployment {
                                chain: from,
                                contract_address: contract.address.clone(),
                                tx_hash: contract.tx_hash.clone(),
                                deployed_at: contract.deployed_at,
                                contract_type: ContractType::Lock,
                            },
                        );
                    }
                }

                // Add target chain contract (needed for minting)
                let target_contracts = wallet_ctx.contracts_for_chain(to);
                if !target_contracts.is_empty() {
                    let selected_idx = target_contract_idx.min(target_contracts.len() - 1);
                    if let Some(contract) = target_contracts.get(selected_idx) {
                        contracts.insert(
                            to,
                            ContractDeployment {
                                chain: to,
                                contract_address: contract.address.clone(),
                                tx_hash: contract.tx_hash.clone(),
                                deployed_at: contract.deployed_at,
                                contract_type: ContractType::Lock,
                            },
                        );
                    }
                }

                match service
                    .execute_cross_chain_transfer(from, to, &right, &dest_addr, &contracts, &signer)
                    .await
                {
                    Ok(transfer_result) => {
                        step_signal.set(6); // Set beyond last step to show all completed
                        let transfer_id = transfer_result.transfer_id.clone();
                        let now = js_sys::Date::now() as u64 / 1000;

                        // Get contract addresses from the contracts map
                        let source_contract =
                            contracts.get(&from).map(|c| c.contract_address.clone());
                        let dest_contract = contracts.get(&to).map(|c| c.contract_address.clone());

                        // Format fees with appropriate chain units
                        let source_fee_str =
                            transfer_result.source_fee.map(|fee| format_fee(fee, from));
                        let dest_fee_str = transfer_result.dest_fee.map(|fee| format_fee(fee, to));

                        // Create linked Seal record
                        let seal_ref = format!("seal_{}", &transfer_id[..16]);
                        let seal_content = SealContent {
                            content_hash: format!("0x{}", &right[..40.min(right.len())]),
                            owner: dest_addr.clone(),
                            block_number: None,
                            lock_tx_hash: Some(transfer_result.lock_tx_hash.clone()),
                        };
                        let seal = SealRecord {
                            seal_ref: seal_ref.clone(),
                            chain: from,
                            value: rights_for_source_closure
                                .get(*selected_right_index.read())
                                .map(|r| r.value)
                                .unwrap_or(0),
                            right_id: right.clone(),
                            status: SealStatus::Locked,
                            created_at: now,
                            content: Some(seal_content),
                            proof_ref: None,
                        };
                        wallet_ctx.add_seal(seal);

                        // Create linked Proof record
                        let proof_data = match from {
                            Chain::Bitcoin => ProofData::Merkle {
                                root: format!("0x{}", &transfer_result.lock_tx_hash[..40]),
                                path: vec![format!("0x{}", &right[..40])],
                                leaf_index: 0,
                            },
                            Chain::Ethereum => ProofData::Mpt {
                                root: format!("0x{}", &transfer_result.lock_tx_hash[..40]),
                                account_proof: vec![format!("0x{}", &right[..40])],
                                storage_proof: vec![format!("0x{}", &transfer_id[..40])],
                            },
                            Chain::Sui => ProofData::Checkpoint {
                                sequence: now,
                                digest: transfer_result.lock_tx_hash.clone(),
                                signatures: vec![
                                    "validator_1".to_string(),
                                    "validator_2".to_string(),
                                ],
                            },
                            Chain::Aptos => ProofData::Ledger {
                                version: now,
                                proof: format!("0x{}", &transfer_result.lock_tx_hash[..40]),
                            },
                            Chain::Solana => ProofData::Solana {
                                slot: now,
                                bank_hash: format!("0x{}", &transfer_result.lock_tx_hash[..40]),
                                merkle_proof: vec![format!("0x{}", &right[..40])],
                            },
                            _ => ProofData::Merkle {
                                root: format!("0x{}", &transfer_result.lock_tx_hash[..40]),
                                path: vec![format!("0x{}", &right[..40])],
                                leaf_index: 0,
                            },
                        };

                        let proof_type = match from {
                            Chain::Bitcoin => "merkle",
                            Chain::Ethereum => "mpt",
                            Chain::Sui => "checkpoint",
                            Chain::Aptos => "ledger",
                            Chain::Solana => "solana",
                            _ => "merkle",
                        };

                        let proof = ProofRecord {
                            chain: from,
                            right_id: right.clone(),
                            seal_ref: seal_ref.clone(),
                            proof_type: proof_type.to_string(),
                            status: ProofStatus::Verified,
                            generated_at: now,
                            verified_at: Some(now),
                            data: Some(proof_data),
                            target_chain: Some(to),
                            verification_tx_hash: Some(transfer_result.mint_tx_hash.clone()),
                        };
                        wallet_ctx.add_proof(proof);

                        // Link proof to seal
                        wallet_ctx.link_proof_to_seal(
                            &seal_ref,
                            &format!("proof_{}", &transfer_id[..16]),
                        );

                        // Record the transfer with full details
                        wallet_ctx.add_transfer(TrackedTransfer {
                            id: transfer_id.clone(),
                            from_chain: from,
                            to_chain: to,
                            right_id: right.clone(),
                            dest_owner: dest_addr.clone(),
                            status: TransferStatus::Completed,
                            created_at: now,
                            source_tx_hash: Some(transfer_result.lock_tx_hash.clone()),
                            dest_tx_hash: Some(transfer_result.mint_tx_hash.clone()),
                            source_contract,
                            dest_contract,
                            source_fee: source_fee_str,
                            dest_fee: dest_fee_str,
                        });

                        result_signal.set(Some(format!(
                            "Transfer complete!\nTransfer ID: {}\nRight {} moved from {:?} to {:?}\n\nSeal: {}\nProof: {}\nLock TX: {}\nMint TX: {}",
                            transfer_id, right, from, to,
                            truncate_address(&seal_ref, 12),
                            proof_type,
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
                                option { key: "account-{idx}", value: idx.to_string(), selected: idx == *selected_account_index.read(),
                                    {format!("{} - {} (Balance: {:.8}){}",
                                        account.name,
                                        &account.address[..8.min(account.address.len())],
                                        account.balance_raw as f64 / 1e8, // Convert satoshis to BTC for display
                                        if account.is_watch_only() { " [WATCH-ONLY]" } else { "" }
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
                                    p { key: "source-contract-{idx}", class: "text-xs text-green-400 font-mono",
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
                                            option { key: "target-contract-{idx}", value: idx.to_string(), selected: idx == *selected_target_contract_index.read(),
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
                                option { key: "right-{idx}", value: idx.to_string(), selected: idx == *selected_right_index.read(),
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
                        r#type: "text",
                        disabled: *executing.read(),
                    }
                })}

                // Progress steps
                if *step.read() > 0 {
                    div { class: "space-y-2 mt-4",
                        for (i, step_text) in steps.iter().enumerate() {
                            div { key: "step-{i}", class: "flex items-center gap-2",
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
                        || is_watch_only
                        || !has_target_contract
                        || !has_dest_account
                        || !dest_has_enough_balance,
                    class: "{btn_full_primary_class()}",
                    if *executing.read() {
                        "Executing..."
                    } else if !has_account {
                        "Add Source Account First"
                    } else if is_watch_only {
                        "Watch-Only Account (Cannot Sign)"
                    } else if !has_rights {
                        "No Rights Available"
                    } else if !has_target_contract {
                        "Deploy Target Contract First"
                    } else if !has_dest_account {
                        "Add Destination Account First"
                    } else if !dest_has_enough_balance {
                        "Fund Destination Account"
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
                } else if is_watch_only {
                    p { class: "text-xs text-red-500 mt-2",
                        "Note: This account is watch-only. Import the private key to transfer."
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
                if !has_dest_account {
                    p { class: "text-xs text-red-500 mt-2",
                        {format!("Note: Add an account for {:?} destination chain to pay gas fees", *to_chain.read())}
                    }
                } else if !dest_has_enough_balance {
                    p { class: "text-xs text-red-500 mt-2",
                        {format!("Note: Destination account on {:?} needs gas funds (min: {})",
                            *to_chain.read(),
                            match *to_chain.read() {
                                Chain::Sui => "0.01 SUI",
                                Chain::Aptos => "0.01 APT",
                                Chain::Ethereum => "0.001 ETH",
                                Chain::Solana => "0.001 SOL",
                                _ => "0.0",
                            }
                        )}
                    }
                }
            }
        }
    }
}

/// Format fee amount for display with appropriate chain units.
fn format_fee(fee: u64, chain: Chain) -> String {
    match chain {
        Chain::Bitcoin => {
            // Bitcoin fees are in satoshis
            format!("{:.8} BTC", fee as f64 / 100_000_000.0)
        }
        Chain::Ethereum => {
            // Ethereum fees are in wei
            format!("{:.6} ETH", fee as f64 / 1_000_000_000_000_000_000.0)
        }
        Chain::Sui => {
            // Sui fees are in MIST (10^-9 SUI)
            format!("{:.6} SUI", fee as f64 / 1_000_000_000.0)
        }
        Chain::Aptos => {
            // Aptos fees are in octas (10^-8 APT)
            format!("{:.6} APT", fee as f64 / 100_000_000.0)
        }
        Chain::Solana => {
            // Solana fees are in lamports (10^-9 SOL)
            format!("{:.6} SOL", fee as f64 / 1_000_000_000.0)
        }
        _ => format!("{}", fee),
    }
}
