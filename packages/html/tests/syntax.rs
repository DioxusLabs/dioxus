use dioxus_core::prelude::*;
use dioxus_html::builder::*;

#[test]
fn test_builder() {
    #[allow(unused)]
    fn please(cx: Scope) -> Element {
        div(&cx)
            .class("a")
            .draggable(false)
            .id("asd")
            .accesskey(false)
            .class(false)
            .contenteditable(false)
            .data(false)
            .dir(false)
            .dangerous_inner_html(false)
            .attr("name", "asd")
            .onclick(move |_| println!("clicked"))
            .onclick(move |evt| println!("clicked"))
            .onclick(move |_| println!("clicked"))
            .children([
                match true {
                    true => div(&cx),
                    false => div(&cx).class("asd"),
                },
                match 10 {
                    10 => div(&cx),
                    _ => div(&cx).class("asd"),
                },
            ])
            .children([
                match true {
                    true => div(&cx),
                    false => div(&cx).class("asd"),
                },
                match 10 {
                    10 => div(&cx),
                    _ => div(&cx).class("asd"),
                },
            ])
            .fragment((0..10).map(|i| {
                div(&cx)
                    .class("val")
                    .class(format_args!("{}", i))
                    .class("val")
                    .class("val")
                    .class("val")
                    .class("val")
                    .class("val")
                    .class("val")
                    .class("val")
            }))
            .fragment((0..10).map(|_i| div(&cx).class("val")))
            .fragment((0..20).map(|_i| div(&cx).class("val")))
            .fragment((0..30).map(|_i| div(&cx).class("val")))
            .fragment((0..40).map(|_i| div(&cx).class("val")))
            .children([
                match true {
                    true => div(&cx),
                    false => div(&cx).class("asd"),
                },
                match 10 {
                    10 => div(&cx),
                    _ => div(&cx).class("asd"),
                },
                if 20 == 10 {
                    div(&cx)
                } else {
                    div(&cx).class("asd")
                },
            ])
            .render()
    }
}
