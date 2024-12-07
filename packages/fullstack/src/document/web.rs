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
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(format!("document.title = {title:?};"));
        });
    }

    /// Create a new meta tag in the head
    fn create_meta(&self, props: MetaProps) {
        if head_element_written_on_server() {
            return;
        }
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(create_element_in_head("meta", &props.attributes(), None));
        });
    }

    /// Create a new script tag in the head
    fn create_script(&self, props: ScriptProps, fresh_url: bool) {
        if head_element_written_on_server() {
            return;
        }
        if !fresh_url {
            return;
        }
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(create_element_in_head(
                "script",
                &props.attributes(),
                props.script_contents().ok(),
            ));
        });
    }

    /// Create a new style tag in the head
    fn create_style(&self, props: StyleProps, fresh_url: bool) {
        if head_element_written_on_server() {
            return;
        }
        if !fresh_url {
            return;
        }
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(create_element_in_head(
                "style",
                &props.attributes(),
                props.style_contents().ok(),
            ));
        });
    }

    /// Create a new link tag in the head
    fn create_link(&self, props: LinkProps, fresh_url: bool) {
        if head_element_written_on_server() {
            return;
        }
        if !fresh_url {
            return;
        }
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(create_element_in_head("link", &props.attributes(), None));
        });
    }
}
