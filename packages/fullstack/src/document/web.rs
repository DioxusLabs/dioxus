#![allow(unused)]
//! On the client, we use the [`WebDocument`] implementation to render the head for any elements that were not rendered on the server.

use dioxus_document::{Document, LinkProps, MetaProps, ScriptProps, StyleProps};
use dioxus_web::WebDocument;

fn head_element_written_on_server() -> bool {
    dioxus_web::take_server_data()
        .ok()
        .flatten()
        .unwrap_or_default()
}

pub(crate) struct FullstackWebDocument {
    document: WebDocument,
}

impl Document for FullstackWebDocument {
    fn create_head_element(
        &self,
        name: &str,
        attributes: Vec<(&str, String)>,
        contents: Option<String>,
    ) {
        if head_element_written_on_server() {
            return;
        }

        self.document
            .create_head_element(name, attributes, contents);
    }

    fn set_title(&self, title: String) {
        if head_element_written_on_server() {
            return;
        }

        self.document.set_title(title);
    }

    fn eval(&self, js: String) -> dioxus_document::Eval {
        self.document.eval(js)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
