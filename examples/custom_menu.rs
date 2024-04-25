//! This example shows how to use a custom menu bar with Dioxus desktop.
//! This example is not supported on the mobile or web renderers.

use dioxus::desktop::{muda::*, use_muda_event_handler};
use dioxus::prelude::*;

fn main() {
    // Create a menu bar that only contains the edit menu
    let menu = Menu::new();
    let edit_menu = Submenu::new("Edit", true);

    edit_menu
        .append_items(&[
            &PredefinedMenuItem::undo(None),
            &PredefinedMenuItem::redo(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::cut(None),
            &PredefinedMenuItem::copy(None),
            &PredefinedMenuItem::paste(None),
            &PredefinedMenuItem::select_all(None),
            &MenuItem::with_id("switch-text", "Switch text", true, None),
        ])
        .unwrap();

    menu.append(&edit_menu).unwrap();

    // Create a desktop config that overrides the default menu with the custom menu
    let config = dioxus::desktop::Config::new().with_menu(menu);

    // Launch the app with the custom menu
    LaunchBuilder::new().with_cfg(config).launch(app)
}

fn app() -> Element {
    let mut text = use_signal(String::new);
    // You can use the `use_muda_event_handler` hook to run code when a menu event is triggered.
    use_muda_event_handler(move |muda_event| {
        if muda_event.id() == "switch-text" {
            text.set("Switched to text".to_string());
        }
    });

    rsx! {
        div {
            h1 { "Custom Menu" }
            p { "Text: {text}" }
        }
    }
}
