#[path = "events/drag_and_drop.rs"]
mod drag_and_drop;
#[path = "events/event_queue.rs"]
mod event_queue;
#[path = "events/native_wheel_data.rs"]
mod native_wheel_data;

use blitz_dom::{BaseDocument, DocumentConfig, NodeId, UserAgentStylesheet};
use blitz_html::HtmlDocument;
use blitz_traits::events::{
    BlitzPointerId, BlitzScrollEvent, BlitzWheelDelta, BlitzWheelEvent, BlitzWheelPhase,
    BlitzWheelSource,
};
use dioxus::prelude::{Element, VirtualDom};
use dioxus_html::{FocusData, ScrollData, TouchData, WheelData};
use dioxus_native_dom::{
    DioxusDocument, NativeFocusData, NativeScrollData, NativeTouchData, NativeWheelData,
};

fn config() -> DocumentConfig {
    DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::default_sheet()],
        ..Default::default()
    }
}

fn resolve(doc: &mut BaseDocument) {
    doc.viewport_mut().window_size = (300, 300);
    doc.resolve(0.0);
}

fn document(html: &str) -> BaseDocument {
    let mut doc = HtmlDocument::from_html(html, config()).into_inner();
    resolve(&mut doc);
    doc
}

fn component(app: fn() -> Element) -> DioxusDocument {
    use blitz_dom::Document;

    let mut doc = DioxusDocument::new(VirtualDom::new(app), config());
    doc.initial_build();
    resolve(&mut doc.inner_mut());
    doc
}

fn node(doc: &BaseDocument, selector: &str) -> NodeId {
    doc.query_selector(selector).unwrap().unwrap()
}

#[test]
fn native_event_data_can_be_constructed_outside_the_crate() {
    let doc = document(r#"<div id="target"></div>"#);
    let mut pointer = doc
        .get_node(node(&doc, "#target"))
        .unwrap()
        .synthetic_click_event_data(Default::default());
    pointer.id = BlitzPointerId::finger(7);

    let wheel = WheelData::new(NativeWheelData::new(BlitzWheelEvent {
        delta: BlitzWheelDelta::Pixels(0.0, -10.0),
        phase: BlitzWheelPhase::Started,
        source: BlitzWheelSource::Finger,
        coords: pointer.coords,
        buttons: Default::default(),
        mods: Default::default(),
        element: Default::default(),
    }));
    let native_wheel = wheel.downcast::<NativeWheelData>().unwrap();
    assert_eq!(native_wheel.phase(), BlitzWheelPhase::Started);
    assert_eq!(native_wheel.source(), BlitzWheelSource::Finger);

    let scroll = ScrollData::new(NativeScrollData::new(BlitzScrollEvent {
        scroll_top: 30.0,
        scroll_left: 0.0,
        scroll_width: 100,
        scroll_height: 200,
        client_width: 100,
        client_height: 100,
    }));
    assert_eq!(scroll.scroll_top(), 30.0);

    let touch = TouchData::new(NativeTouchData::new(pointer));
    assert_eq!(touch.touches_changed().len(), 1);

    let focus = FocusData::new(NativeFocusData);
    assert!(focus.downcast::<NativeFocusData>().is_some());
}
