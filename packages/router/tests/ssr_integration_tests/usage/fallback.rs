use dioxus::prelude::*;
use dioxus_router::{history::MemoryHistory, prelude::*};

use crate::{render, test_routes};

#[test]
fn root_fallback() {
    assert_eq!("<p>Root Fallback</p>", render(App));

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                history: &|| MemoryHistory::with_first(String::from("/invalid")),

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
                init_only: true,
                history: &|| MemoryHistory::with_first(String::from("/test/nest/invalid")),

                Outlet { }
            }
        })
    }
}
