#![allow(non_snake_case)]

use dioxus::prelude::*;

#[test]
fn catches_panic() {
    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild();
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            h1 { "Title" }

            NoneChild {}
            ThrowChild {}
        }
    })
}

fn NoneChild(_cx: Scope) -> Element {
    None
}

fn ThrowChild(cx: Scope) -> Element {
    Err(std::io::Error::new(std::io::ErrorKind::AddrInUse, "asd")).throw()?;

    let _g: i32 = "123123".parse().throw()?;

    cx.render(rsx! {
        div {}
    })
}
