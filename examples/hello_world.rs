use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    render! {
        div { "Hello, world!" }
    }
}
