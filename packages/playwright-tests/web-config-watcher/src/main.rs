use dioxus::prelude::*;

fn app() -> Element {
    let mut num = use_signal(|| 0);
    rsx! {
        div { id: "main",
            button {
                id: "increment-button",
                onclick: move |_| { num += 1; },
                "Count: {num}"
            }
        }
    }
}

fn main() {
    dioxus::launch(app);
}
