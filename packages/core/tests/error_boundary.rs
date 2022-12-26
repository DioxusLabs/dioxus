use dioxus::prelude::*;
use futures_util::Future;

#[test]
fn catches_panic() {
    let mut dom = VirtualDom::new(app);

    let a = dom.rebuild();

    dbg!(a);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            h1 { "Title" }

            NoneChild {}
        }
    })
}

fn NoneChild(cx: Scope) -> Element {
    None
}

fn PanicChild(cx: Scope) -> Element {
    panic!("Rendering panicked for whatever reason");

    cx.render(rsx! {
        h1 { "It works!" }
    })
}

fn ThrowChild(cx: Scope) -> Element {
    cx.throw(std::io::Error::new(std::io::ErrorKind::AddrInUse, "asd"))?;

    let g: i32 = "123123".parse().throw(cx)?;

    cx.render(rsx! {
        div {}
    })
}

fn custom_allocator(cx: Scope) -> Element {
    let g = String::new();

    let p = g.as_str();

    let g2 = cx.use_hook(|| 123);
    // cx.spawn(async move {

    //     //
    //     // println!("Thig is {p}");
    // });

    None
}
