use dioxus::prelude::*;

#[test]
fn simple() {
    fn app(cx: Scope) -> Element {
        render! { div { "hello!" } }
    }

    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild();

    assert_eq!(dioxus_ssr::render(&dom), "<div>hello!</div>");

    assert_eq!(
        dioxus_ssr::render_lazy(rsx!( div {"hello!"} )),
        "<div>hello!</div>"
    );
}

#[test]
fn lists() {
    assert_eq!(
        dioxus_ssr::render_lazy(rsx! {
            ul {
                (0..5).map(|i| rsx! {
                    li { "item {i}" }
                })
            }
        }),
        "<ul><li>item 0</li><li>item 1</li><li>item 2</li><li>item 3</li><li>item 4</li></ul>"
    );
}

#[test]
fn dynamic() {
    let dynamic = 123;
    assert_eq!(
        dioxus_ssr::render_lazy(rsx! {
            div { "Hello world 1 -->" "{dynamic}" "<-- Hello world 2" }
        }),
        "<div>Hello world 1 --&gt;123&lt;-- Hello world 2</div>"
    );
}

#[test]
fn components() {
    #[inline_props]
    fn my_component(cx: Scope, name: i32) -> Element {
        render! {
            div { "component {name}" }
        }
    }

    assert_eq!(
        dioxus_ssr::render_lazy(rsx! {
            div {
                (0..5).map(|name| rsx! {
                    my_component { name: name }
                })
            }
        }),
        "<div><div>component 0</div><div>component 1</div><div>component 2</div><div>component 3</div><div>component 4</div></div>"
    );
}

#[test]
fn fragments() {
    assert_eq!(
        dioxus_ssr::render_lazy(rsx! {
            div {
                (0..5).map(|_| rsx! (()))
            }
        }),
        "<div></div>"
    );
}
