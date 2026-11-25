//! On the client, we use the [`FullstackWebDocument`] implementation to render the head for any elements that were not rendered on the server.

use dioxus_document::*;

fn head_element_written_on_server() -> bool {
    crate::transport::head_element_hydration_entry()
        .get()
        .ok()
        .unwrap_or_default()
}

/// A document provider for fullstack web clients
#[derive(Clone)]
pub struct FullstackWebDocument<D> {
    document: D,
}

impl<D> From<D> for FullstackWebDocument<D> {
    fn from(document: D) -> Self {
        Self { document }
    }
}

impl<D: Document> Document for FullstackWebDocument<D> {
    fn eval(&self, js: String) -> Eval {
        self.document.eval(js)
    }

    fn supports_global_listeners(&self) -> bool {
        self.document.supports_global_listeners()
    }

    fn add_window_listener(
        &self,
        name: &'static str,
        options: &GlobalListenerOptions,
        callback: GlobalEventCallback,
    ) -> Option<GlobalListenerRegistration> {
        self.document.add_window_listener(name, options, callback)
    }

    fn add_document_listener(
        &self,
        name: &'static str,
        options: &GlobalListenerOptions,
        callback: GlobalEventCallback,
    ) -> Option<GlobalListenerRegistration> {
        self.document.add_document_listener(name, options, callback)
    }

    /// Set the title of the document
    fn set_title(&self, title: String) {
        self.document.set_title(title);
    }

    /// Create a new meta tag in the head
    fn create_meta(&self, props: MetaProps) {
        self.document.create_meta(props);
    }

    /// Create a new script tag in the head
    fn create_script(&self, props: ScriptProps) {
        self.document.create_script(props);
    }

    /// Create a new style tag in the head
    fn create_style(&self, props: StyleProps) {
        self.document.create_style(props);
    }

    /// Create a new link tag in the head
    fn create_link(&self, props: LinkProps) {
        self.document.create_link(props);
    }

    fn create_head_component(&self) -> bool {
        !head_element_written_on_server()
    }
}
