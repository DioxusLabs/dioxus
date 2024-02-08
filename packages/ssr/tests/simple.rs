#![allow(non_snake_case)]

use dioxus::prelude::*;

#[test]
fn simple() {
    fn App() -> Element {
        rsx! { div { "hello!" } }
    }

    let mut dom = VirtualDom::new(App);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(dioxus_ssr::render(&dom), "<div>hello!</div>");

    assert_eq!(
        dioxus_ssr::render_element(rsx!( div {"hello!"} )),
        "<div>hello!</div>"
    );
}

#[test]
fn lists() {
    assert_eq!(
        dioxus_ssr::render_element(rsx! {
            ul {
                {
                    (0..5).map(|i| rsx! {
                        li { "item {i}" }
                    })
                }
            }
        }),
        "<ul><li>item 0</li><li>item 1</li><li>item 2</li><li>item 3</li><li>item 4</li></ul>"
    );
}

#[test]
fn dynamic() {
    let dynamic = 123;
    assert_eq!(
        dioxus_ssr::render_element(rsx! {
            div { "Hello world 1 -->" "{dynamic}" "<-- Hello world 2" }
        }),
        "<div>Hello world 1 --&gt;123&lt;-- Hello world 2</div>"
    );
}

#[test]
fn components() {
    #[derive(Props, Clone, PartialEq)]
    struct MyComponentProps {
        name: i32,
    }

    fn MyComponent(MyComponentProps { name }: MyComponentProps) -> Element {
        rsx! { div { "component {name}" } }
    }

    assert_eq!(
        dioxus_ssr::render_element(rsx! {
            div {
                {
                    (0..5).map(|name| rsx! {
                        MyComponent { name: name }
                    })
                }
            }
        }),
        "<div><div>component 0</div><div>component 1</div><div>component 2</div><div>component 3</div><div>component 4</div></div>"
    );
}

#[test]
fn fragments() {
    assert_eq!(
        dioxus_ssr::render_element(rsx! {
            div {
                {
                    (0..5).map(|_| rsx! ({}))
                }
            }
        }),
        "<div></div>"
    );
}
