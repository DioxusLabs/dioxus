//! Listen for window focus events using a wry event handler
//!
//! This example shows how to use the use_wry_event_handler hook to listen for window focus events.
//! We can intercept any Wry event, but in this case we're only interested in the WindowEvent::Focused event.
//!
//! This lets you do things like backgrounding tasks, pausing animations, or changing the UI when the window is focused or not.

use dioxus::desktop::tao::event::Event as WryEvent;
use dioxus::desktop::tao::event::WindowEvent;
use dioxus::desktop::use_wry_event_handler;
use dioxus::desktop::{Config, WindowCloseBehaviour};
use dioxus::prelude::*;

fn main() {
    LaunchBuilder::desktop()
        .with_cfg(Config::new().with_close_behaviour(WindowCloseBehaviour::CloseWindow))
        .launch(app)
}

fn app() -> Element {
    let mut focused = use_signal(|| true);

    use_wry_event_handler(move |event, _| {
        if let WryEvent::WindowEvent {
            event: WindowEvent::Focused(new_focused),
            ..
        } = event
        {
            focused.set(*new_focused)
        }
    });

    rsx! {
        div { width: "100%", height: "100%", display: "flex", flex_direction: "column", align_items: "center",
            if focused() {
                "This window is focused!"
            } else {
                "This window is not focused!"
            }
        }
    }
}
