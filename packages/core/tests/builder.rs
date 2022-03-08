#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

#[test]
fn test_borrowed_state() {
    let mut dom = VirtualDom::new(Parent);
    dom.rebuild();

    dbg!(dom.base_scope().root_node());
}

fn Parent(cx: Scope) -> Element {
    use dioxus_elements::builder::*;

    let value = cx.use_hook(|_| String::new());

    div(&cx)
        .child(div(&cx).class("val").class("val").class("val"))
        .child(if true { div(&cx) } else { h2(&cx) })
        .children([
            h2(&cx)
                .class("val")
                .id("asd")
                .name("asd")
                .onclick(move |_| {
                    //
                }),
            h3(&cx)
                .class("val")
                .id("asd")
                .name("asd")
                .onclick(move |_| {
                    //
                }),
            h3(&cx)
                .class("val")
                .id("asd")
                .name("asd")
                .onclick(move |_| {
                    //
                }),
            h3(&cx)
                .class("val")
                .id("asd")
                .name("asd")
                .onclick(move |_| {
                    //
                }),
        ])
        .build()
}
// cx.render(rsx! {
//     div {
//         Child { name: value }
//         Child { name: value }
//         Child { name: value }
//         Child { name: value }
//     }
// })

// #[derive(Props)]
// struct ChildProps<'a> {
//     name: &'a str,
// }

// fn Child<'a>(cx: Scope<'a, ChildProps<'a>>) -> Element {
//     cx.render(rsx! {
//         div {
//             h1 { "it's nested" }
//             Child2 { name: cx.props.name }
//         }
//     })
// }

// #[derive(Props)]
// struct Grandchild<'a> {
//     name: &'a str,
// }

// fn Child2<'a>(cx: Scope<'a, Grandchild<'a>>) -> Element {
//     cx.render(rsx! {
//         div { "Hello {cx.props.name}!" }
//     })
// }
