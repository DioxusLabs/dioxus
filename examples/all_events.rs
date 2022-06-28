use dioxus::prelude::*;
use dioxus_html::on::{FocusData, KeyboardData, MouseData, WheelData};
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

    Wheel(Arc<WheelData>),

    KeyDown(Arc<KeyboardData>),
    KeyUp(Arc<KeyboardData>),
    KeyPress(Arc<KeyboardData>),

    FocusIn(Arc<FocusData>),
    FocusOut(Arc<FocusData>),
}

const MAX_EVENTS: usize = 8;

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
        color: white;
        padding: 20px;
        margin: 20px;
        text-aligh: center;
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

                onwheel: move |event| log_event(Event::Wheel(event.data)),

                onkeydown: move |event| log_event(Event::KeyDown(event.data)),
                onkeyup: move |event| log_event(Event::KeyUp(event.data)),
                onkeypress: move |event| log_event(Event::KeyPress(event.data)),

                onfocusin: move |event| log_event(Event::FocusIn(event.data)),
                onfocusout: move |event| log_event(Event::FocusOut(event.data)),

                "Hover, click, type or scroll to see the info down below"
            }
            div { events_rendered },
        },
    ))
}
