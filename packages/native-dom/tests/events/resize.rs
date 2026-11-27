//! `onresize` reports an element's border-box and content-box sizes after layout.

use super::node;
use blitz_dom::Document;
use blitz_traits::shell::Viewport;
use dioxus::prelude::*;

fn app() -> Element {
    let mut reported = use_signal(String::new);
    rsx! {
        style { "body {{ margin: 0; }}" }
        div {
            id: "target",
            style: "width: 50%; height: 100px; padding: 10px; box-sizing: border-box;",
            onresize: move |event| {
                let border = event.data().get_border_box_size().unwrap();
                let content = event.data().get_content_box_size().unwrap();
                reported.set(format!(
                    "{}x{} / {}x{}",
                    border.width, border.height, content.width, content.height
                ));
            },
        }
        div { id: "reported", "{reported}" }
    }
}

fn reported(doc: &dioxus_native_dom::DioxusDocument) -> String {
    let base = doc.inner();
    base.get_node(node(&base, "#reported"))
        .unwrap()
        .text_content()
}

#[test]
fn onresize_reports_initial_and_changed_sizes() {
    let mut doc = super::component(app);
    doc.resolve_and_settle(0.0, None);
    assert_eq!(reported(&doc), "150x100 / 130x80");

    let viewport = doc.inner().viewport().clone();
    doc.inner_mut().set_viewport(Viewport::new(
        600,
        300,
        viewport.hidpi_scale,
        viewport.color_scheme,
    ));
    doc.resolve_and_settle(0.0, None);
    assert_eq!(reported(&doc), "300x100 / 280x80");
}
