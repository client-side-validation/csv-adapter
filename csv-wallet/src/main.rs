//! CSV Wallet — Standalone Multi-Chain Wallet with Dioxus UI

#![warn(missing_docs)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use dioxus::prelude::*;

mod chains;
mod components;
mod context;
mod hooks;
mod layout;
mod pages;
mod routes;
mod services;
mod storage;
mod wallet_core;

use context::WalletProvider;
use routes::Route;

fn main() {
    console_error_panic_hook::set_once();
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        // Critical reset (required for base styling)
        document::Style {
            r#type: "text/css",
            "{CRITICAL_CSS}"
        }

        // Animations and transitions
        document::Style {
            r#type: "text/css",
            "{GLOBAL_CSS}"
        }

        // Main app with all required providers
        WalletProvider {
            hooks::WalletProvider {
                hooks::NetworkProvider {
                    hooks::BalanceProvider {
                        hooks::WalletConnectionProvider {
                            Router::<Route> {}
                        }
                    }
                }
            }
        }
    }
}

const CRITICAL_CSS: &str = "
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
body { min-height: 100vh; background: #030712; color: #f3f4f6; font-family: system-ui, -apple-system, sans-serif; }
#main { display: block; }
";

const GLOBAL_CSS: &str = r#"
/* Page Transitions */
.page-enter {
    animation: pageFadeIn 0.3s ease-out;
}
@keyframes pageFadeIn {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: translateY(0); }
}

/* Stagger Children */
.stagger-children > * {
    animation: staggerFadeIn 0.4s ease-out backwards;
}
.stagger-children > *:nth-child(1) { animation-delay: 0.05s; }
.stagger-children > *:nth-child(2) { animation-delay: 0.1s; }
.stagger-children > *:nth-child(3) { animation-delay: 0.15s; }
.stagger-children > *:nth-child(4) { animation-delay: 0.2s; }
.stagger-children > *:nth-child(5) { animation-delay: 0.25s; }
.stagger-children > *:nth-child(6) { animation-delay: 0.3s; }
.stagger-children > *:nth-child(7) { animation-delay: 0.35s; }
.stagger-children > *:nth-child(8) { animation-delay: 0.4s; }
@keyframes staggerFadeIn {
    from { opacity: 0; transform: translateY(12px); }
    to { opacity: 1; transform: translateY(0); }
}

/* Button Ripple */
.btn-ripple {
    position: relative;
    overflow: hidden;
}
.btn-ripple::after {
    content: '';
    position: absolute;
    top: 50%; left: 50%;
    width: 0; height: 0;
    border-radius: 50%;
    background: rgba(255,255,255,0.15);
    transform: translate(-50%,-50%);
    transition: width 0.4s ease, height 0.4s ease, opacity 0.4s ease;
    opacity: 0;
}
.btn-ripple:active::after {
    width: 200px; height: 200px; opacity: 1; transition: 0s;
}

/* Card Hover */
.card-hover {
    transition: transform 0.2s ease, box-shadow 0.2s ease;
}
.card-hover:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0,0,0,0.3);
}

/* Pulse Glow */
.pulse-glow {
    animation: pulseGlow 2s ease-in-out infinite;
}
@keyframes pulseGlow {
    0%,100% { box-shadow: 0 0 4px rgba(59,130,246,0.3); }
    50% { box-shadow: 0 0 16px rgba(59,130,246,0.6); }
}

/* Status Pulse */
.status-online {
    animation: statusPulse 2s ease-in-out infinite;
}
@keyframes statusPulse {
    0%,100% { opacity: 1; }
    50% { opacity: 0.5; }
}

/* Count Up */
.count-up {
    animation: countUp 0.5s ease-out;
}
@keyframes countUp {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: translateY(0); }
}

/* Input Focus */
.input-focus {
    transition: border-color 0.2s ease, box-shadow 0.2s ease;
}
.input-focus:focus {
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59,130,246,0.15);
    outline: none;
}

/* Modal */
.modal-backdrop {
    animation: backdropFadeIn 0.2s ease-out;
}
@keyframes backdropFadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
}
.modal-content {
    animation: modalSlideIn 0.25s ease-out;
}
@keyframes modalSlideIn {
    from { opacity: 0; transform: scale(0.95) translateY(-10px); }
    to { opacity: 1; transform: scale(1) translateY(0); }
}

/* Scrollbar */
::-webkit-scrollbar { width: 8px; height: 8px; }
::-webkit-scrollbar-track { background: #111827; border-radius: 4px; }
::-webkit-scrollbar-thumb { background: #374151; border-radius: 4px; }
::-webkit-scrollbar-thumb:hover { background: #4b5563; }

/* Selection */
::selection {
    background: rgba(59,130,246,0.3);
    color: #f3f4f6;
}

/* Smooth scroll */
html { scroll-behavior: smooth; }
"#;
