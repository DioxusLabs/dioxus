use std::sync::Arc;

use super::*;

/// A context for the document
pub type DocumentContext = Arc<dyn Document>;

fn format_string_for_js(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn format_attributes(attributes: &[(&str, String)]) -> String {
    let mut formatted = String::from("[");
    for (key, value) in attributes {
        formatted.push_str(&format!(
            "[{}, {}],",
            format_string_for_js(key),
            format_string_for_js(value)
        ));
    }
    if formatted.ends_with(',') {
        formatted.pop();
    }
    formatted.push(']');
    formatted
}

/// Create a new element in the head with javascript through the [`Document::eval`] method
///
/// This can be used to implement the head element creation logic for most [`Document`] implementations.
pub fn create_element_in_head(
    tag: &str,
    attributes: &[(&str, String)],
    children: Option<String>,
) -> String {
    let helpers = include_str!("./js/head.js");
    let attributes = format_attributes(attributes);
    let children = children
        .as_deref()
        .map(format_string_for_js)
        .unwrap_or("null".to_string());
    let tag = format_string_for_js(tag);
    format!(r#"{helpers};window.createElementInHead({tag}, {attributes}, {children});"#)
}

/// A provider for document-related functionality.
///
/// Provides things like a history API, a title, a way to run JS, and some other basics/essentials used
/// by nearly every platform.
///
/// An integration with some kind of navigation history.
///
/// Depending on your use case, your implementation may deviate from the described procedure. This
/// is fine, as long as both `current_route` and `current_query` match the described format.
///
/// However, you should document all deviations. Also, make sure the navigation is user-friendly.
/// The described behaviors are designed to mimic a web browser, which most users should already
/// know. Deviations might confuse them.
pub trait Document: 'static {
    /// Run `eval` against this document, returning an [`Eval`] that can be used to await the result.
    fn eval(&self, js: String) -> Eval;

    /// Set the title of the document
    fn set_title(&self, title: String) {
        self.eval(format!("document.title = {title:?};"));
    }

    /// Create a new element in the head
    fn create_head_element(
        &self,
        name: &str,
        attributes: &[(&str, String)],
        contents: Option<String>,
    ) {
        // This default implementation remains to make the trait compatible with the 0.6 version, but it should not be used
        // The element should only be created inside an effect so it is not called while the component is suspended
        self.eval(create_element_in_head(name, attributes, contents));
    }

    /// Create a new meta tag in the head
    fn create_meta(&self, props: MetaProps) {
        let attributes = props.attributes();
        self.create_head_element("meta", &attributes, None);
    }

    /// Create a new script tag in the head
    fn create_script(&self, props: ScriptProps) {
        let attributes = props.attributes();
        self.create_head_element("script", &attributes, props.script_contents().ok());
    }

    /// Create a new style tag in the head
    fn create_style(&self, props: StyleProps) {
        let attributes = props.attributes();
        self.create_head_element("style", &attributes, props.style_contents().ok());
    }

    /// Create a new link tag in the head
    fn create_link(&self, props: LinkProps) {
        let attributes = props.attributes();
        self.create_head_element("link", &attributes, None);
    }

    /// Check if we should create a new head component at all. If it returns false, the head component will be skipped.
    ///
    /// This runs once per head component and is used to hydrate head components in fullstack.
    fn create_head_component(&self) -> bool {
        true
    }
}

/// A document that does nothing
#[derive(Default)]
pub struct NoOpDocument;

impl Document for NoOpDocument {
    fn eval(&self, _: String) -> Eval {
        let owner = generational_box::Owner::default();
        struct NoOpEvaluator;
        impl Evaluator for NoOpEvaluator {
            fn poll_join(
                &mut self,
                _: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
                std::task::Poll::Ready(Err(EvalError::Unsupported))
            }

            fn poll_recv(
                &mut self,
                _: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
                std::task::Poll::Ready(Err(EvalError::Unsupported))
            }

            fn send(&self, _data: serde_json::Value) -> Result<(), EvalError> {
                Err(EvalError::Unsupported)
            }
        }
        Eval::new(owner.insert(Box::new(NoOpEvaluator)))
    }

    fn set_title(&self, _: String) {}
    fn create_meta(&self, _: MetaProps) {}
    fn create_script(&self, _: ScriptProps) {}
    fn create_style(&self, _: StyleProps) {}
    fn create_link(&self, _: LinkProps) {}
}
