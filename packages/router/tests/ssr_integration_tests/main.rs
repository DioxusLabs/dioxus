use dioxus::prelude::*;
use dioxus_router::prelude::*;
use regex::Regex;
use std::sync::Arc;

const ADDRESS: &str = "https://dioxuslabs.com/";

struct TestName;

fn render(component: Component) -> String {
    let mut app = VirtualDom::new(component);
    app.rebuild();
    dioxus_ssr::render_vdom(&app)
}

fn test_routes(cx: &ScopeState) -> Arc<Segment> {
    use_segment(&cx, test_routes_segment).clone()
}

fn test_routes_segment() -> Segment {
    Segment::new()
        .index(TestComponent_0 as Component)
        .fixed(
            "test",
            Route::new(RcMulti(
                TestComponent_1,
                vec![("other", TestComponent_1_0_other)],
            ))
            .name(TestName)
            .nested(
                Segment::new()
                    .index(TestComponent_1_0 as Component)
                    .fixed(
                        "nest",
                        Route::new(TestComponent_1_1 as Component).nested(
                            Segment::new()
                                .fixed("double-nest", TestComponent_1_1_0 as Component)
                                .fallback(NestedFallback as Component),
                        ),
                    )
                    .parameter(("parameter", "/")),
            ),
        )
        .fixed("external-navigation-failure", "https://dioxuslabs.com/")
        .fixed("named-navigation-failure", ("invalid name", []))
        .fixed("redirect", "/test")
        .matching(Regex::new("other").unwrap(), ("matching-parameter", "/"))
        .fallback(RootFallback as Component)
}

#[allow(non_snake_case)]
fn TestComponent_0(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "0: index" }
    })
}

#[allow(non_snake_case)]
fn TestComponent_1(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "0: test" }
        Outlet { }
    })
}

#[allow(non_snake_case)]
fn TestComponent_1_0(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "1: index" }
    })
}

#[allow(non_snake_case)]
fn TestComponent_1_0_other(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "1: index, other" }
    })
}

#[allow(non_snake_case)]
fn TestComponent_1_1(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "1: nest" }
        Outlet { }
    })
}

#[allow(non_snake_case)]
fn TestComponent_1_1_0(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "2: double-nest" }
    })
}

#[allow(non_snake_case)]
fn RootFallback(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "Root Fallback" }
    })
}

#[allow(non_snake_case)]
fn NestedFallback(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "Nested Fallback" }
    })
}

mod components {
    mod go_back_button;
    mod go_forward_button;
    mod link;
    mod outlet;
    mod router;
}

mod usage {
    mod fallback;
    mod fallback_external_navigation;
    mod fallback_named_navigation;
    mod sitemap;
}

mod hooks {
    mod use_navigate;
    mod use_route;
}
