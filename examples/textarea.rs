// How to use textareas

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let model = use_state(cx, || String::from("asd"));

    println!("{model}");

    cx.render(rsx! {
        textarea {
            class: "border",
            rows: "10",
            cols: "80",
            value: "{model}",
            oninput: move |e| model.set(e.value().clone()),
        }
    })
}
