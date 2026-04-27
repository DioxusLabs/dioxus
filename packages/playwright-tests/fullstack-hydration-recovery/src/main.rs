//! Regression test for graceful hydration mismatch recovery.

use dioxus::{fullstack::commit_initial_chunk, prelude::*};

fn main() {
    LaunchBuilder::new()
        .with_cfg(server_only! {
            dioxus::server::ServeConfig::builder().enable_out_of_order_streaming()
        })
        .launch(App);
}

#[component]
fn App() -> Element {
    use_hook(commit_initial_chunk);
    let count = use_signal(|| 0);

    rsx! {
        div {
            id: "app-root",
            h1 { "Hydration recovery regression" }
            RecoveryButton { count }
            NestedMismatch {}
            TextMismatch {}
            AttributeMismatch {}
            AttributeValueMismatch {}
            DangerousInnerHtmlStable {}
            DangerousInnerHtmlMismatch {}
            StyleMismatch {}
            WhitespaceMismatch {}
            ExtraNodeMismatch {}
            PlaceholderMismatch {}
            SuspenseBoundary {
                fallback: |_| rsx! { div { id: "streaming-fallback", "Loading streaming…" } },
                StreamingMismatch {}
            }
            div {
                id: "after-streaming-boundary",
                "After streaming boundary"
            }
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
fn AttributeValueMismatch() -> Element {
    let title = if cfg!(target_arch = "wasm32") {
        "Client value title"
    } else {
        "Server value title"
    };
    let data_side = if cfg!(target_arch = "wasm32") {
        "client"
    } else {
        "server"
    };

    rsx! {
        section {
            id: "attribute-value-mismatch-shell",
            h2 { "Attribute value mismatch" }
            div {
                id: "attribute-value-mismatch",
                title,
                "data-side": data_side,
                "Attribute value branch"
            }
        }
    }
}

#[component]
fn DangerousInnerHtmlStable() -> Element {
    let inner_html = "<strong id='dangerous-inner-html-child'>Shared inner html</strong>";

    rsx! {
        section {
            id: "dangerous-inner-html-stable-shell",
            h2 { "Dangerous inner html stable" }
            div {
                id: "dangerous-inner-html-stable",
                dangerous_inner_html: inner_html,
            }
        }
    }
}

#[component]
fn DangerousInnerHtmlMismatch() -> Element {
    let inner_html = if cfg!(target_arch = "wasm32") {
        "<strong id='dangerous-inner-html-mismatch-child'>Client dangerous inner html</strong>"
    } else {
        "<em id='dangerous-inner-html-mismatch-child'>Server dangerous inner html</em>"
    };

    rsx! {
        section {
            id: "dangerous-inner-html-mismatch-shell",
            h2 { "Dangerous inner html mismatch" }
            div {
                id: "dangerous-inner-html-mismatch",
                dangerous_inner_html: inner_html,
            }
        }
    }
}

#[component]
fn StyleMismatch() -> Element {
    let width = if cfg!(target_arch = "wasm32") {
        "200px"
    } else {
        "100px"
    };
    let height = if cfg!(target_arch = "wasm32") {
        "50px"
    } else {
        "40px"
    };

    rsx! {
        section {
            id: "style-mismatch-shell",
            h2 { "Style mismatch" }
            div {
                id: "style-mismatch",
                width,
                height,
                "Style branch"
            }
        }
    }
}

#[component]
fn WhitespaceMismatch() -> Element {
    let text = if cfg!(target_arch = "wasm32") {
        "  Client whitespace content  "
    } else {
        "Client whitespace content"
    };

    rsx! {
        section {
            id: "whitespace-mismatch-shell",
            h2 { "Whitespace mismatch" }
            pre {
                id: "whitespace-mismatch",
                "{text}"
            }
        }
    }
}

#[component]
fn ExtraNodeMismatch() -> Element {
    rsx! {
        section {
            id: "extra-node-mismatch-shell",
            h2 { "Extra node mismatch" }
            div {
                id: "extra-node-stable",
                "Shared child"
            }
            if !cfg!(target_arch = "wasm32") {
                p {
                    id: "server-extra-node",
                    "Server extra node"
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

/// A component that lives inside a SuspenseBoundary and introduces a tag
/// mismatch after the suspense boundary streams in from the server.
#[component]
fn StreamingMismatch() -> Element {
    // use_server_future suspends until the server resolves.
    let value = use_server_future(|| async {
        async_std::task::sleep(std::time::Duration::from_millis(600)).await;
        "streamed data".to_string()
    })?()
    .unwrap();

    // After resolution the client and server disagree on the tag.
    if cfg!(target_arch = "wasm32") {
        rsx! {
            section {
                id: "streaming-mismatch-shell",
                h2 { "Streaming mismatch" }
                button {
                    id: "streaming-mismatch",
                    "Streaming client: {value}"
                }
            }
        }
    } else {
        rsx! {
            section {
                id: "streaming-mismatch-shell",
                h2 { "Streaming mismatch" }
                div {
                    id: "streaming-mismatch",
                    "Streaming server: {value}"
                }
            }
        }
    }
}
