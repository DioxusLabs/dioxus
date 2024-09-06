use dioxus::prelude::*;

fn main() {
    println!("Launching app...");

    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        div { "Hello, world!" }
    }
}
