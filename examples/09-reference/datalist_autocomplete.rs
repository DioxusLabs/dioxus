//! Regression test for datalist autocomplete + keyboard events.
//!
//! Browsers (and desktop webviews) may dispatch a plain `Event` with type "keydown"
//! when the user selects a datalist suggestion. Because it is not a real `KeyboardEvent`,
//! it lacks properties like `key`, `code`, and `keyCode`. Without proper handling, this
//! causes a panic (BorrowMutError or deserialization failure).
//!
//! To test:
//! 1. Run this example with `dx serve --example datalist_autocomplete` (web) or
//!    `dx serve --example datalist_autocomplete --platform desktop` (desktop).
//! 2. Click on the input field and type "a".
//! 3. A datalist suggestion "apple" should appear.
//! 4. Click on the suggestion to select it.
//! 5. The app should NOT crash. The event log should show the events that fired.
//!
//! See https://github.com/DioxusLabs/dioxus/issues/5375

use dioxus::prelude::*;
use std::collections::VecDeque;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut events = use_signal(VecDeque::<String>::new);

    let mut log = move |msg: String| {
        let mut ev = events.write();
        if ev.len() >= 30 {
            ev.pop_front();
        }
        ev.push_back(msg);
    };

    rsx! {
        div { style: "font-family: sans-serif; max-width: 600px; margin: 40px auto;",
            h2 { "Datalist autocomplete test" }
            p { "Type \"a\" in the input below and select a suggestion from the dropdown." }
            p { "The app should not crash when you pick a suggestion." }

            div {
                style: "padding: 16px; border: 1px solid #ccc; border-radius: 8px; margin-bottom: 16px;",
                onkeydown: move |e: KeyboardEvent| {
                    log(format!("keydown: key={:?} code={:?}", e.key(), e.code()));
                },
                onkeyup: move |e: KeyboardEvent| {
                    log(format!("keyup: key={:?} code={:?}", e.key(), e.code()));
                },

                label { r#for: "fruit", "Pick a fruit: " }
                input {
                    id: "fruit",
                    list: "fruit-list",
                    placeholder: "Start typing...",
                    style: "padding: 8px; font-size: 16px; width: 100%;",
                    oninput: move |e: FormEvent| {
                        log(format!("input: value={:?}", e.value()));
                    },
                    onchange: move |e: FormEvent| {
                        log(format!("change: value={:?}", e.value()));
                    },
                }
                datalist { id: "fruit-list",
                    option { value: "apple" }
                    option { value: "apricot" }
                    option { value: "avocado" }
                    option { value: "banana" }
                    option { value: "blueberry" }
                    option { value: "cherry" }
                    option { value: "grape" }
                    option { value: "orange" }
                    option { value: "peach" }
                    option { value: "strawberry" }
                }
            }

            h3 { "Event log" }
            div {
                style: "background: #f5f5f5; padding: 12px; border-radius: 8px; font-family: monospace; font-size: 13px; max-height: 300px; overflow-y: auto;",
                if events.read().is_empty() {
                    p { style: "color: #999;", "No events yet..." }
                }
                for (i, event) in events.read().iter().enumerate() {
                    div { key: "{i}", "{event}" }
                }
            }
        }
    }
}
