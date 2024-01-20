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
    let mut focused = use_signal(|| false);

    use_wry_event_handler(move |event, _| match event {
        WryEvent::WindowEvent {
            event: WindowEvent::Focused(new_focused),
            ..
        } => focused.set(*new_focused),
        _ => {}
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
