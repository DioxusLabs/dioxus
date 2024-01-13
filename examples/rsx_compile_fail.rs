//! This example just flexes the ability to use arbitrary expressions within RSX.
//! It also proves that lifetimes work properly, especially when used with use_ref

use dioxus::prelude::*;

fn main() {
    let mut vdom = VirtualDom::new(example);
    _ = vdom.rebuild();

    let mut renderer = dioxus_ssr::Renderer::new();
    renderer.pretty = true;
    renderer.render(&vdom);
}

fn example(cx: Scope) -> Element {
    let items = use_state(cx, || {
        vec![Thing {
            a: "asd".to_string(),
            b: 10,
        }]
    });

    let things = use_ref(cx, || {
        vec![Thing {
            a: "asd".to_string(),
            b: 10,
        }]
    });
    let things_list = things.read();

    let mything = use_ref(cx, || Some(String::from("asd")));
    let mything_read = mything.read();

    cx.render(rsx!(
        div {
            div { id: "asd",
                "your neighborhood spiderman"

                for item in items.iter().cycle().take(5) {
                    div { "{item.a}" }
                }

                for thing in things_list.iter() {
                    div { "{thing.a}" "{thing.b}" }
                }

                if let Some(f) = mything_read.as_ref() {
                    div { "{f}" }
                }
            }
        }
    ))
}

struct Thing {
    a: String,
    b: u32,
}
