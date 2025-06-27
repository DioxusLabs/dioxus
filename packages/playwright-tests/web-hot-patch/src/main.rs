use dioxus::prelude::*;

const CSS: Asset = asset!("/assets/style.css");

fn app() -> Element {
    let mut num = use_signal(|| 0);

    rsx! {
        document::Link {
            href: CSS,
            rel: "stylesheet",
        }
        button {
            id: "increment-button",
            onclick: move |_| {
                num += 1;
            },
            "Click me! Count: {num}"
        }
    }
}

fn main() {
    dioxus::launch(app);
}
