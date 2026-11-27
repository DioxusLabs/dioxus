//! `MountedData::set_pointer_capture` sends a pointer's events to the capturing element
//! instead of the element under the pointer.

use super::node;
use blitz_dom::Document;
use blitz_traits::events::{
    BlitzPointerEvent, MouseEventButton, MouseEventButtons, PointerCoords, UiEvent,
};
use dioxus::prelude::*;
use std::rc::Rc;

fn pointer(doc: &dioxus_native_dom::DioxusDocument, x: f32, y: f32) -> BlitzPointerEvent {
    let base = doc.inner();
    let mut event = base
        .get_node(node(&base, "#handle"))
        .unwrap()
        .synthetic_click_event_data(Default::default());
    event.coords = PointerCoords {
        page_x: x,
        page_y: y,
        screen_x: x,
        screen_y: y,
        client_x: x,
        client_y: y,
    };
    event.button = MouseEventButton::Main;
    event.buttons = MouseEventButtons::from(MouseEventButton::Main);
    event
}

fn app() -> Element {
    let mut handle = use_signal(|| None::<Rc<MountedData>>);
    let mut log = use_signal(Vec::<String>::new);
    rsx! {
        div { style: "display: flex",
            div {
                id: "handle",
                style: "width: 100px; height: 100px",
                onmounted: move |event| handle.set(Some(event.data())),
                onpointerdown: move |event| {
                    let element = handle().unwrap();
                    spawn(async move {
                        let result = element.set_pointer_capture(event.pointer_id()).await;
                        log.push(format!("capture {}", result.is_ok()));
                    });
                },
                ongotpointercapture: move |_| log.push("got".into()),
                onlostpointercapture: move |_| log.push("lost".into()),
                onpointermove: move |_| log.push("handle move".into()),
                onpointerup: move |event| {
                    let element = handle().unwrap();
                    spawn(async move {
                        let result = element.release_pointer_capture(event.pointer_id()).await;
                        log.push(format!("release {}", result.is_ok()));
                    });
                },
            }
            div {
                id: "other",
                style: "width: 100px; height: 100px",
                onpointermove: move |_| log.push("other move".into()),
            }
        }
        div { id: "log", {log().join(", ")} }
    }
}

#[test]
fn captured_pointer_events_go_to_the_capturing_element() {
    let mut doc = super::component(app);
    doc.poll(None);

    doc.handle_ui_event(UiEvent::PointerDown(pointer(&doc, 50.0, 50.0)));
    doc.poll(None);
    doc.handle_ui_event(UiEvent::PointerMove(pointer(&doc, 150.0, 50.0)));
    doc.poll(None);
    // The pointer leaves the window while captured
    doc.handle_ui_event(UiEvent::PointerLeave(pointer(&doc, 150.0, 50.0)));
    doc.poll(None);
    doc.handle_ui_event(UiEvent::PointerMove(pointer(&doc, 900.0, -40.0)));
    doc.poll(None);
    doc.handle_ui_event(UiEvent::PointerUp(pointer(&doc, 150.0, 50.0)));
    doc.poll(None);
    doc.handle_ui_event(UiEvent::PointerMove(pointer(&doc, 150.0, 60.0)));
    doc.poll(None);

    let base = doc.inner();
    let log = base.get_node(node(&base, "#log")).unwrap().text_content();
    assert_eq!(
        log,
        "capture true, got, handle move, handle move, lost, release true, other move"
    );
}
