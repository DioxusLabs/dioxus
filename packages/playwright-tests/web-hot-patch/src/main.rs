use dioxus::prelude::*;

const CSS: Asset = asset!("/assets/style.css");
const IMAGE: Asset = asset!("/assets/toasts.png");

fn app() -> Element {
    let mut num = use_signal(|| 0);

    rsx! {
        document::Link {
            href: CSS,
            rel: "stylesheet",
        }
        img {
            id: "toasts",
            src: IMAGE,
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
