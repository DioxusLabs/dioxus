use dioxus::prelude::*;
use dioxus_router::history::MemoryHistoryProvider;

use crate::{render, test_routes};

#[test]
fn index() {
    assert_eq!("<p>test1</p>", render(App));

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                Outlet {}
            }
        })
    }
}

#[test]
fn route() {
    assert_eq!("<p>test2</p><p>test3</p>", render(App));

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                history: &|| MemoryHistoryProvider::with_first(String::from("/test")),

                Outlet {}
            }
        })
    }
}

#[test]
fn nested_route() {
    assert_eq!("<p>test2</p><p>test4</p>", render(App));

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                history: &|| MemoryHistoryProvider::with_first(String::from("/test/nest")),

                Outlet {}
            }
        })
    }
}

#[test]
fn with_depth() {
    assert_eq!("<p>test3</p>", render(App));

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router{
                routes: test_routes(&cx),
                init_only: true,
                history: &|| MemoryHistoryProvider::with_first(String::from("/test")),

                Outlet {
                    depth: 1
                }
            }
        })
    }
}

#[test]
fn with_name() {
    assert_eq!("<p>test5</p>", render(App));

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                history: &|| MemoryHistoryProvider::with_first(String::from("/test")),

                Outlet {
                    name: "other"
                }
            }
        })
    }
}

#[cfg(debug_assertions)]
#[test]
#[should_panic = "`Outlet` can only be used as a descendent of a `Router`"]
fn without_router_panic_in_debug() {
    render(OutletWithoutRouter);
}

#[cfg(not(debug_assertions))]
#[test]
fn without_router_ignore_in_release() {
    render(OutletWithoutRouter);
}

#[allow(non_snake_case)]
fn OutletWithoutRouter(cx: Scope) -> Element {
    cx.render(rsx! {
        Outlet {}
    })
}
