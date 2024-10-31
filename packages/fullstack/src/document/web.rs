#![allow(unused)]
//! On the client, we use the [`WebDocument`] implementation to render the head for any elements that were not rendered on the server.

use dioxus_lib::document::*;
use dioxus_web::WebDocument;

fn head_element_written_on_server() -> bool {
    dioxus_web::take_server_data()
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// A document provider for fullstack web clients
pub struct FullstackWebDocument;

impl Document for FullstackWebDocument {
    fn eval(&self, js: String) -> Eval {
        WebDocument.eval(js)
    }

    fn set_title(&self, title: String) {
        if head_element_written_on_server() {
            return;
        }
        WebDocument.set_title(title);
    }

    fn create_meta(&self, props: MetaProps) {
        if head_element_written_on_server() {
            return;
        }
        WebDocument.create_meta(props);
    }

    fn create_script(&self, props: ScriptProps) {
        if head_element_written_on_server() {
            return;
        }
        WebDocument.create_script(props);
    }

    fn create_style(&self, props: StyleProps) {
        if head_element_written_on_server() {
            return;
        }
        WebDocument.create_style(props);
    }

    fn create_link(&self, props: LinkProps) {
        if head_element_written_on_server() {
            return;
        }
        WebDocument.create_link(props);
    }
}
