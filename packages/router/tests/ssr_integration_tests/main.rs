use dioxus::prelude::*;
use std::sync::Arc;

const ADDRESS: &str = "https://dioxuslabs.com/";

fn render(component: Component) -> String {
    let mut app = VirtualDom::new(component);
    app.rebuild();
    dioxus::ssr::render_vdom(&app)
}

fn test_routes(cx: &ScopeState) -> Arc<Segment> {
    use_segment(&cx, || {
        Segment::new().index(RcComponent(TestComponent_0)).fixed(
            "test",
            Route::new(RcMulti(
                TestComponent_1,
                vec![("other", TestComponent_1_0_other)],
            ))
            .name("test")
            .nested(
                Segment::new().index(RcComponent(TestComponent_1_0)).fixed(
                    "nest",
                    Route::new(RcComponent(TestComponent_1_1)).nested(
                        Segment::new()
                            .fixed("double-nest", Route::new(RcComponent(TestComponent_1_1_0))),
                    ),
                ),
            ),
        )
    })
    .clone()
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

mod components {
    mod go_back_button;
    mod go_forward_button;
    mod link;
    mod outlet;
    mod router;
}

mod hooks {
    mod use_navigate;
    mod use_route;
}
