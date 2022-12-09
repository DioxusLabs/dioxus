#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::prelude::*;
use dioxus_router_core::history::MemoryHistory;

fn main() {
    dioxus_desktop::launch(App);
}

fn App(cx: Scope) -> Element {
    use_router(
        &cx,
        &|| RouterConfiguration {
            // history: Box::new(MemoryHistory::with_initial_path("/apple").unwrap()),
            ..Default::default()
        },
        &|| Segment::content(comp(RootIndex)).fixed("apple", comp(Apple)),
    );

    render! {
        h1 { "Simple Example App" }
        nav {
            Link {
                target: named::<RootIndex>(),
                "Go to root"
            }
        }
        Outlet { }
    }
}

fn RootIndex(cx: Scope) -> Element {
    render! {
        h2 { "Root Index" }
        ul {
            li {
                Link {
                    target: "/apple",
                    "Read about apples…"
                }
            }
        }
    }
}

fn Apple(cx: Scope) -> Element {
    render! {
        h2 { "Apple" }
        p {
            "An apple is a tasty fruit. It grows on trees and many varieties are either red or "
            "green."
        }
    }
}
