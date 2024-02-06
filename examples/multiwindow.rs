use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let onclick = move |_| {
        let dom = VirtualDom::new(popup);
        dioxus::desktop::window().new_window(dom, Default::default());
    };

    rsx! {
        button { onclick, "New Window" }
    }
}

fn popup() -> Element {
    rsx! {
        div { "This is a popup window!" }
    }
}
