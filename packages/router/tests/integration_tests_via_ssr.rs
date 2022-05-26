use dioxus::prelude::*;
use std::sync::Arc;

const ADDRESS: &str = "https://dioxuslabs.com/";

fn render(component: Component) -> String {
    let mut app = VirtualDom::new(component);
    app.rebuild();
    dioxus::ssr::render_vdom(&app)
}

fn test_routes(cx: &Scope) -> Arc<Segment> {
    use_segment(&cx, || {
        Segment::new().index(RcComponent(TestComponent1)).fixed(
            "test",
            Route::new(RcMulti(TestComponent2, vec![("other", TestComponent5)]))
                .name("test")
                .nested(
                    Segment::new()
                        .index(RcComponent(TestComponent3))
                        .fixed("nest", Route::new(RcComponent(TestComponent4))),
                ),
        )
    })
    .clone()
}

#[allow(non_snake_case)]
fn TestComponent1(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "test1" }
    })
}

#[allow(non_snake_case)]
fn TestComponent2(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "test2" }
        Outlet { }
    })
}

#[allow(non_snake_case)]
fn TestComponent3(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "test3" }
    })
}

#[allow(non_snake_case)]
fn TestComponent4(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "test4" }
    })
}

#[allow(non_snake_case)]
fn TestComponent5(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "test5" }
    })
}

mod components {
    mod go_back_button;
    mod go_forward_button;
    mod link;
    mod outlet;
    mod router;
}
