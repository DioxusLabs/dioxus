use dioxus::prelude::*;
use dioxus_html::{FocusData, KeyboardData, MouseData, WheelData};
use std::{fmt::Debug, rc::Rc};

fn main() {
    dioxus_tui::launch(app);
}

const MAX_EVENTS: usize = 8;

fn app() -> Element {
    let mut events = use_signal(|| Vec::new() as Vec<Rc<dyn Debug>>);

    let mut log_event = move |event: Rc<dyn Debug>| events.write().push(event);

    rsx! {
        div { width: "100%", height: "100%", flex_direction: "column",
            div {
                width: "80%",
                height: "50%",
                border_width: "1px",
                justify_content: "center",
                align_items: "center",
                background_color: "hsl(248, 53%, 58%)",

                // Mosue
                onmousemove: move |event| log_event(event.inner().clone()),
                onclick: move |event| log_event(event.inner().clone()),
                ondoubleclick: move |event| log_event(event.inner().clone()),
                onmousedown: move |event| log_event(event.inner().clone()),
                onmouseup: move |event| log_event(event.inner().clone()),

                // Scroll
                onwheel: move |event| log_event(event.inner().clone()),

                // Keyboard
                onkeydown: move |event| log_event(event.inner().clone()),
                onkeyup: move |event| log_event(event.inner().clone()),
                onkeypress: move |event| log_event(event.inner().clone()),

                // Focus
                onfocusin: move |event| log_event(event.inner().clone()),
                onfocusout: move |event| log_event(event.inner().clone()),

                "Hover, click, type or scroll to see the info down below"
            }
            div { width: "80%", height: "50%", flex_direction: "column",
                // A trailing iterator of the last MAX_EVENTS events
                // The index actually is a fine key here, since events are append-only and therefore stable
                for (index, event) in events.read().iter().enumerate().rev().take(MAX_EVENTS).rev() {
                    p { key: "{index}",
                        {
                            // TUI panics if text overflows (https://github.com/DioxusLabs/dioxus/issues/371)
                            // temporary hack: just trim the strings (and make sure viewport is big enough)
                            // todo: remove
                            let mut trimmed = format!("{event:?}");
                            trimmed.truncate(200);
                            trimmed
                        }
                    }
                }
            }
        }
    }
}
