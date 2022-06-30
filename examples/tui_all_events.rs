use dioxus::prelude::*;
use dioxus_html::on::{FocusData, KeyboardData, MouseData, WheelData};
use std::sync::Arc;

fn main() {
    dioxus::tui::launch(app);
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
    let events = use_ref(&cx, || Vec::new());

    let events_lock = events.read();
    let first_index = events_lock.len().saturating_sub(MAX_EVENTS);
    let events_rendered = events_lock[first_index..].iter().map(|event| {
        // TUI panics if text overflows (https://github.com/DioxusLabs/dioxus/issues/371)
        // temporary hack: just trim the strings (and make sure viewport is big enough)
        // todo: remove
        let mut trimmed = format!("{event:?}");
        trimmed.truncate(200);
        cx.render(rsx!(p { "{trimmed}" }))
    });

    let log_event = move |event: Event| {
        events.write().push(event);
    };

    cx.render(rsx! {
        div {
            width: "100%",
            height: "100%",
            flex_direction: "column",
            div {
                width: "80%",
                height: "50%",
                border_width: "1px",
                justify_content: "center",
                align_items: "center",
                background_color: "hsl(248, 53%, 58%)",

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
            },
            div {
                width: "80%",
                height: "50%",
                flex_direction: "column",
                events_rendered,
            },
        },
    })
}
