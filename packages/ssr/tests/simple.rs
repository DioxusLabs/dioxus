use dioxus::prelude::*;

#[test]
fn simple() {
    fn app(cx: Scope) -> Element {
        render! { div { "hello!" } }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild();

    assert_eq!(
        dioxus_ssr::SsrRender::default().render_vdom(&dom),
        "<div>hello!</div>"
    );
}

#[test]
fn lists() {
    fn app(cx: Scope) -> Element {
        render! {
            ul {
                (0..5).map(|i| rsx! {
                    li { "item {i}" }
                })
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild();

    assert_eq!(
        dioxus_ssr::SsrRender::default().render_vdom(&dom),
        "<ul><li>item 0</li><li>item 1</li><li>item 2</li><li>item 3</li><li>item 4</li></ul>"
    );
}

#[test]
fn dynamic() {
    fn app(cx: Scope) -> Element {
        let dynamic = 123;

        render! {
            div { "Hello world 1 -->" "{dynamic}" "<-- Hello world 2" }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild();

    assert_eq!(
        dioxus_ssr::SsrRender::default().render_vdom(&dom),
        "<div>Hello world 1 -->123<-- Hello world 2</div>"
    );
}

#[test]
fn components() {
    fn app(cx: Scope) -> Element {
        render! {
            div {
                (0..5).map(|name| rsx! {
                    my_component { name: name }
                })
            }
        }
    }

    #[inline_props]
    fn my_component(cx: Scope, name: i32) -> Element {
        render! {
            div { "component {name}" }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild();

    assert_eq!(
        dioxus_ssr::SsrRender::default().render_vdom(&dom),
        "<div><div>component 0</div><div>component 1</div><div>component 2</div><div>component 3</div><div>component 4</div></div>"
    );
}
