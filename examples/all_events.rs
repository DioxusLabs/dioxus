use dioxus::prelude::*;
use dioxus_core::UiEvent;
use dioxus_html::on::MouseData;
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
        .map(|event| cx.render(rsx!(div{"{event:?}"})));

    let log_event = move |event: Event| {
        events.write().push(event);
    };

    cx.render(rsx! (
        div {
            style: "{container_style}",
            "Hover over to display coordinates:",
            div {
                style: "{rect_style}",
                onmousemove: move |event| log_event(Event::MouseMove(event.data)),
                onclick: move |event| log_event(Event::MouseClick(event.data)),
                ondblclick: move |event| log_event(Event::MouseDoubleClick(event.data)),
                onmousedown: move |event| log_event(Event::MouseDown(event.data)),
                onmouseup: move |event| log_event(Event::MouseUp(event.data)),
                prevent_default: "mousedown",
            }
        },
        events_rendered,
    ))
}
