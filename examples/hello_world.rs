use dioxus::prelude::*;
use dioxus_desktop::tao::menu::MenuBar;

fn main() {
    dioxus::desktop::launch_cfg(app, |c| c.with_window(|w| w.with_menu(MenuBar::default())));
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! (
        div { "Hello, world!" }
    ))
}
