#![allow(non_snake_case, unused)]
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

// When hydrating nested suspense boundaries, we still need to run code in the unresolved suspense boundary to replicate what the server has already done:
fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        button {
            id: "increment-carousel-button",
            onclick: move |_| count += 1,
            "Increment"
        }
        button {
            id: "decrement-carousel-button",
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
                    SuspendedComponent {
                        id: i
                    }
                }
            }
        }
        div { "footer 123" }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
enum ResolvedOn {
    Server,
    Client,
}

impl ResolvedOn {
    #[cfg(feature = "web")]
    const CURRENT: Self = Self::Client;
    #[cfg(not(feature = "web"))]
    const CURRENT: Self = Self::Server;
}

#[component]
fn SuspendedComponent(id: i32) -> Element {
    let resolved_on = use_server_future(move || async move {
        async_std::task::sleep(std::time::Duration::from_secs(1)).await;
        ResolvedOn::CURRENT
    })?()
    .unwrap();

    let mut count = use_signal(|| 0);

    rsx! {
        div {
            id: "outer-{id}",
            "outer suspense result: {resolved_on:?}"
            button {
                id: "outer-button-{id}",
                onclick: move |_| count += 1,
                "{count}"
            }
            SuspenseBoundary {
                fallback: |_| rsx! {
                    "Loading... more"
                },
                NestedSuspendedComponent {
                    id
                }
            }
        }
    }
}

#[component]
fn NestedSuspendedComponent(id: i32) -> Element {
    let resolved_on = use_server_future(move || async move {
        async_std::task::sleep(std::time::Duration::from_secs(1)).await;
        ResolvedOn::CURRENT
    })?()
    .unwrap();
    let mut count = use_signal(|| 0);
    rsx! {
        div {
            "nested suspense result: {resolved_on:?}"
            button {
                id: "nested-button-{id}",
                onclick: move |_| count += 1,
                "{count}"
            }
        }
    }
}

fn main() {
    launch(app);
}
