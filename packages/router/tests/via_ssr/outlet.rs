use dioxus::prelude::*;
use dioxus_router::{history::MemoryHistory, prelude::*};

fn prepare(path: impl Into<String>) -> VirtualDom {
    #![allow(non_snake_case)]

    let mut vdom = VirtualDom::new_with_props(App, AppProps { path: path.into() });
    let _ = vdom.rebuild();
    return vdom;

    #[derive(Debug, Props, PartialEq)]
    struct AppProps {
        path: String,
    }

    fn App(cx: Scope<AppProps>) -> Element {
        use_router(
            cx,
            &|| RouterConfiguration {
                synchronous: true,
                history: Box::new(MemoryHistory::with_initial_path(cx.props.path.clone()).unwrap()),
                ..Default::default()
            },
            &|| {
                Segment::content(comp(RootIndex))
                    .fixed(
                        "fixed",
                        Route::content(comp(Fixed)).nested(
                            Segment::content(comp(FixedIndex)).fixed("fixed", comp(FixedFixed)),
                        ),
                    )
                    .catch_all(ParameterRoute::content::<u8>(comp(Parameter)).nested(
                        Segment::content(comp(ParameterIndex)).fixed("fixed", comp(ParameterFixed)),
                    ))
            },
        );

        render! {
            h1 { "App" }
            Outlet { }
        }
    }

    fn RootIndex(cx: Scope) -> Element {
        render! {
            h2 { "Root Index" }
        }
    }

    fn Fixed(cx: Scope) -> Element {
        render! {
            h2 { "Fixed" }
            Outlet { }
        }
    }

    fn FixedIndex(cx: Scope) -> Element {
        render! {
            h3 { "Fixed - Index" }
        }
    }

    fn FixedFixed(cx: Scope) -> Element {
        render! {
            h3 { "Fixed - Fixed"}
        }
    }

    fn Parameter(cx: Scope) -> Element {
        let val = use_route(cx)?.parameter::<u8>().unwrap();

        render! {
            h2 { "Parameter {val}" }
            Outlet { }
        }
    }

    fn ParameterIndex(cx: Scope) -> Element {
        render! {
            h3 { "Parameter - Index" }
        }
    }

    fn ParameterFixed(cx: Scope) -> Element {
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
