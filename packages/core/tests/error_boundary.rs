#![allow(non_snake_case)]

use dioxus::prelude::*;

#[test]
fn catches_panic() {
    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild_to_vec(&mut dioxus_core::NoOpMutations);
}

fn app() -> Element {
    render! {
        div {
            h1 { "Title" }

            NoneChild {}
            ThrowChild {}
        }
    }
}

fn NoneChild(_cx: Scope) -> Element {
    None
}

fn ThrowChild(cx: Scope) -> Element {
    Err(std::io::Error::new(std::io::ErrorKind::AddrInUse, "asd")).throw()?;

    let _g: i32 = "123123".parse().throw()?;

    render! { div {} }
}
