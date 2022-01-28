use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            "This should show an image:"
            img { src: "examples/assets/logo.png", }
        }
    })
}
