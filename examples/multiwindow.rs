//! Multiwindow with tray icon example
//!
//! This example shows how to implement a simple multiwindow application using dioxus.
//! This works by spawning a new window when the user clicks a button. We have to build a new virtualdom which has its
//! own context, root elements, etc.

use dioxus::desktop::trayicon::*;
use dioxus::desktop::*;
use dioxus::prelude::*;
fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(Config::new().with_close_behaviour(WindowCloseBehaviour::LastWindowHides))
        .launch(app);
}

fn app() -> Element {
    // new window needs to be in async, otherwise it freezes
    let onclick = move |_| async move {
        let dom = VirtualDom::new(popup);
        dioxus::desktop::window().new_window(dom, Default::default());
    };
    init_tray_icon(default_tray_icon(), None);
    rsx! {
        //"{TRAY.peek().id().0}"
        button { onclick, "New Window" }
    }
}

fn popup() -> Element {
    rsx! {
        div {
            //"{TRAY.peek().id().0}"
            "This is a popup window!" }
    }
}
