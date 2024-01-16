use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    render! {
        div { "Hello, world!" }
    }
}
