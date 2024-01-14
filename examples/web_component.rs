use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    cx.render(rsx! {
        web-component {
            "my-prop": "5%",
        }
    })
}
