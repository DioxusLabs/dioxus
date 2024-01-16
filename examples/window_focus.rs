use dioxus::prelude::*;
use dioxus_desktop::tao::event::Event as WryEvent;
use dioxus_desktop::tao::event::WindowEvent;
use dioxus_desktop::use_wry_event_handler;
use dioxus_desktop::{Config, WindowCloseBehaviour};

fn main() {
    Config::new()
        .with_close_behaviour(WindowCloseBehaviour::CloseWindow)
        .launch(app)
}

fn app() -> Element {
    let focused = use_signal(|| false);

    use_wry_event_handler(move |event, _| match event {
        WryEvent::WindowEvent {
            event: WindowEvent::Focused(new_focused),
            ..
        } => focused.set(*new_focused),
        _ => {}
    });

    rsx! {
        div {
            width: "100%",
            height: "100%",
            display: "flex",
            flex_direction: "column",
            align_items: "center",
            if focused() {
                "This window is focused!"
            } else {
                "This window is not focused!"
            }
        }
    }
}
