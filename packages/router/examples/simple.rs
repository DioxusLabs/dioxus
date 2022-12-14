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
        &|| {
            Segment::content(comp(Home))
                .fixed("apple", comp(Apple))
                .fixed("potato", Route::content(comp(Potato)).name::<PotatoName>())
                .fixed("earth_apple", named::<PotatoName>())
        },
    );

    render! {
            h1 { "Simple Example App" }
            Outlet { }
            Link {
            target: named::<RootIndex>(),
            "Go to root"
        }
    }
}

fn Home(cx: Scope) -> Element {
    render! {
        h2 { "Root Index" }
        ul {
            li { Link {
                target: "/apple",
                "Read about apples…"
            } }
            li { Link {
                target: named::<PotatoName>(),
                "Read about potatoes…"
            } }
            li { Link {
                target: "/earth_apple",
                "Read about earth apples (literal translation of a german word for potato)…"
            } }
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

struct PotatoName;
fn Potato(cx: Scope) -> Element {
    render! {
        h2 { "Potato" }
        p { "The potato grows underground. There are many recipes involving potatoes." }
    }
}
