// How to use textareas

use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut model = use_signal(|| String::from("asd"));

    rsx! {
        textarea {
            class: "border",
            rows: "10",
            cols: "80",
            value: "{model}",
            oninput: move |e| model.set(e.value().clone()),
        }
    }
}
