#![allow(unused)]
//! On the client, we use the [`WebDocument`] implementation to render the head for any elements that were not rendered on the server.

use dioxus_lib::{document::*, prelude::queue_effect};
use dioxus_web::WebDocument;

fn head_element_written_on_server() -> bool {
    dioxus_web::take_server_data()
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// A document provider for fullstack web clients
#[derive(Clone)]
pub struct FullstackWebDocument;

impl Document for FullstackWebDocument {
    fn eval(&self, js: String) -> Eval {
        WebDocument.eval(js)
    }

    /// Set the title of the document
    fn set_title(&self, title: String) {
        WebDocument.set_title(title);
    }

    /// Create a new meta tag in the head
    fn create_meta(&self, props: MetaProps) {
        WebDocument.create_meta(props);
    }

    /// Create a new script tag in the head
    fn create_script(&self, props: ScriptProps) {
        WebDocument.create_script(props);
    }

    /// Create a new style tag in the head
    fn create_style(&self, props: StyleProps) {
        WebDocument.create_style(props);
    }

    /// Create a new link tag in the head
    fn create_link(&self, props: LinkProps) {
        WebDocument.create_link(props);
    }

    fn create_head_component(&self) -> bool {
        !head_element_written_on_server()
    }
}
