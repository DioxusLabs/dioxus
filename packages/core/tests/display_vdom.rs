#![allow(unused, non_upper_case_globals)]

//! test that we can display the virtualdom properly
//!
//!
//!

use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
mod test_logging;

#[test]
fn please_work() {
    static App: FC<()> = |(cx, props)| {
        cx.render(rsx! {
            div {
                hidden: "true"
                "hello"
                div { "hello" }
                Child {}
                Child {}
                Child {}
            }
            div { "hello" }
        })
    };

    static Child: FC<()> = |(cx, props)| {
        cx.render(rsx! {
            div { "child" }
        })
    };

    let mut dom = VirtualDom::new(App);
    dom.rebuild();

    println!("{}", dom);
}
