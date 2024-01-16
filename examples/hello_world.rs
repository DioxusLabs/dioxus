use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    render! {
        div { "Hello, world!" }
    }
}
