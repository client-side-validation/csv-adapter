//! CSV Wallet — Standalone Multi-Chain Wallet with Dioxus UI

#![warn(missing_docs)]
#![allow(dead_code)]

use dioxus::prelude::*;

mod routes;
mod context;
mod wallet_core;
mod storage;
mod pages;
mod components;
mod hooks;
mod services;

use routes::Route;
use context::WalletProvider;
use components::{Sidebar, Header};

fn main() {
    console_error_panic_hook::set_once();
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut ready = use_signal(|| false);

    use_effect(move || {
        use gloo_timers::future::sleep;
        use std::time::Duration;

        wasm_bindgen_futures::spawn_local(async move {
            // Wait for Tailwind Play CDN to scan and apply classes
            sleep(Duration::from_millis(300)).await;
            ready.set(true);
        });
    });

    rsx! {
        // Tailwind CSS CDN (play CDN auto-scans classes)
        document::Script {
            src: "https://cdn.tailwindcss.com",
        }

        // Critical reset
        document::Style {
            r#type: "text/css",
            "{CRITICAL_CSS}"
        }

        // Animations and transitions
        document::Style {
            r#type: "text/css",
            "{GLOBAL_CSS}"
        }

        if *ready.read() {
            // Main app with providers
            WalletProvider {
                Router::<Route> {}
            }
        } else {
            // Loading screen (inline styles, no Tailwind needed)
            div {
                style: "min-height:100vh;display:flex;align-items:center;justify-content:center;background:#030712;color:#f3f4f6;",
                div { style: "text-align:center;",
                    div { style: "font-size:48px;margin-bottom:16px;", "\u{1F510}" }
                    p { style: "color:#9ca3af;font-size:14px;", "Loading CSS Wallet\u{2026}" }
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

// ===== Layout =====
/// Main application layout component.
#[component]
pub fn Layout() -> Element {
    let mut sidebar_open = use_signal(|| true);

    rsx! {
        div { class: "min-h-screen bg-gray-950 text-gray-100 flex",
            Sidebar { sidebar_open: *sidebar_open.read() }

            // Main content area
            div { class: "flex-1 flex flex-col min-w-0",
                Header {
                    sidebar_open: *sidebar_open.read(),
                    on_sidebar_toggle: move |_| {
                        let open = *sidebar_open.read();
                        sidebar_open.set(!open);
                    },
                }

                // Page content with fade-in transition
                main { class: "flex-1 px-4 sm:px-6 lg:px-8 py-6 overflow-auto",
                    div { class: "page-enter",
                        Outlet::<Route> {}
                    }
                }
            }
        }
    }
}
