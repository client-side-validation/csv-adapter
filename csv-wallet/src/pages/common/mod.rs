//! Common UI helpers and styling functions for pages.

use crate::context::types::{RightStatus, SealRecord, TestStatus, TransferStatus};
use crate::routes::Route;
use csv_adapter_core::Chain;
use dioxus::prelude::*;

// ===== Chain Styling Helpers =====
pub fn chain_color(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "#F7931A",
        Chain::Ethereum => "#627EEA",
        Chain::Sui => "#06BDFF",
        Chain::Aptos => "#2DD8A3",
        Chain::Solana => "#9945FF",
        _ => "#888888",
    }
}

pub fn chain_badge_class(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-orange-400 bg-orange-500/20 border border-orange-500/30",
        Chain::Ethereum => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-blue-400 bg-blue-500/20 border border-blue-500/30",
        Chain::Sui => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-cyan-400 bg-cyan-500/20 border border-cyan-500/30",
        Chain::Aptos => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-emerald-400 bg-emerald-500/20 border border-emerald-500/30",
        Chain::Solana => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-purple-400 bg-purple-500/20 border border-purple-500/30",
        _ => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-gray-400 bg-gray-500/20 border border-gray-500/30",
    }
}

pub fn chain_icon_emoji(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "\u{1F7E0}",
        Chain::Ethereum => "\u{1F537}",
        Chain::Sui => "\u{1F30A}",
        Chain::Aptos => "\u{1F7E2}",
        Chain::Solana => "\u{25C8}",
        _ => "\u{26AA}",
    }
}

pub fn chain_name(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "Bitcoin",
        Chain::Ethereum => "Ethereum",
        Chain::Sui => "Sui",
        Chain::Aptos => "Aptos",
        Chain::Solana => "Solana",
        _ => "Unknown",
    }
}

pub fn format_timestamp(timestamp: u64) -> String {
    let now = js_sys::Date::now() as u64 / 1000;
    let diff = now.saturating_sub(timestamp);
    
    if diff < 60 {
        "Just now".to_string()
    } else if diff < 3600 {
        format!("{} minutes ago", diff / 60)
    } else if diff < 86400 {
        format!("{} hours ago", diff / 3600)
    } else {
        format!("{} days ago", diff / 86400)
    }
}

pub fn right_status_class(status: &RightStatus) -> &'static str {
    match status {
        RightStatus::Active => "text-green-400 bg-green-500/20",
        RightStatus::Transferred => "text-blue-400 bg-blue-500/20",
        RightStatus::Consumed => "text-gray-400 bg-gray-500/20",
    }
}

pub fn transfer_status_class(status: &TransferStatus) -> &'static str {
    match status {
        TransferStatus::Completed => "text-green-400 bg-green-500/20",
        TransferStatus::Failed => "text-red-400 bg-red-500/20",
        _ => "text-yellow-400 bg-yellow-500/20",
    }
}

pub fn test_status_class(status: &TestStatus) -> &'static str {
    match status {
        TestStatus::Passed => "text-green-400 bg-green-500/20",
        TestStatus::Failed => "text-red-400 bg-red-500/20",
        TestStatus::Running => "text-blue-400 bg-blue-500/20",
        TestStatus::Pending => "text-gray-400 bg-gray-500/20",
    }
}

pub fn seal_consumed_class(consumed: bool) -> &'static str {
    if consumed {
        "bg-red-900/30 border-red-700/50"
    } else {
        "bg-green-900/30 border-green-700/50"
    }
}

pub fn seal_consumed_text_class(consumed: bool) -> &'static str {
    if consumed {
        "text-red-300"
    } else {
        "text-green-300"
    }
}

// ===== Shared UI Patterns =====
pub fn card_class() -> &'static str {
    "bg-gray-900 rounded-xl border border-gray-800"
}

pub fn card_header_class() -> &'static str {
    "px-4 py-3 border-b border-gray-800"
}

pub fn input_class() -> &'static str {
    "w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
}

pub fn input_mono_class() -> &'static str {
    "w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono"
}

pub fn btn_primary_class() -> &'static str {
    "px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-sm font-medium transition-colors"
}

pub fn btn_secondary_class() -> &'static str {
    "px-4 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 text-sm font-medium transition-colors"
}

pub fn btn_full_primary_class() -> &'static str {
    "w-full px-4 py-2.5 rounded-lg bg-blue-600 hover:bg-blue-700 text-sm font-medium transition-colors"
}

pub fn table_class() -> &'static str {
    "bg-gray-900 rounded-xl border border-gray-800 overflow-hidden"
}

pub fn label_class() -> &'static str {
    "block text-sm text-gray-400 mb-1"
}

pub fn select_class() -> &'static str {
    "w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
}

pub fn chain_options() -> Vec<(Chain, &'static str)> {
    vec![
        (Chain::Bitcoin, "\u{1F7E0} Bitcoin"),
        (Chain::Ethereum, "\u{1F537} Ethereum"),
        (Chain::Sui, "\u{1F30A} Sui"),
        (Chain::Aptos, "\u{1F7E2} Aptos"),
        (Chain::Solana, "\u{25C8} Solana"),
    ]
}

pub fn network_options() -> Vec<(crate::context::Network, &'static str)> {
    use crate::context::Network;
    vec![
        (Network::Dev, "Dev"),
        (Network::Test, "Test"),
        (Network::Main, "Main"),
    ]
}

pub fn chain_select(mut onchange: impl FnMut(std::rc::Rc<FormData>) + 'static, value: Chain) -> Element {
    rsx! {
        select {
            class: "{select_class()}",
            value: "{value}",
            onchange: move |evt| onchange(evt.data()),
            for (c, label) in chain_options() {
                option { key: "{c}", value: "{c}", selected: c == value, "{label}" }
            }
        }
    }
}

pub fn network_select(mut onchange: impl FnMut(crate::context::Network) + 'static, value: crate::context::Network) -> Element {
    use crate::context::Network;
    rsx! {
        select {
            class: "{select_class()}",
            value: "{value:?}",
            onchange: move |evt| {
                let val = evt.value();
                let network = match val.as_str() {
                    "Dev" => Network::Dev,
                    "Test" => Network::Test,
                    "Main" => Network::Main,
                    _ => Network::Dev,
                };
                onchange(network);
            },
            for (n, label) in network_options() {
                option { key: "{n:?}", value: "{n:?}", selected: n == value, "{label}" }
            }
        }
    }
}

pub fn empty_state(icon: &'static str, title: &'static str, subtitle: &'static str) -> Element {
    rsx! {
        div { class: "text-center py-12 space-y-3",
            div { class: "text-5xl", "{icon}" }
            p { class: "text-gray-400 text-lg", "{title}" }
            p { class: "text-sm text-gray-500", "{subtitle}" }
        }
    }
}

pub fn form_field(label: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "space-y-2",
            label { class: "{label_class()}", "{label}" }
            {children}
        }
    }
}

pub fn truncate_address(addr: &str, chars: usize) -> String {
    if addr.len() <= chars * 2 + 2 {
        addr.to_string()
    } else {
        format!("{}...{}", &addr[..chars], &addr[addr.len() - chars..])
    }
}
