//! Multiwindow with tray icon example
//!
//! This example shows how to implement a simple multiwindow application and tray icon using dioxus.
//! This works by spawning a new window when the user clicks a button. We have to build a new virtualdom which has its
//! own context, root elements, etc.
//!
//! This is useful for apps that incorporate settings panels or persistent windows like Raycast.

use dioxus::desktop::{
    WindowCloseBehaviour,
    trayicon::{default_tray_icon, init_tray_icon},
    window,
};
use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    use_hook(|| {
        // Set the close behavior for the main window
        // This will hide the window instead of closing it when the user clicks the close button
        window().set_close_behavior(WindowCloseBehaviour::WindowHides);

        // Initialize the tray icon with a default icon and no menu
        // This will provide the tray into context for the application
        init_tray_icon(default_tray_icon(), None)
    });

    rsx! {
        button {
            onclick: move |_| {
                window().new_window(VirtualDom::new(popup), Default::default());
            },
            "New Window"
        }
    }
}

fn popup() -> Element {
    rsx! {
        div { "This is a popup window!" }
    }
}
