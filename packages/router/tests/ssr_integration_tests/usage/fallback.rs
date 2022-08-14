use dioxus::prelude::*;
use dioxus_router::prelude::*;

use crate::{render, test_routes};

#[test]
fn root_fallback() {
    assert_eq!("<p>Root Fallback</p>", render(App));

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                initial_path: "/invalid",

                Outlet { }
            }
        })
    }
}

#[test]
fn nested_fallback() {
    assert_eq!("<p>Nested Fallback</p>", render(App));

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                initial_path: "/test/nest/invalid",

                Outlet { }
            }
        })
    }
}
