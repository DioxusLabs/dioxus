//! This example shows how to listen to all events on a div and log them to the console.
//!
//! The primary demonstration here is the properties on the events themselves, hoping to give you some inspiration
//! on adding interactivity to your own application.

use dioxus::prelude::*;
use std::{collections::VecDeque, fmt::Debug, rc::Rc};

fn main() {
    launch(app);
}

fn app() -> Element {
    // Using a VecDeque so its cheap to pop old events off the front
    let mut events = use_signal(VecDeque::new);

    // All events and their data implement Debug, so we can re-cast them as Rc<dyn Debug> instead of their specific type
    let mut log_event = move |event: Rc<dyn Debug>| {
        // Only store the last 20 events
        if events.read().len() >= 20 {
            events.write().pop_front();
        }
        events.write().push_back(event);
    };

    rsx! {
        style { {include_str!("./assets/events.css")} }
        div { id: "container",
            // focusing is necessary to catch keyboard events
            div { id: "receiver", tabindex: 0,
                onmousemove: move |event| log_event(event.data()),
                onclick: move |event| log_event(event.data()),
                ondoubleclick: move |event| log_event(event.data()),
                onmousedown: move |event| log_event(event.data()),
                onmouseup: move |event| log_event(event.data()),

                onwheel: move |event| log_event(event.data()),

                onkeydown: move |event| log_event(event.data()),
                onkeyup: move |event| log_event(event.data()),
                onkeypress: move |event| log_event(event.data()),

                onfocusin: move |event| log_event(event.data()),
                onfocusout: move |event| log_event(event.data()),

                "Hover, click, type or scroll to see the info down below"
            }
            div { id: "log",
                for event in events.read().iter() {
                    div { "{event:?}" }
                }
            }
        }
    }
}
