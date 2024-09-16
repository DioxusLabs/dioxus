// This test is used by playwright configured in the root of the repo
// Tests:
// - Streaming hydration
// - Suspense
// - Server futures

#![allow(non_snake_case, unused)]
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

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
        sleep(1000).await;
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
        sleep(1000).await;
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

async fn sleep(millis: u64) {
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::sleep(std::time::Duration::from_millis(millis)).await;

    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
}

fn main() {
    launch(app);
}
