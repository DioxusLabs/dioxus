//! Regression test for graceful hydration mismatch recovery.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let count = use_signal(|| 0);

    rsx! {
        div {
            id: "app-root",
            h1 { "Hydration recovery regression" }
            RecoveryButton { count }
            NestedMismatch {}
            TextMismatch {}
            AttributeMismatch {}
            PlaceholderMismatch {}
        }
    }
}

#[component]
fn RecoveryButton(mut count: Signal<i32>) -> Element {
    if cfg!(target_arch = "wasm32") {
        rsx! {
            button {
                id: "recovery-button",
                onclick: move |_| count += 1,
                "Recovered {count}"
            }
        }
    } else {
        rsx! {
            div {
                id: "recovery-button",
                "Recovered 0"
            }
        }
    }
}

#[component]
fn NestedMismatch() -> Element {
    rsx! {
        section {
            id: "nested-mismatch-shell",
            h2 { "Nested mismatch" }
            div {
                class: "nested-wrapper",
                ul {
                    li {
                        NestedLeaf {}
                    }
                }
            }
        }
    }
}

#[component]
fn NestedLeaf() -> Element {
    if cfg!(target_arch = "wasm32") {
        rsx! {
            strong {
                id: "nested-leaf",
                "Nested client leaf"
            }
        }
    } else {
        rsx! {
            span {
                id: "nested-leaf",
                "Nested client leaf"
            }
        }
    }
}

#[component]
fn TextMismatch() -> Element {
    let text = if cfg!(target_arch = "wasm32") {
        "Client text content"
    } else {
        "Server text content"
    };

    rsx! {
        section {
            id: "text-mismatch-shell",
            h2 { "Text mismatch" }
            p {
                id: "text-mismatch",
                "{text}"
            }
        }
    }
}

#[component]
fn AttributeMismatch() -> Element {
    if cfg!(target_arch = "wasm32") {
        let client_title = "Client attribute title";

        rsx! {
            section {
                id: "attribute-mismatch-shell",
                h2 { "Attribute mismatch" }
                div {
                    id: "attribute-mismatch",
                    role: "status",
                    title: client_title,
                    "Attribute branch"
                }
            }
        }
    } else {
        rsx! {
            section {
                id: "attribute-mismatch-shell",
                h2 { "Attribute mismatch" }
                div {
                    id: "attribute-mismatch",
                    "Attribute branch"
                }
            }
        }
    }
}

#[component]
fn PlaceholderMismatch() -> Element {
    rsx! {
        section {
            id: "placeholder-mismatch-shell",
            h2 { "Placeholder mismatch" }
            PlaceholderSlot {}
        }
    }
}

#[component]
fn PlaceholderSlot() -> Element {
    if cfg!(target_arch = "wasm32") {
        Ok(VNode::placeholder())
    } else {
        rsx! {
            p {
                id: "server-placeholder-content",
                "Server placeholder content"
            }
        }
    }
}
