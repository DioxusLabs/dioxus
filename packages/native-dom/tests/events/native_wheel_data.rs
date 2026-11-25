//! A Dioxus Native app reads a wheel event's gesture phase and source by
//! downcasting its data to `NativeWheelData`.

use blitz_dom::Document;
use blitz_traits::events::{
    BlitzWheelDelta, BlitzWheelEvent, BlitzWheelPhase, BlitzWheelSource, UiEvent,
};
use dioxus::prelude::*;
use dioxus_native_dom::NativeWheelData;

use super::{component, node};

fn app() -> Element {
    let mut seen = use_signal(String::new);
    rsx! {
        div {
            id: "scroller",
            style: "width: 100px; height: 100px",
            onwheel: move |event| {
                let data = event.data();
                let native = data.downcast::<NativeWheelData>().unwrap();
                seen.set(format!("{:?} {:?}", native.phase(), native.source()));
            },
        }
        div { id: "seen", "{seen}" }
    }
}

#[test]
fn an_onwheel_handler_reads_the_phase_and_source() {
    let mut doc = component(app);

    let pointer = {
        let base = doc.inner();
        base.get_node(node(&base, "#scroller"))
            .unwrap()
            .synthetic_click_event_data(Default::default())
    };
    let coords = pointer.coords;
    doc.handle_ui_event(UiEvent::PointerMove(pointer));
    doc.handle_ui_event(UiEvent::Wheel(BlitzWheelEvent {
        delta: BlitzWheelDelta::Pixels(0.0, -10.0),
        phase: BlitzWheelPhase::Started,
        source: BlitzWheelSource::Finger,
        coords,
        buttons: Default::default(),
        mods: Default::default(),
        element: Default::default(),
    }));
    doc.poll(None);

    let base = doc.inner();
    let seen = base.get_node(node(&base, "#seen")).unwrap().text_content();
    assert_eq!(seen, "Started Finger");
}
