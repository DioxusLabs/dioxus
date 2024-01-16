use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    rsx! {
        web-component {
            "my-prop": "5%",
        }
    }
}
