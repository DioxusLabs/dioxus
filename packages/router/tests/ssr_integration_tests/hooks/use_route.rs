use dioxus::prelude::*;
use dioxus_router::prelude::*;

use crate::{render, test_routes};

#[test]
fn with_router() {
    assert_eq!("<p>path: /</p>", render(App));

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                ComponentWithHook { }
            }
        })
    }
}

#[cfg(debug_assertions)]
#[test]
#[should_panic = "`use_route` can only be used in descendants of a `Router`"]
fn without_router_panic_in_debug() {
    render(ComponentWithHook);
}

#[cfg(not(debug_assertions))]
#[test]
fn without_router_ignore_in_release() {
    assert_eq!("<p>path: unknown</p>", render(ComponentWithHook));
}

#[allow(non_snake_case)]
fn ComponentWithHook(cx: Scope) -> Element {
    let route = use_route(&cx);
    let path = route
        .map(|r| r.path.clone())
        .unwrap_or(String::from("unknown"));

    cx.render(rsx! {
        p { "path: {path}" }
    })
}
