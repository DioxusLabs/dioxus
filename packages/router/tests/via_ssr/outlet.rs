#![allow(non_snake_case, unused)]

use dioxus::prelude::*;
use dioxus_router::{history::MemoryHistory, prelude::*};

fn prepare(path: impl Into<String>) -> VirtualDom {
    let mut vdom = VirtualDom::new_with_props(App, AppProps { path: path.into() });
    let _ = vdom.rebuild();
    return vdom;

    #[derive(Routable, Clone)]
    #[rustfmt::skip]
    enum Route {
        #[route("/")]
        RootIndex {},
        #[nest("/fixed")]
            #[layout(Fixed)]
                #[route("/")]
                FixedIndex {},
                #[route("/fixed")]
                FixedFixed {},
            #[end_layout]
        #[end_nest]
        #[nest("/:id")]
            #[layout(Parameter)]
                #[route("/")]
                ParameterIndex { id: u8 },
                #[route("/fixed")]
                ParameterFixed { id: u8 },
    }

    #[derive(Debug, Props, PartialEq)]
    struct AppProps {
        path: String,
    }

    fn App(cx: Scope<AppProps>) -> Element {
        let cfg = RouterConfiguration {
            history: Box::new(MemoryHistory::with_initial_path(cx.props.path.clone()).unwrap()),
            ..Default::default()
        };

        render! {
            h1 { "App" }
            Router {
                config: cfg
            }
        }
    }

    #[inline_props]
    fn RootIndex(cx: Scope) -> Element {
        render! {
            h2 { "Root Index" }
        }
    }

    #[inline_props]
    fn Fixed(cx: Scope) -> Element {
        render! {
            h2 { "Fixed" }
            Outlet { }
        }
    }

    #[inline_props]
    fn FixedIndex(cx: Scope) -> Element {
        render! {
            h3 { "Fixed - Index" }
        }
    }

    #[inline_props]
    fn FixedFixed(cx: Scope) -> Element {
        render! {
            h3 { "Fixed - Fixed"}
        }
    }

    #[inline_props]
    fn Parameter(cx: Scope, id: u8) -> Element {
        render! {
            h2 { "Parameter {id}" }
            Outlet { }
        }
    }

    #[inline_props]
    fn ParameterIndex(cx: Scope, id: u8) -> Element {
        render! {
            h3 { "Parameter - Index" }
        }
    }

    #[inline_props]
    fn ParameterFixed(cx: Scope, id: u8) -> Element {
        render! {
            h3 { "Parameter - Fixed" }
        }
    }
}

#[test]
fn root_index() {
    let vdom = prepare("/");
    let html = dioxus_ssr::render(&vdom);

    assert_eq!(html, "<h1>App</h1><h2>Root Index</h2>");
}

#[test]
fn fixed() {
    let vdom = prepare("/fixed");
    let html = dioxus_ssr::render(&vdom);

    assert_eq!(html, "<h1>App</h1><h2>Fixed</h2><h3>Fixed - Index</h3>");
}

#[test]
fn fixed_fixed() {
    let vdom = prepare("/fixed/fixed");
    let html = dioxus_ssr::render(&vdom);

    assert_eq!(html, "<h1>App</h1><h2>Fixed</h2><h3>Fixed - Fixed</h3>");
}

#[test]
fn parameter() {
    let vdom = prepare("/18");
    let html = dioxus_ssr::render(&vdom);

    assert_eq!(
        html,
        "<h1>App</h1><h2>Parameter 18</h2><h3>Parameter - Index</h3>"
    );
}

#[test]
fn parameter_fixed() {
    let vdom = prepare("/18/fixed");
    let html = dioxus_ssr::render(&vdom);

    assert_eq!(
        html,
        "<h1>App</h1><h2>Parameter 18</h2><h3>Parameter - Fixed</h3>"
    );
}
