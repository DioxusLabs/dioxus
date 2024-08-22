use error::EvalError;

use super::*;
use std::rc::Rc;

/// A provider for document-related functionality. By default most methods are driven through [`eval`].
pub trait Document {
    /// Create a new evaluator for the document that evaluates JavaScript and facilitates communication between JavaScript and Rust.
    fn eval(&self, js: String) -> Eval;

    /// Set the title of the document
    fn set_title(&self, title: String) {
        _ = self.eval(format!("document.title = {title:?};"));
    }

    /// Create a new meta tag
    fn create_meta(&self, props: MetaProps) {
        let attributes = props.attributes();
        let js = create_element_in_head("meta", &attributes, None);
        _ = self.eval(js);
    }

    /// Create a new script tag
    fn create_script(&self, props: ScriptProps) {
        let attributes = props.attributes();
        let js = create_element_in_head("script", &attributes, props.script_contents());
        _ = self.eval(js);
    }

    /// Create a new style tag
    fn create_style(&self, props: StyleProps) {
        let attributes = props.attributes();
        let js = create_element_in_head("style", &attributes, props.style_contents());
        _ = self.eval(js);
    }

    /// Create a new link tag
    fn create_link(&self, props: head::LinkProps) {
        let attributes = props.attributes();
        let js = create_element_in_head("link", &attributes, None);
        _ = self.eval(js);
    }

    /// Get a reference to the document as `dyn Any`
    fn as_any(&self) -> &dyn std::any::Any;
}

fn create_element_in_head(
    tag: &str,
    attributes: &[(&str, String)],
    children: Option<String>,
) -> String {
    fn format_attributes(attributes: &[(&str, String)]) -> String {
        let mut formatted = String::from("[");
        for (key, value) in attributes {
            formatted.push_str(&format!("[{key:?}, {value:?}],"));
        }
        if formatted.ends_with(',') {
            formatted.pop();
        }
        formatted.push(']');
        formatted
    }

    let helpers: &str = todo!();
    // let helpers = include_str!("../js/head.js");
    let attributes = format_attributes(attributes);
    let children = children
        .map(|c| format!("\"{c}\""))
        .unwrap_or("null".to_string());
    format!(r#"{helpers};window.createElementInHead("{tag}", {attributes}, {children});"#)
}
