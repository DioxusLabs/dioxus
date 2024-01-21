use std::{collections::VecDeque, fmt::Debug, rc::Rc};

use dioxus::{events::*, html::MouseEvent, prelude::*};

fn main() {
    launch(app);
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

fn app() -> Element {
    let mut events = use_signal(|| VecDeque::new() as VecDeque<Rc<dyn Debug>>);

    let mut log_event = move |event: Rc<dyn Debug>| {
        let mut events = events.write();

        if events.len() >= MAX_EVENTS {
            events.pop_front();
        }

        events.push_back(event);
    };

    rsx! {
        div { style: "{CONTAINER_STYLE}",
            // focusing is necessary to catch keyboard events
            div { style: "{RECT_STYLE}", tabindex: "0",
                onmousemove: move |event| log_event(event.inner().clone()),
                onclick: move |event| log_event(event.inner().clone()),
                ondoubleclick: move |event| log_event(event.inner().clone()),
                onmousedown: move |event| log_event(event.inner().clone()),
                onmouseup: move |event| log_event(event.inner().clone()),

                onwheel: move |event| log_event(event.inner().clone()),

                onkeydown: move |event| log_event(event.inner().clone()),
                onkeyup: move |event| log_event(event.inner().clone()),
                onkeypress: move |event| log_event(event.inner().clone()),

                onfocusin: move |event| log_event(event.inner().clone()),
                onfocusout: move |event| log_event(event.inner().clone()),

                "Hover, click, type or scroll to see the info down below"
            }
            div {
                for event in events.read().iter() {
                    div { "{event:?}" }
                }
            }
        }
    }
}
