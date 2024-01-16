use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    rsx! {
        div { "Hello, world!" }
    }
}
