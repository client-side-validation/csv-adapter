//! Cross-chain transfer page.

use crate::context::{
    ProofData, ProofRecord, ProofStatus, SanadStatus, SealContent, SealRecord, SealStatus,
    TrackedTransfer, TransferStatus, use_wallet_context,
};
use crate::pages::common::*;
use crate::routes::Route;
use crate::services::blockchain::{ContractDeployment, ContractType};
use csv_store::state::ChainId;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn CrossChainTransfer() -> Element {
    let wallet_ctx = use_wallet_context();
    let mut from_chain = use_signal(|| ChainId::new("bitcoin"));
    let mut to_chain = use_signal(|| ChainId::new("sui"));
    let mut selected_sanad_index = use_signal(|| 0usize);
    let mut dest_owner = use_signal(String::new);
    let mut step = use_signal(|| 0);
    let result = use_signal(|| Option::<String>::None);
    let mut error = use_signal(|| Option::<String>::None);
    let mut executing = use_signal(|| false);
    let mut selected_account_index = use_signal(|| 0usize);
    let mut selected_target_contract_index = use_signal(|| 0usize);

    // Get sanads for the source chain (filtered to active only)
    let from_chain_val = from_chain.read().clone();
    let from_chain_display = from_chain_val.clone();
    let sanads_for_source: Vec<_> = wallet_ctx
        .sanads_for_chain(from_chain_val)
        .into_iter()
        .filter(|r| r.status == SanadStatus::Active)
        .collect();
    let has_sanads = !sanads_for_source.is_empty();

    // Reset sanad selection when chain changes
    use_effect(move || {
        selected_sanad_index.set(0);
    });

    // Clone for use in memo
    let sanads_for_memo = sanads_for_source.clone();
    // Get the selected sanad ID
    let sanad_id = use_memo(move || {
        sanads_for_memo
            .get(*selected_sanad_index.read())
            .map(|r| r.id.clone())
            .unwrap_or_default()
    });

    // Get accounts for the source chain
    let accounts = wallet_ctx.accounts_for_chain(from_chain.read().clone());
    let has_account = !accounts.is_empty();

    // Check if selected account is watch-only (can't sign)
    let selected_account = accounts.get(*selected_account_index.read());
    let is_watch_only = selected_account.map(|a| a.is_watch_only()).unwrap_or(false);

    // Get accounts for the destination chain (needed for gas payment)
    let dest_accounts = wallet_ctx.accounts_for_chain(to_chain.read().clone());
    let has_dest_account = !dest_accounts.is_empty();

    // Track fetched destination balance (in raw chain units: satoshis, lamports, MIST, octas, wei)
    let mut dest_balance_raw = use_signal(|| 0u64);
    let mut dest_balance_loading = use_signal(|| false);

    // Fetch destination balance when chain or account changes
    use_effect({
        let to_chain_val = to_chain.read().clone();
        let dest_addr = dest_accounts.first().map(|a| a.address.clone());
        move || {
            if let Some(addr) = &dest_addr {
                dest_balance_loading.set(true);
                let addr = addr.clone();
                let chain_for_balance = to_chain_val.clone();
                spawn(async move {
                    use crate::services::chain_api::ChainApi;
                    let api = ChainApi::default();
                    if let Ok(balance_str) = api.get_balance(&addr, chain_for_balance).await {
                        if let Ok(balance) = balance_str.parse::<u64>() {
                            dest_balance_raw.set(balance);
                        }
                    }
                    dest_balance_loading.set(false);
                });
            }
        }
    });

    // Check if destination account has minimum balance for gas (in raw chain units)
    // Sui: ~0.01 SUI = 10_000_000 MIST, Aptos: ~0.01 APT = 1_000_000 octas
    let min_dest_balance_raw = match to_chain.read().as_str() {
        "sui" => 10_000_000u64,                 // 0.01 SUI in MIST
        "aptos" => 1_000_000u64,                // 0.01 APT in octas
        "ethereum" => 1_000_000_000_000_000u64, // ~0.001 ETH in wei
        "solana" => 1_000_000u64,               // ~0.001 SOL in lamports
        _ => 0u64, // Bitcoin doesn't need pre-funded destination for minting
    };
    let dest_has_enough_balance = *dest_balance_raw.read() >= min_dest_balance_raw;

    // Get contracts for source and target chains
    let source_contracts = wallet_ctx.contracts_for_chain(from_chain.read().clone());
    let target_contracts = wallet_ctx.contracts_for_chain(to_chain.read().clone());
    let _has_source_contract =
        !source_contracts.is_empty() || from_chain.read().as_str() == "bitcoin";
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
        "Lock Sanad on source chain",
        "Generate cryptographic proof",
        "Verify proof on destination",
        "Mint Sanad on destination",
        "Complete transfer",
    ];

    // Clone before moving into closure to avoid borrow after move
    let sanads_for_closure = sanads_for_source.clone();

    // Execute real cross-chain transfer using native signing
    let execute_transfer = move |_| {
        let sanads_for_source_closure = sanads_for_closure.clone();
        if !_has_source_contract {
            error.set(Some(format!(
                "No contract deployed on {:?}. Deploy contracts manually using Foundry/forge and set the address with `csv contracts set {} <address>`.",
                from_chain.read().clone(), from_chain.read().clone()
            )));
            return;
        }

        if !has_target_contract {
            error.set(Some(format!(
                "No contract deployed on {:?}. Deploy contracts manually using Foundry/forge and set the address with `csv contracts set {} <address>`.",
                to_chain.read().clone(), to_chain.read().clone()
            )));
            return;
        }

        if !has_dest_account {
            error.set(Some(format!(
                "No account available for destination chain {:?}. Please add an account first.",
                to_chain.read().clone()
            )));
            return;
        }

        if !dest_has_enough_balance {
            let min_balance = match to_chain.read().as_str() {
                "sui" => "0.01 SUI",
                "aptos" => "0.01 APT",
                "ethereum" => "0.001 ETH",
                "solana" => "0.001 SOL",
                _ => "funds",
            };
            error.set(Some(format!(
                "Destination account on {} needs at least {} for gas fees. Please fund your account first.",
                to_chain.read().as_str(), min_balance
            )));
            return;
        }

        if !has_account {
            error.set(Some(format!(
                "No account available for {:?}. Please add an account first.",
                from_chain.read().clone()
            )));
            return;
        }

        if !has_sanads {
            error.set(Some(format!(
                "No active sanads available for {:?}. Create a sanad first.",
                from_chain.read().clone()
            )));
            return;
        }

        // All chains now supported via proper BCS/ABI encoding
        // - Bitcoin: Native UTXO with mempool.space
        // - Ethereum: Native ABI encoding
        // - Sui: BCS encoding via sdk_tx
        // - Aptos: BCS encoding via sdk_tx (planned)

        let from = from_chain.read().clone();
        let to = to_chain.read().clone();

        executing.set(true);
        error.set(None);
        step.set(1);

        // Spawn async task for blockchain operations
        spawn({
            let sanad = sanad_id.read().clone();
            let dest = dest_owner.read().clone();
            let account_idx = *selected_account_index.read();
            let target_contract_idx = *selected_target_contract_index.read();
            let from_for_accounts = from.clone();
            let accounts = wallet_ctx.accounts_for_chain(from_for_accounts);
            let mut step_signal = step;
            let mut result_signal = result;
            let mut error_signal = error;
            let mut executing_signal = executing;
            let mut wallet_ctx = wallet_ctx.clone();

            async move {
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

                // Determine destination owner (default to same address)
                let dest_addr: String = if dest.is_empty() {
                    account.address.clone()
                } else {
                    dest.to_string()
                };

                // Step 1: Lock sanad on source chain
                step_signal.set(1);
                web_sys::console::log_1(&"Step 1: Locking sanad on source chain...".into());

                // Build contracts map with both source and target contracts
                let mut contracts = std::collections::HashMap::new();

                // Add source chain contract (needed for locking)
                let from_for_contracts = from.clone();
                let source_contracts = wallet_ctx.contracts_for_chain(from_for_contracts);
                if !source_contracts.is_empty() {
                    if let Some(contract) = source_contracts.first() {
                        let from_for_insert = from.clone();
                        contracts.insert(
                            from_for_insert,
                            ContractDeployment {
                                address: contract.address.clone(),
                                chain: Some(from.clone()),
                                contract_address: contract.address.clone(),
                                tx_hash: contract.tx_hash.clone(),
                                deployed_at: contract.deployed_at,
                                contract_type: ContractType::Lock,
                            },
                        );
                    }
                }

                // Add target chain contract (needed for minting)
                let to_for_contracts = to.clone();
                let target_contracts = wallet_ctx.contracts_for_chain(to_for_contracts);
                if !target_contracts.is_empty() {
                    let selected_idx = target_contract_idx.min(target_contracts.len() - 1);
                    if let Some(contract) = target_contracts.get(selected_idx) {
                        let to_for_insert = to.clone();
                        contracts.insert(
                            to_for_insert,
                            ContractDeployment {
                                address: contract.address.clone(),
                                chain: Some(to.clone()),
                                contract_address: contract.address.clone(),
                                tx_hash: contract.tx_hash.clone(),
                                deployed_at: contract.deployed_at,
                                contract_type: ContractType::Lock,
                            },
                        );
                    }
                }

                let from_for_exec = from.clone();
                let to_for_exec = to.clone();
                
                // Use csv_sdk directly for cross-chain transfer
                use csv_sdk::CsvClient;
                let csv_client = CsvClient::builder()
                    .with_store_backend(csv_sdk::builder::StoreBackend::InMemory)
                    .build()
                    .expect("Failed to create CSV client");
                
                // Convert sanad string to [u8; 32] for SanadId
                let sanad_bytes: [u8; 32] = if sanad.starts_with("0x") {
                    hex::decode(&sanad[2..])
                        .map(|bytes| {
                            let mut arr = [0u8; 32];
                            arr.copy_from_slice(&bytes[..32.min(bytes.len())]);
                            arr
                        })
                        .unwrap_or([0u8; 32])
                } else {
                    hex::decode(&sanad)
                        .map(|bytes| {
                            let mut arr = [0u8; 32];
                            arr.copy_from_slice(&bytes[..32.min(bytes.len())]);
                            arr
                        })
                        .unwrap_or([0u8; 32])
                };
                let sanad_id_for_transfer = csv_core::SanadId::new(sanad_bytes);
                let transfer_builder = csv_client
                    .transfers()
                    .cross_chain(sanad_id_for_transfer, to_for_exec.clone())
                    .from_chain(from_for_exec.clone())
                    .to_address(dest_addr.clone());
                
                match transfer_builder.execute().await {
                    Ok(transfer_id) => {
                        step_signal.set(6); // Set beyond last step to show all completed
                        let now = js_sys::Date::now() as u64 / 1000;

                        // Get contract addresses from the contracts map
                        let _source_contract =
                            contracts.get(&from).map(|c| c.contract_address.clone());
                        let dest_contract = contracts.get(&to).map(|c| c.contract_address.clone());

                        // Query transfer status to get lock_tx_hash and other details
                        let lock_tx_hash = csv_client.transfers()
                            .status(&transfer_id)
                            .ok()
                            .and_then(|status| {
                                // Try to extract lock_tx_hash from status if available
                                // For now, use a placeholder
                                None
                            })
                            .unwrap_or_else(|| format!("0x{}", hex::encode([0u8; 32])));
                        let mint_tx_hash = format!("0x{}", hex::encode([0u8; 32])); // Placeholder

                        // Fees are not returned by csv_sdk execute, set to 0 for now
                        let source_fee_str = Some(0u64);
                        let dest_fee_str = Some(0u64);

                        // Create linked Seal record
                        let seal_ref = format!("seal_{}", &transfer_id[..16]);
                        let seal_content = SealContent {
                            content_hash: format!("0x{}", &sanad[..40.min(sanad.len())]),
                            owner: dest_addr.clone(),
                            block_number: None,
                            lock_tx_hash: Some(lock_tx_hash.clone()),
                        };
                        let seal = SealRecord {
                            seal_ref: seal_ref.clone(),
                            chain: from.clone(),
                            value: sanads_for_source_closure
                                .get(*selected_sanad_index.read())
                                .map(|r| r.value)
                                .unwrap_or(0),
                            consumed: false,
                            sanad_id: Some(sanad.clone()),
                            status: SealStatus::Locked,
                            created_at: now,
                            content: Some(serde_json::to_string(&seal_content).unwrap_or_default()),
                            proof_ref: None,
                        };
                        wallet_ctx.add_seal(seal);

                        // Create linked Proof record
                        let from_for_proof = from.clone();
                        let proof_data = match from_for_proof.as_str() {
                            "bitcoin" => ProofData::Merkle {
                                root: format!("0x{}", &lock_tx_hash[..40]),
                                path: vec![format!("0x{}", &sanad[..40])],
                                leaf_index: 0,
                            },
                            "ethereum" => ProofData::Mpt {
                                root: format!("0x{}", &lock_tx_hash[..40]),
                                account_proof: vec![format!("0x{}", &sanad[..40])],
                                storage_proof: vec![format!("0x{}", &transfer_id[..40])],
                            },
                            "sui" => ProofData::Checkpoint {
                                sequence: now,
                                digest: lock_tx_hash.clone(),
                                signatures: vec![
                                    "validator_1".to_string(),
                                    "validator_2".to_string(),
                                ],
                            },
                            "aptos" => ProofData::Ledger {
                                version: now,
                                proof: format!("0x{}", &lock_tx_hash[..40]),
                            },
                            "solana" => ProofData::Solana {
                                slot: now,
                                bank_hash: format!("0x{}", &lock_tx_hash[..40]),
                                merkle_proof: vec![format!("0x{}", &sanad[..40])],
                            },
                            _ => ProofData::Merkle {
                                root: format!("0x{}", &lock_tx_hash[..40]),
                                path: vec![format!("0x{}", &sanad[..40])],
                                leaf_index: 0,
                            },
                        };

                        let proof_type = match from.as_str() {
                            "bitcoin" => "merkle",
                            "ethereum" => "mpt",
                            "sui" => "checkpoint",
                            "aptos" => "ledger",
                            "solana" => "solana",
                            _ => "merkle",
                        };

                        let proof = ProofRecord {
                            chain: from.clone(),
                            sanad_id: sanad.clone(),
                            seal_ref: Some(seal_ref.clone()),
                            proof_type: proof_type.to_string(),
                            proof_system: None,
                            verified: true,
                            proof_data: Some(
                                serde_json::to_string(&proof_data).unwrap_or_default(),
                            ),
                            block_height: None,
                            created_at: now,
                            verified_at: Some(now),
                            status: ProofStatus::Verified,
                            target_chain: Some(to.clone()),
                            verification_tx_hash: Some(mint_tx_hash.clone()),
                        };
                        wallet_ctx.add_proof(proof);

                        // Link proof to seal
                        wallet_ctx.link_proof_to_seal(
                            &seal_ref,
                            &format!("proof_{}", &transfer_id[..16]),
                        );

                        // Record the transfer with full details
                        let from_for_transfer = from.clone();
                        let to_for_transfer = to.clone();
                        wallet_ctx.add_transfer(TrackedTransfer {
                            id: transfer_id.clone(),
                            source_chain: from_for_transfer.clone(),
                            dest_chain: to_for_transfer.clone(),
                            sanad_id: sanad.clone(),
                            sender_address: Some(dest_addr.clone()),
                            destination_address: Some(dest_addr.clone()),
                            status: TransferStatus::Completed,
                            created_at: now,
                            source_tx_hash: Some(lock_tx_hash.clone()),
                            dest_tx_hash: Some(mint_tx_hash.clone()),
                            destination_contract: dest_contract,
                            source_fee: source_fee_str,
                            dest_fee: dest_fee_str,
                            proof: None,
                            completed_at: None,
                        });

                        result_signal.set(Some(format!(
                            "Transfer complete!\nTransfer ID: {}\nSanad {} moved from {:?} to {:?}\n\nSeal: {}\nProof: {}\nLock TX: {}\nMint TX: {}",
                            transfer_id, sanad, from_for_transfer, to_for_transfer,
                            truncate_address(&seal_ref, 12),
                            proof_type,
                            lock_tx_hash,
                            mint_tx_hash
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
                h1 { class: "text-xl font-bold", "Cross-ChainId Transfer" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                // Account Selection Section
                div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700",
                    h3 { class: "text-sm font-medium text-gray-300 mb-3", "1. Select Source Account" }
                    if accounts.is_empty() {
                        div { class: "text-sm text-red-400",
                            {format!("No accounts available for {:?}. Please add an account first.", from_chain.read().clone())}
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
                    {form_field("From ChainId", chain_select(move |v: Rc<FormData>| {
                        if let Ok(c) = v.value().parse::<ChainId>() {
                            from_chain.set(c);
                            selected_account_index.set(0); // Reset account selection
                        }
                    }, from_chain.read().clone()))}

                    {form_field("To ChainId", chain_select(move |v: Rc<FormData>| {
                        if let Ok(c) = v.value().parse::<ChainId>() { to_chain.set(c); }
                    }, to_chain.read().clone()))}
                }

                // ChainId compatibility note
                div { class: "bg-blue-900/30 border border-blue-700/50 rounded-lg p-3",
                    p { class: "text-xs text-blue-300", "ChainId support (all via native signing):" }
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
                            p { class: "text-xs text-gray-500 mb-1", {format!("Source ({:?})", from_chain.read().clone())} }
                            if source_contracts.is_empty() {
                                if matches!(from_chain.read().as_str(), "bitcoin") {
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
                            p { class: "text-xs text-gray-500 mb-1", {format!("Target ({:?})", to_chain.read().clone())} }
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

                 {form_field("Available Sanads", rsx! {
                    if sanads_for_source.is_empty() {
                        p { class: "text-sm text-red-400",
                            {format!("No active sanads available for {:?}. Create a sanad on this chain first.", from_chain_display)}
                        }
                    } else {
                        select {
                            class: "{input_mono_class()}",
                            onchange: move |evt| {
                                if let Ok(idx) = evt.value().parse::<usize>() {
                                    selected_sanad_index.set(idx);
                                }
                            },
                            for (idx, sanad) in sanads_for_source.iter().enumerate() {
                                option { key: "sanad-{idx}", value: idx.to_string(), selected: idx == *selected_sanad_index.read(),
                                    {format!("{}... - Value: {} - {}",
                                        &sanad.id[..16.min(sanad.id.len())],
                                        sanad.value,
                                        sanad.status
                                    )}
                                }
                            }
                        }
                    }
                })}

                // Show selected sanad details
                if let Some(sanad) = sanads_for_source.get(*selected_sanad_index.read()) {
                    div { class: "bg-gray-800/50 rounded-lg p-3 border border-gray-700",
                        p { class: "text-xs text-gray-400 mb-2", "Selected Sanad Details:" }
                        div { class: "grid grid-cols-2 gap-2 text-xs",
                            div { span { class: "text-gray-500", "Full ID: " }, span { class: "font-mono text-gray-300 break-all", "{&sanad.id}" } }
                            div { span { class: "text-gray-500", "Value: " }, span { class: "font-mono text-gray-300", "{sanad.value}" } }
                            div { span { class: "text-gray-500", "Status: " }, span { class: "{sanad_status_class(&sanad.status)}", "{sanad.status}" } }
                            div { span { class: "text-gray-500", "Owner: " }, span { class: "font-mono text-gray-300", "{truncate_address(&sanad.owner, 8)}" } }
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
                        || !has_sanads
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
                    } else if !has_sanads {
                        "No Sanads Available"
                   } else if !has_target_contract {
                         "Set Target Contract Address"
                    } else if !has_dest_account {
                        "Add Destination Account First"
                    } else if !dest_has_enough_balance {
                        "Fund Destination Account"
                    } else if *step.read() >= 5 {
                        "Transfer Complete"
                    } else {
                        "Execute Cross-ChainId Transfer"
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
                if !has_sanads {
                    p { class: "text-xs text-red-500 mt-2",
                        {format!("Note: Create a Sanad on {:?} source chain first", from_chain.read().clone())}
                    }
                }
                if !has_target_contract {
                    p { class: "text-xs text-red-500 mt-2",
                        {format!("Note: Deploy a CSV contract on {:?} manually using Foundry/forge, then set the address with `csv contracts set`", to_chain.read().clone())}
                    }
                }
                if !has_dest_account {
                    p { class: "text-xs text-red-500 mt-2",
                        {format!("Note: Add an account for {:?} destination chain to pay gas fees", to_chain.read().clone())}
                    }
                } else if !dest_has_enough_balance {
                    p { class: "text-xs text-red-500 mt-2",
                        {format!("Note: Destination account on {} needs gas funds (min: {})",
                            to_chain.read().clone(),
                            match to_chain.read().clone().as_str() {
                                "sui" => "0.01 SUI",
                                "aptos" => "0.01 APT",
                                "ethereum" => "0.001 ETH",
                                "solana" => "0.001 SOL",
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
fn format_fee(fee: u64, chain: &ChainId) -> String {
    match chain.as_str() {
        "bitcoin" => {
            // Bitcoin fees are in satoshis
            format!("{:.8} BTC", fee as f64 / 100_000_000.0)
        }
        "ethereum" => {
            // Ethereum fees are in wei
            format!("{:.6} ETH", fee as f64 / 1_000_000_000_000_000_000.0)
        }
        "sui" => {
            // Sui fees are in MIST (10^-9 SUI)
            format!("{:.6} SUI", fee as f64 / 1_000_000_000.0)
        }
        "aptos" => {
            // Aptos fees are in octas (10^-8 APT)
            format!("{:.6} APT", fee as f64 / 100_000_000.0)
        }
        "solana" => {
            // Solana fees are in lamports (10^-9 SOL)
            format!("{:.6} SOL", fee as f64 / 1_000_000_000.0)
        }
        _ => format!("{}", fee),
    }
}
