#[path = "events/drag_and_drop.rs"]
mod drag_and_drop;
#[path = "events/event_queue.rs"]
mod event_queue;
#[path = "events/native_wheel_data.rs"]
mod native_wheel_data;

use blitz_dom::{BaseDocument, DocumentConfig, NodeId, UserAgentStylesheet};
use blitz_html::HtmlDocument;
use dioxus::prelude::{Element, VirtualDom};
use dioxus_native_dom::DioxusDocument;

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
