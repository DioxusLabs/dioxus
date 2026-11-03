//! Multiwindow with tray icon example
//!
//! This example shows how to implement a simple multiwindow application and tray icon using dioxus.
//! This works by rendering a `Window` component when the user clicks a button.
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
    let mut windows = use_signal(Vec::<usize>::new);
    let mut next_id = use_signal(|| 0usize);

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
                let id = next_id();
                next_id.set(id + 1);
                windows.write().push(id);
            },
            "New Window"
        }
        for id in windows() {
            Window {
                key: "{id}",
                onclose: move |_| windows.write().retain(|window_id| *window_id != id),
                Popup {}
            }
        }
    }
}

#[component]
fn Popup() -> Element {
    rsx! {
        div { "This is a popup window!" }
    }
}
