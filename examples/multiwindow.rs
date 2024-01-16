use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    rsx! {
        div {
            button {
                onclick: move |_| {
                    let dom = VirtualDom::new(popup);
                    dioxus_desktop::window().new_window(dom, Default::default());
                },
                "New Window"
            }
        }
    }
}

fn popup() -> Element {
    rsx! {
        div { "This is a popup!" }
    }
}
