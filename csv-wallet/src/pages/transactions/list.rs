//! Transactions list page.

use crate::context::{use_wallet_context, TransactionRecord};
use crate::pages::common::*;
use crate::pages::transactions::TransactionCard;
use crate::routes::Route;
use dioxus::prelude::*;

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
