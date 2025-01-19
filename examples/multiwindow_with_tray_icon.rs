//! Multiwindow with tray icon example
//!
//! This example shows how to implement a simple multiwindow application and tray icon using dioxus.
//! This works by spawning a new window when the user clicks a button. We have to build a new virtualdom which has its
//! own context, root elements, etc.

use dioxus::desktop::{
    trayicon::{default_tray_icon, init_tray_icon},
    Config, WindowCloseBehaviour,
};
use dioxus::prelude::*;

fn main() {
    dioxus::LaunchBuilder::desktop()
        // We can choose the close behavior of this window to hide. See WindowCloseBehaviour for more options.
        .with_cfg(Config::new().with_window_close_behaviour(WindowCloseBehaviour::WindowHides))
        .launch(app);
}

fn app() -> Element {
    // async should not be needed, check if issue 3542 has been resolved
    let onclick = move |_| async {
        let dom = VirtualDom::new(popup);
        dioxus::desktop::window().new_window(dom, Default::default());
    };

    init_tray_icon(default_tray_icon(), None);

    rsx! {
        button { onclick, "New Window" }
    }
}

fn popup() -> Element {
    rsx! {
        div { "This is a popup window!" }
    }
}
