use dioxus::{events::*, html::MouseEvent, prelude::*};

fn main() {
    dioxus_desktop::launch(app);
}

#[derive(Debug)]
enum Event {
    MouseMove(MouseEvent),
    MouseClick(MouseEvent),
    MouseDoubleClick(MouseEvent),
    MouseDown(MouseEvent),
    MouseUp(MouseEvent),

    Wheel(WheelEvent),

    KeyDown(KeyboardEvent),
    KeyUp(KeyboardEvent),
    KeyPress(KeyboardEvent),

    FocusIn(FocusEvent),
    FocusOut(FocusEvent),
}

const MAX_EVENTS: usize = 8;

const CONTAINER_STYLE: &str = r#"
        display: flex;
        flex-direction: column;
        align-items: center;
    "#;

const RECT_STYLE: &str = r#"
        background: deepskyblue;
        height: 50vh;
        width: 50vw;
        color: white;
        padding: 20px;
        margin: 20px;
        text-aligh: center;
    "#;

fn app(cx: Scope) -> Element {
    let events = use_ref(cx, std::collections::VecDeque::new);

    let log_event = move |event: Event| {
        let mut events = events.write();

        if events.len() >= MAX_EVENTS {
            events.pop_front();
        }
        events.push_back(event);
    };

    cx.render(rsx! (
        div { style: "{CONTAINER_STYLE}",
            div {
                style: "{RECT_STYLE}",
                // focusing is necessary to catch keyboard events
                tabindex: "0",

                onmousemove: move |event| log_event(Event::MouseMove(event)),
                onclick: move |event| log_event(Event::MouseClick(event)),
                ondoubleclick: move |event| log_event(Event::MouseDoubleClick(event)),
                onmousedown: move |event| log_event(Event::MouseDown(event)),
                onmouseup: move |event| log_event(Event::MouseUp(event)),

                onwheel: move |event| log_event(Event::Wheel(event)),

                onkeydown: move |event| log_event(Event::KeyDown(event)),
                onkeyup: move |event| log_event(Event::KeyUp(event)),
                onkeypress: move |event| log_event(Event::KeyPress(event)),

                onfocusin: move |event| log_event(Event::FocusIn(event)),
                onfocusout: move |event| log_event(Event::FocusOut(event)),

                "Hover, click, type or scroll to see the info down below"
            }
            div {
                for event in events.read().iter() {
                    div { "{event:?}" }
                }
            }
        }
    ))
}
