// How to use textareas

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    let model = use_signal(|| String::from("asd"));

    println!("{model}");

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
