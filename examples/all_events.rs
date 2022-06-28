use dioxus::prelude::*;
use dioxus_core::UiEvent;
use dioxus_html::on::{KeyboardData, MouseData};
use std::sync::Arc;

fn main() {
    dioxus::desktop::launch(app);
}

#[derive(Debug)]
enum Event {
    MouseMove(Arc<MouseData>),
    MouseClick(Arc<MouseData>),
    MouseDoubleClick(Arc<MouseData>),
    MouseDown(Arc<MouseData>),
    MouseUp(Arc<MouseData>),
    KeyDown(Arc<KeyboardData>),
    KeyUp(Arc<KeyboardData>),
    KeyPress(Arc<KeyboardData>),
}

const MAX_EVENTS: usize = 4;

fn app(cx: Scope) -> Element {
    let container_style = r#"
        display: flex;
        flex-direction: column;
        align-items: center;
    "#;
    let rect_style = r#"
        background: deepskyblue;
        height: 50vh;
        width: 50vw;
    "#;

    let events = use_ref(&cx, || Vec::new());

    let events_lock = events.read();
    let first_index = events_lock.len().saturating_sub(MAX_EVENTS);
    let events_rendered = events_lock[first_index..]
        .iter()
        .map(|event| cx.render(rsx!(div {"{event:?}"})));

    let log_event = move |event: Event| {
        events.write().push(event);
    };

    cx.render(rsx! (
        div {
            style: "{container_style}",
            div {
                style: "{rect_style}",
                // focusing is necessary to catch keyboard events
                tabindex: "0",

                onmousemove: move |event| log_event(Event::MouseMove(event.data)),
                onclick: move |event| log_event(Event::MouseClick(event.data)),
                ondblclick: move |event| log_event(Event::MouseDoubleClick(event.data)),
                onmousedown: move |event| log_event(Event::MouseDown(event.data)),
                onmouseup: move |event| log_event(Event::MouseUp(event.data)),

                onkeydown: move |event| log_event(Event::KeyDown(event.data)),
                onkeyup: move |event| log_event(Event::KeyUp(event.data)),
                onkeypress: move |event| log_event(Event::KeyPress(event.data)),
            }
            div { events_rendered },
        },
    ))
}
