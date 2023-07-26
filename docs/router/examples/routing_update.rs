#![allow(non_snake_case, unused)]
use dioxus::prelude::*;
use dioxus_router::prelude::*;

// ANCHOR: router
#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[route("/")]
    Index {},
    #[route("/home")]
    Home {},
}

#[inline_props]
fn Home(cx: Scope) -> Element {
    render! {
        p { "Home" }
    }
}

#[inline_props]
fn Index(cx: Scope) -> Element {
    render! {
        p { "Index" }
    }
}

fn app(cx: Scope) -> Element {
    render! {
        Router::<Route> {
            config: || RouterConfig::default().on_update(|state|{
                (state.current() == Route::Index {}).then_some(
                    NavigationTarget::Internal(Route::Home {})
                )
            })
        }
    }
}
// ANCHOR_END: router

fn main() {}
