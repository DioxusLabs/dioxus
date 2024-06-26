//! Run with:
//!
//! ```sh
//! dx serve --platform fullstack
//! ```

#![allow(non_snake_case, unused)]
use dioxus::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};

// When hydrating nested suspense boundaries, we still need to run code in the unresolved suspense boundary to replicate what the server has already done:
fn app() -> Element {
    let mut count = use_signal(|| 1234);

    if cfg!(feature = "web") {
        match generation() {
            0 => {
                needs_update();
            }
            1 => {
                count.set(count() + 100);
            }
            _ => {}
        }
    }

    rsx! {
        div {
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
            for i in count()..count() + 200 {
                // Imagine, we just resolve this suspense boundary. We pass down whatever data we resolved with it and None for any unresolved server functions in nested server functions [Some(data), None]
                SuspenseBoundary {
                    key: "{i}",
                    fallback: |_| rsx! {
                        "Loading..."
                    },
                    SuspendedComponent {}
                }
            }
            div { "footer 123" }
        }
    }
}

#[component]
fn SuspendedComponent() -> Element {
    let mut count = use_signal(|| 0);

    use_server_future(move || {
        // let count = count();
        async move {
            async_std::task::sleep(std::time::Duration::from_millis(
                rand::thread_rng().gen_range(0..1000) + 1000,
            ))
            .await;
            1234
        }
    })?;

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
        async_std::task::sleep(std::time::Duration::from_millis(
            rand::thread_rng().gen_range(0..1000) + 1000,
        ))
        .await;
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
