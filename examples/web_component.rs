use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    rsx! {
        web-component {
            "my-prop": "5%",
        }
    }
}
