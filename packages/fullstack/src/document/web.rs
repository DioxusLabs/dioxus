#![allow(unused)]
//! On the client, we use the [`WebDocument`] implementation to render the head for any elements that were not rendered on the server.

use dioxus_lib::events::Document;
use dioxus_web::WebDocument;

fn head_element_written_on_server() -> bool {
    dioxus_web::take_server_data()
        .ok()
        .flatten()
        .unwrap_or_default()
}

pub(crate) struct FullstackWebDocument;

impl Document for FullstackWebDocument {
    fn new_evaluator(
        &self,
        js: String,
    ) -> generational_box::GenerationalBox<Box<dyn dioxus_lib::prelude::document::Evaluator>> {
        WebDocument.new_evaluator(js)
    }

    fn set_title(&self, title: String) {
        if head_element_written_on_server() {
            return;
        }
        WebDocument.set_title(title);
    }

    fn create_meta(&self, props: dioxus_lib::prelude::MetaProps) {
        if head_element_written_on_server() {
            return;
        }
        WebDocument.create_meta(props);
    }

    fn create_script(&self, props: dioxus_lib::prelude::ScriptProps) {
        if head_element_written_on_server() {
            return;
        }
        WebDocument.create_script(props);
    }

    fn create_style(&self, props: dioxus_lib::prelude::StyleProps) {
        if head_element_written_on_server() {
            return;
        }
        WebDocument.create_style(props);
    }

    fn create_link(&self, props: dioxus_lib::prelude::head::LinkProps) {
        if head_element_written_on_server() {
            return;
        }
        WebDocument.create_link(props);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
