use dioxus::prelude::*;

use crate::{render, test_routes};

#[test]
fn non_nested_router() {
    render(App);

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx)
            }
        })
    }
}

#[cfg(debug_assertions)]
#[test]
#[should_panic = "`Router` can not be used as a descendent of a `Router`"]
fn nested_routers_panic_in_debug() {
    render(NestedRouters);
}

#[cfg(not(debug_assertions))]
#[test]
fn nested_routes_ignore_in_release() {
    assert_eq!("<!--placeholder-->", render(NestedRouters));
}

#[allow(non_snake_case)]
fn NestedRouters(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            routes: test_routes(&cx),
            Router {
                routes: test_routes(&cx),
            }
        }
    })
}
