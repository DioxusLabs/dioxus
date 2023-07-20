use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            p {
                "This should show an image:"
            }
            img { src: "examples/assets/logo.png" }
        }
    })
}
