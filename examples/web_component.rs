use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    render! {
        web-component {
            "my-prop": "5%",
        }
    }
}
