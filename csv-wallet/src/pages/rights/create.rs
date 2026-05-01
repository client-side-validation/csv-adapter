//! Create right page.

use crate::context::{generate_id, use_wallet_context, RightStatus, TrackedRight};
use crate::pages::common::*;
use csv_adapter_core::Chain;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn CreateRight() -> Element {
    let mut wallet_ctx = use_wallet_context();

    if let Some(n) = wallet_ctx.notification() {
        return rsx! {
            div { class: "max-w-2xl space-y-6",
                {notification_banner(n.kind, n.message, move || { wallet_ctx.clear_notification(); })}
                CreateRightForm {}
            }
        };
    }

    rsx! {
        div { class: "max-w-2xl space-y-6",
            CreateRightForm {}
        }
    }
}

#[component]
pub fn CreateRightForm() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Bitcoin);
    let mut value = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);
    let mut error = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "{card_class()} p-6 space-y-5",
            div { class: "{card_header_class()} -mx-6 -mt-6 mb-4",
                h2 { class: "font-semibold text-sm", "Create New Right" }
            }

            {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
            }, *selected_chain.read()))}

            {form_field("Value (optional)", rsx! {
                input {
                    value: "{value.read()}",
                    oninput: move |evt| { value.set(evt.value()); },
                    class: "{input_mono_class()}",
                    r#type: "text",
                    r#type: "text"
                }
            })}

            if let Some(e) = error.read().as_ref().cloned() {
                div { class: "p-3 bg-red-900/30 border border-red-700/50 rounded-lg text-sm text-red-300", "{e}" }
            }

            if let Some(right_id) = result.read().clone() {
                div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg space-y-2",
                    p { class: "text-green-300 font-medium", "Right Created!" }
                    p { class: "font-mono text-sm text-green-400 break-all", "{right_id}" }
                    div { class: "flex gap-2 mt-2",
                        button {
                            onclick: move |_| {
                                let right = TrackedRight {
                                    id: right_id.clone(),
                                    chain: *selected_chain.read(),
                                    value: value.read().parse().unwrap_or(0),
                                    status: RightStatus::Active,
                                    owner: wallet_ctx.address_for_chain(*selected_chain.read()).unwrap_or_default(),
                                };
                                wallet_ctx.add_right(right);
                                result.set(None);
                                value.set(String::new());
                            },
                            class: "{btn_primary_class()}",
                            "Save to State"
                        }
                        button {
                            onclick: move |_| { result.set(None); },
                            class: "{btn_secondary_class()}",
                            "Dismiss"
                        }
                    }
                }
            }

            button {
                onclick: move |_| {
                    let new_id = generate_id();
                    result.set(Some(new_id));
                    error.set(None);
                },
                class: "{btn_full_primary_class()}",
                "Create Right"
            }
        }
    }
}

fn notification_banner(
    kind: crate::context::NotificationKind,
    message: String,
    on_close: impl FnOnce() + 'static,
) -> Element {
    let (bg_class, icon) = match kind {
        crate::context::NotificationKind::Success => (
            "bg-green-900/30 border-green-700/50 text-green-300",
            "\u{2705}",
        ),
        crate::context::NotificationKind::Error => {
            ("bg-red-900/30 border-red-700/50 text-red-300", "\u{274C}")
        }
        crate::context::NotificationKind::Info => (
            "bg-blue-900/30 border-blue-700/50 text-blue-300",
            "\u{2139}",
        ),
        crate::context::NotificationKind::Warning => (
            "bg-yellow-900/30 border-yellow-700/50 text-yellow-300",
            "\u{26A0}",
        ),
    };

    let on_close_cell = std::cell::RefCell::new(Some(on_close));

    rsx! {
        div { class: "p-4 {bg_class} rounded-lg flex items-center justify-between",
            div { class: "flex items-center gap-2",
                span { "{icon}" }
                p { "{message}" }
            }
            button {
                onclick: move |_| {
                    if let Some(cb) = on_close_cell.borrow_mut().take() {
                        cb();
                    }
                },
                class: "text-sm hover:opacity-70",
                "\u{2715}"
            }
        }
    }
}
