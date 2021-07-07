//! Example: README.md showcase
//!
//! The example from the README.md

use dioxus::prelude::*;

fn main() {
    dioxus::web::launch(Example)
}

fn Example(cx: Context<()>) -> VNode {
    let name = use_state(&cx, || "..?");

    cx.render(rsx! {
        h1 { "Hello, {name}" }
        button { "?", onclick: move |_| name.set("world!")}
        button { "?", onclick: move |_| name.set("Dioxus 🎉")}
    })
}

static Example2: FC<()> = |cx| {
    let (g, set_g) = use_state_classic(&cx, || 0);
    let v = (0..10).map(|f| {
        dioxus::prelude::LazyNodes::new(move |__cx: &NodeFactory| {
            __cx.element(dioxus_elements::li)
                .listeners([dioxus::events::on::onclick(__cx, move |_| set_g(10))])
                .finish()
        })
    });
    cx.render(dioxus::prelude::LazyNodes::new(
        move |__cx: &NodeFactory| {
            __cx.element(dioxus_elements::div)
                .children([__cx.fragment_from_iter(v)])
                .finish()
        },
    ))
};
