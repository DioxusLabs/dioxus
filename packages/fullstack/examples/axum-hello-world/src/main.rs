//! Run with:
//!
//! ```sh
//! dx serve --platform fullstack
//! ```

#![allow(non_snake_case, unused)]
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

// When hydrating nested suspense boundaries, we still need to run code in the unresolved suspense boundary to replicate what the server has already done:
fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        button {
            onclick: move |_| count += 1,
            "Increment"
        }
        button {
            onclick: move |_| count -= 1,
            "Decrement"
        }
        div {
            "Hello world"
        }
        div {
            for i in count()..count() + 3 {
                // Imagine, we just resolve this suspense boundary. We pass down whatever data we resolved with it and None for any unresolved server functions in nested server functions [Some(data), None]
                SuspenseBoundary {
                    key: "{i}",
                    fallback: |_| rsx! {
                        "Loading..."
                    },
                    SuspendedComponent {}
                }
            }
        }
        div { "footer 123" }
    }
}

#[component]
fn SuspendedComponent() -> Element {
    use_server_future(move || async move {
        async_std::task::sleep(std::time::Duration::from_secs(1)).await;
        1234
    })?;

    let mut count = use_signal(|| 0);

    rsx! {
        "Suspended???"
        button {
            onclick: move |_| count += 1,
            "first {count}"
        }
        SuspenseBoundary {
            fallback: |_| rsx! {
                "Loading... more"
            },
            NestedSuspendedComponent {}
        }
    }
}

#[component]
fn NestedSuspendedComponent() -> Element {
    use_server_future(move || async move {
        async_std::task::sleep(std::time::Duration::from_secs(1)).await;
        12345678
    })?;
    let mut count = use_signal(|| 0);
    rsx! {
        "Suspended Nested"
        button {
            onclick: move |_| count += 1,
            "{count}"
        }
    }
}

fn main() {
    #[cfg(feature = "web")]
    tracing_wasm::set_as_global_default();

    #[cfg(feature = "server")]
    tracing_subscriber::fmt::init();

    launch(app);
}
