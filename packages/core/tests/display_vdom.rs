//! test that we can display the virtualdom properly
//!
//!
//!

use std::{cell::RefCell, rc::Rc};

use anyhow::{Context, Result};
use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
mod test_logging;

const IS_LOGGING_ENABLED: bool = true;

#[test]
fn please_work() {
    static App: FC<()> = |cx, props| {
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

    static Child: FC<()> = |cx, props| {
        cx.render(rsx! {
            div { "child" }
        })
    };

    let mut dom = VirtualDom::new(App);
    dom.rebuild();

    println!("{}", dom);
}
