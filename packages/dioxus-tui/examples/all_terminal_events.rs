use dioxus::prelude::*;
use dioxus_html::{FocusData, KeyboardData, MouseData, WheelData};
use std::rc::Rc;

fn main() {
    dioxus_tui::launch(app);
}

#[derive(Debug)]
enum Event {
    MouseMove(Rc<MouseData>),
    MouseClick(Rc<MouseData>),
    MouseDoubleClick(Rc<MouseData>),
    MouseDown(Rc<MouseData>),
    MouseUp(Rc<MouseData>),

    Wheel(Rc<WheelData>),

    KeyDown(Rc<KeyboardData>),
    KeyUp(Rc<KeyboardData>),
    KeyPress(Rc<KeyboardData>),

    FocusIn(Rc<FocusData>),
    FocusOut(Rc<FocusData>),
}

const MAX_EVENTS: usize = 8;

fn app(cx: Scope) -> Element {
    let events = use_ref(cx, Vec::new);

    let events_lock = events.read();
    let first_index = events_lock.len().saturating_sub(MAX_EVENTS);
    let events_rendered = events_lock[first_index..].iter().map(|event| {
        // TUI panics if text overflows (https://github.com/DioxusLabs/dioxus/issues/371)
        // temporary hack: just trim the strings (and make sure viewport is big enough)
        // todo: remove
        let mut trimmed = format!("{event:?}");
        trimmed.truncate(200);
        rsx!( p { "{trimmed}" } )
    });

    let log_event = move |event: Event| {
        events.write().push(event);
    };

    cx.render(rsx! {
        div { width: "100%", height: "100%", flex_direction: "column",
            div {
                width: "80%",
                height: "50%",
                border_width: "1px",
                justify_content: "center",
                align_items: "center",
                background_color: "hsl(248, 53%, 58%)",

                onmousemove: move |event| log_event(Event::MouseMove(event.inner().clone())),
                onclick: move |event| log_event(Event::MouseClick(event.inner().clone())),
                ondoubleclick: move |event| log_event(Event::MouseDoubleClick(event.inner().clone())),
                onmousedown: move |event| log_event(Event::MouseDown(event.inner().clone())),
                onmouseup: move |event| log_event(Event::MouseUp(event.inner().clone())),

                onwheel: move |event| log_event(Event::Wheel(event.inner().clone())),

                onkeydown: move |event| log_event(Event::KeyDown(event.inner().clone())),
                onkeyup: move |event| log_event(Event::KeyUp(event.inner().clone())),
                onkeypress: move |event| log_event(Event::KeyPress(event.inner().clone())),

                onfocusin: move |event| log_event(Event::FocusIn(event.inner().clone())),
                onfocusout: move |event| log_event(Event::FocusOut(event.inner().clone())),

                "Hover, click, type or scroll to see the info down below"
            }
            div { width: "80%", height: "50%", flex_direction: "column", {events_rendered} }
        }
    })
}
