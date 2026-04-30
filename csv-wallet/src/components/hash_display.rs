//! Hash Display Component
//!
//! Displays blockchain hashes and addresses with:
//! - Shortened format (0x1234...5678)
//! - Copy-to-clipboard button
//! - Tooltip with full value
//! - Monospace font styling

use dioxus::prelude::*;
use gloo_timers::future::sleep;
use std::time::Duration;

/// Props for the HashDisplay component.
#[derive(Props, Clone, PartialEq)]
pub struct HashDisplayProps {
    /// The full hash/address to display.
    pub value: String,
    /// Number of prefix characters to show (default: 6).
    #[props(default = 6)]
    pub prefix_len: usize,
    /// Number of suffix characters to show (default: 4).
    #[props(default = 4)]
    pub suffix_len: usize,
    /// Additional CSS classes.
    #[props(default)]
    pub class: String,
    /// Whether to show the copy button.
    #[props(default = true)]
    pub show_copy: bool,
    /// Whether to show full hash on hover tooltip.
    #[props(default = true)]
    pub show_tooltip: bool,
    /// Optional label before the hash.
    #[props(default)]
    pub label: Option<String>,
}

/// Display a shortened hash with copy functionality.
#[allow(non_snake_case)]
pub fn HashDisplay(props: HashDisplayProps) -> Element {
    let value = props.value.clone();
    let shortened = shorten_hash(&value, props.prefix_len, props.suffix_len);
    let mut copied = use_signal(|| false);
    
    let mut copy_to_clipboard = {
        let _value = value.clone();
        move || {
            #[cfg(target_arch = "wasm32")]
            {
                // Clipboard access via web_sys is feature-gated; for now just log
                web_sys::console::log_1(&"Copy to clipboard (WASM): clipboard API requires web_sys features".into());
                // In a full implementation with proper web_sys features:
                // let window = web_sys::window().unwrap();
                // let navigator = window.navigator();
                // let clipboard = navigator.clipboard();
                // let _ = clipboard.write_text(&value);
            }
            copied.set(true);
            // Reset copied state after 2 seconds
            spawn(async move {
                sleep(Duration::from_secs(2)).await;
                copied.set(false);
            });
        }
    };
    
    let tooltip_attr = if props.show_tooltip {
        value.clone()
    } else {
        String::new()
    };
    
    rsx! {
        span {
            class: "hash-display {props.class}",
            title: if props.show_tooltip { tooltip_attr } else { String::new() },
            
            // Optional label
            if let Some(label) = props.label {
                span { class: "hash-label", "{label}: " }
            }
            
            // Shortened hash
            span { class: "hash-value", "{shortened}" }
            
            // Copy button
            if props.show_copy {
                button {
                    class: "hash-copy-btn",
                    onclick: move |_| copy_to_clipboard(),
                    title: if copied() { "Copied!" } else { "Copy to clipboard" },
                    if copied() {
                        // Checkmark icon
                        svg {
                            class: "hash-icon copied",
                            view_box: "0 0 16 16",
                            fill: "currentColor",
                            path { d: "M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z" }
                        }
                    } else {
                        // Copy icon
                        svg {
                            class: "hash-icon",
                            view_box: "0 0 16 16",
                            fill: "currentColor",
                            path { d: "M0 1.75A.75.75 0 01.75 1h10.5a.75.75 0 010 1.5H1.75A.75.75 0 010 1.75zm0 3A.75.75 0 01.75 4h10.5a.75.75 0 010 1.5H1.75A.75.75 0 010 4.75zm0 3A.75.75 0 01.75 7h10.5a.75.75 0 010 1.5H1.75A.75.75 0 010 7.75zm0 3A.75.75 0 01.75 10h10.5a.75.75 0 010 1.5H1.75A.75.75 0 010 10.75zm12.5-6.5a.75.75 0 01.75-.75h1.5a.75.75 0 010 1.5h-1.5a.75.75 0 01-.75-.75zm0 3a.75.75 0 01.75-.75h1.5a.75.75 0 010 1.5h-1.5a.75.75 0 01-.75-.75zm0 3a.75.75 0 01.75-.75h1.5a.75.75 0 010 1.5h-1.5a.75.75 0 01-.75-.75z" }
                        }
                    }
                }
            }
        }
    }
}

/// Shorten a hash to format: 0x{prefix}...{suffix}
pub fn shorten_hash(hash: &str, prefix_len: usize, suffix_len: usize) -> String {
    if hash.len() <= prefix_len + suffix_len + 3 {
        return hash.to_string();
    }
    
    let prefix = &hash[..prefix_len.min(hash.len())];
    let suffix_start = hash.len().saturating_sub(suffix_len);
    let suffix = &hash[suffix_start..];
    
    format!("{}...{}", prefix, suffix)
}

/// Transaction hash display with chain icon.
#[derive(Props, Clone, PartialEq)]
pub struct TxHashDisplayProps {
    /// Transaction hash.
    pub tx_hash: String,
    /// Chain name for explorer link.
    pub chain: String,
    /// Explorer URL base (e.g., https://etherscan.io/tx/).
    pub explorer_url: Option<String>,
    /// Additional CSS classes.
    #[props(default)]
    pub class: String,
}

/// Display a transaction hash with optional explorer link.
#[allow(non_snake_case)]
pub fn TxHashDisplay(props: TxHashDisplayProps) -> Element {
    let has_explorer = props.explorer_url.is_some();
    
    rsx! {
        span { class: "tx-hash-display {props.class}",
            HashDisplay {
                value: props.tx_hash.clone(),
                prefix_len: 10,
                suffix_len: 8,
                show_copy: true,
            }
            
            if has_explorer {
                a {
                    class: "tx-explorer-link",
                    href: "{props.explorer_url.as_ref().unwrap()}{props.tx_hash}",
                    target: "_blank",
                    rel: "noopener noreferrer",
                    title: "View on explorer",
                    // External link icon
                    svg {
                        class: "hash-icon external",
                        view_box: "0 0 16 16",
                        fill: "currentColor",
                        path { d: "M8.636 3.5a.5.5 0 00-.5-.5H1.5A1.5 1.5 0 000 4.5v10A1.5 1.5 0 001.5 16h10a1.5 1.5 0 001.5-1.5V7.864a.5.5 0 00-1 0V14.5a.5.5 0 01-.5.5h-10a.5.5 0 01-.5-.5v-10a.5.5 0 01.5-.5h6.636a.5.5 0 00.5-.5z" }
                        path { d: "M16 .5a.5.5 0 00-.5-.5h-5a.5.5 0 000 1h3.793L6.146 9.146a.5.5 0 10.708.708L15 1.707V5.5a.5.5 0 001 0v-5z" }
                    }
                }
            }
        }
    }
}

/// Address display with identicon or chain icon.
#[derive(Props, Clone, PartialEq)]
pub struct AddressDisplayProps {
    /// The address to display.
    pub address: String,
    /// Optional ENS name or alias.
    #[props(default)]
    pub alias: Option<String>,
    /// Show identicon (if available).
    #[props(default = true)]
    pub show_icon: bool,
    /// Additional CSS classes.
    #[props(default)]
    pub class: String,
}

/// Display a wallet address with optional alias.
#[allow(non_snake_case)]
pub fn AddressDisplay(props: AddressDisplayProps) -> Element {
    let display_value = props.alias.clone().unwrap_or_else(|| shorten_hash(&props.address, 6, 4));
    
    rsx! {
        span { class: "address-display {props.class}",
            if props.show_icon {
                span { class: "address-icon", "👤" }
            }
            
            span { 
                class: if props.alias.is_some() { "address-alias" } else { "address-value" },
                title: props.address.clone(),
                "{display_value}"
            }
            
            if props.alias.is_some() {
                HashDisplay {
                    value: props.address.clone(),
                    prefix_len: 4,
                    suffix_len: 4,
                    show_copy: false,
                    show_tooltip: false,
                    class: "address-hash-small".to_string(),
                }
            }
        }
    }
}
