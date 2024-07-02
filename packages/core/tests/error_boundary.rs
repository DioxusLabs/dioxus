#![allow(non_snake_case)]

use dioxus::prelude::*;

#[test]
fn catches_panic() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);
}

fn app() -> Element {
    rsx! {
        div {
            h1 { "Title" }

            NoneChild {}
            ThrowChild {}
        }
    }
}

fn NoneChild() -> Element {
    VNode::empty()
}

fn ThrowChild() -> Element {
    Err(std::io::Error::new(std::io::ErrorKind::AddrInUse, "asd"))?;

    let _g: i32 = "123123".parse()?;

    rsx! { div {} }
}
