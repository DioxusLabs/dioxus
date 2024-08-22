use error::EvalError;

use super::*;
use std::rc::Rc;

/// A provider for document-related functionality. By default most methods are driven through [`eval`].
pub trait Document {
    /// Create a new evaluator for the document that evaluates JavaScript and facilitates communication between JavaScript and Rust.
    fn eval(&self, js: String) -> Eval;

    /// Set the title of the document
    fn set_title(&self, title: String);

    /// Create a new element in the head
    fn create_head_element(
        &self,
        name: &str,
        attributes: Vec<(&str, String)>,
        contents: Option<String>,
    );

    /// Create a new meta tag in the head
    fn create_meta(&self, props: MetaProps) {
        self.create_head_element("meta", props.attributes(), None);
    }

    /// Create a new script tag in the head
    fn create_script(&self, props: ScriptProps) {
        self.create_head_element("script", props.attributes(), props.script_contents());
    }

    /// Create a new style tag in the head
    fn create_style(&self, props: StyleProps) {
        self.create_head_element("style", props.attributes(), props.style_contents());
    }

    /// Create a new link tag in the head
    fn create_link(&self, props: LinkProps) {
        self.create_head_element("link", props.attributes(), None);
    }

    /// Get a reference to the document as `dyn Any`
    fn as_any(&self) -> &dyn std::any::Any;
}

// _ = self.eval(format!("document.title = {title:?};"));
//         let attributes = props.attributes();
//         let js = create_element_in_head("meta", &attributes, None);
//         _ = self.eval(js);
//         let attributes = props.attributes();
//         let js = create_element_in_head("script", &attributes, props.script_contents());
//         _ = self.eval(js);
//         let attributes = props.attributes();
//         let js = create_element_in_head("style", &attributes, props.style_contents());
//         _ = self.eval(js);
//         let attributes = props.attributes();
//         let js = create_element_in_head("link", &attributes, None);
//         _ = self.eval(js);

// fn create_element_in_head(
//     tag: &str,
//     attributes: &[(&str, String)],
//     children: Option<String>,
// ) -> String {
//     fn format_attributes(attributes: &[(&str, String)]) -> String {
//         let mut formatted = String::from("[");
//         for (key, value) in attributes {
//             formatted.push_str(&format!("[{key:?}, {value:?}],"));
//         }
//         if formatted.ends_with(',') {
//             formatted.pop();
//         }
//         formatted.push(']');
//         formatted
//     }

//     let helpers: &str = todo!();
//     // let helpers = include_str!("../js/head.js");
//     let attributes = format_attributes(attributes);
//     let children = children
//         .map(|c| format!("\"{c}\""))
//         .unwrap_or("null".to_string());
//     format!(r#"{helpers};window.createElementInHead("{tag}", {attributes}, {children});"#)
// }
