use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    render! {
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
    render! {
        div { "This is a popup!" }
    }
}
