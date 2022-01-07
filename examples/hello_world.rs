use dioxus::prelude::*;
use dioxus_desktop::tao::menu::MenuBar;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! (
        div { "Hello, world!" }
    ))
}
