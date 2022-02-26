use dioxus_core::prelude::*;
use dioxus_html::builder::*;

#[test]
fn test_builder() {
    #[allow(unused)]
    fn please(cx: Scope, val: i32) -> Element {
        div(&cx)
            .background_color("red")
            .background_attachment("red")
            .background("red")
            .background("red")
            .aria_errormessage("asd")
            .hidden(true)
            .background("red")
            .children([
                div(&cx).background("red"),
                div(&cx).background("red"),
                div(&cx).background("red"),
                div(&cx).background("red"),
                if val == 20 {
                    div(&cx).background("red")
                } else {
                    div(&cx).background("black")
                },
            ])
            .build()
    }
}
