// API inspired by Reacts implementation of head only elements. We use components here instead of elements to simplify internals.

use std::{
    cell::RefCell,
    rc::Rc,
    task::{Context, Poll},
};

use dioxus_core::{prelude::*, DynamicNode};
use dioxus_core_macro::*;
use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};

#[allow(unused)]
mod bindings;
pub use bindings::*;
mod eval;
pub use eval::*;

fn format_attributes(attributes: &[(&str, String)]) -> String {
    let mut formatted = String::from("[");
    for (key, value) in attributes {
        formatted.push_str(&format!("[{key:?}, {value:?}],"));
    }
    formatted.pop();
    formatted.push(']');
    formatted
}

fn create_element_in_head(
    tag: &str,
    attributes: &[(&str, String)],
    children: Option<String>,
) -> String {
    let helpers = include_str!("../js/head.js");
    let attributes = format_attributes(attributes);
    let children = children
        .map(|c| format!("\"{c}\""))
        .unwrap_or("null".to_string());
    format!(r#"{helpers};createElementInHead("{tag}", {attributes}, {children});"#)
}

/// A provider for document-related functionality. By default most methods are driven through [`eval`].
pub trait Document {
    /// Create a new evaluator for the document that evaluates JavaScript and facilitates communication between JavaScript and Rust.
    fn new_evaluator(&self, js: String) -> GenerationalBox<Box<dyn Evaluator>>;

    /// Set the title of the document
    fn set_title(&self, title: String) {
        self.new_evaluator(format!("document.title = {title:?};"));
    }

    fn create_meta(&self, props: MetaProps) {
        let attributes = props.attributes();
        let js = create_element_in_head("meta", &attributes, None);
        self.new_evaluator(js);
    }

    fn create_script(&self, props: ScriptProps) {
        let attributes = props.attributes();
        let js = create_element_in_head("script", &attributes, props.script_contents());
        self.new_evaluator(js);
    }

    fn create_style(&self, props: StyleProps) {
        let attributes = props.attributes();
        let js = create_element_in_head("style", &attributes, props.style_contents());
        self.new_evaluator(js);
    }

    fn create_link(&self, props: LinkProps) {
        let attributes = props.attributes();
        let js = create_element_in_head("link", &attributes, None);
        self.new_evaluator(js);
    }
}

/// The default No-Op document
pub struct NoOpDocument;

impl Document for NoOpDocument {
    fn new_evaluator(&self, _js: String) -> GenerationalBox<Box<dyn Evaluator>> {
        tracing::error!("Eval is not supported on this platform. If you are using dioxus fullstack, you can wrap your code with `client! {{}}` to only include the code that runs eval in the client bundle.");
        UnsyncStorage::owner().insert(Box::new(NoOpEvaluator))
    }
}

struct NoOpEvaluator;
impl Evaluator for NoOpEvaluator {
    fn send(&self, _data: serde_json::Value) -> Result<(), EvalError> {
        Err(EvalError::Unsupported)
    }
    fn poll_recv(
        &mut self,
        _context: &mut Context<'_>,
    ) -> Poll<Result<serde_json::Value, EvalError>> {
        Poll::Ready(Err(EvalError::Unsupported))
    }
    fn poll_join(
        &mut self,
        _context: &mut Context<'_>,
    ) -> Poll<Result<serde_json::Value, EvalError>> {
        Poll::Ready(Err(EvalError::Unsupported))
    }
}

/// Get the document provider for the current platform or a no-op provider if the platform doesn't document functionality.
pub fn document() -> Rc<dyn Document> {
    dioxus_core::prelude::try_consume_context::<Rc<dyn Document>>()
        // Create a NoOp provider that always logs an error when trying to evaluate
        // That way, we can still compile and run the code without a real provider
        .unwrap_or_else(|| Rc::new(NoOpDocument) as Rc<dyn Document>)
}

/// Warn the user if they try to change props on a element that is injected into the head
#[allow(unused)]
fn use_update_warning<T: PartialEq + Clone + 'static>(value: &T, name: &'static str) {
    #[cfg(debug_assertions)]
    {
        let cloned_value = value.clone();
        let initial = use_hook(move || value.clone());

        if initial != cloned_value {
            tracing::warn!("Changing the props of `{name}` is not supported ");
        }
    }
}

fn extract_single_text_node(children: &Element, component: &str) -> Option<String> {
    let vnode = match children {
        Element::Ok(vnode) => vnode,
        Element::Err(err) => {
            tracing::error!("Error while rendering {component}: {err}");
            return None;
        }
    };
    // The title's children must be in one of two forms:
    // 1. rsx! { "static text" }
    // 2. rsx! { "title: {dynamic_text}" }
    match vnode.template.get() {
        // rsx! { "static text" }
        Template {
            roots: &[TemplateNode::Text { text }],
            node_paths: &[],
            attr_paths: &[],
            ..
        } => Some(text.to_string()),
        // rsx! { "title: {dynamic_text}" }
        Template {
            roots: &[TemplateNode::Dynamic { id }],
            node_paths: &[&[0]],
            attr_paths: &[],
            ..
        } => {
            let node = &vnode.dynamic_nodes[id];
            match node {
                DynamicNode::Text(text) => Some(text.value.clone()),
                _ => {
                    tracing::error!("Error while rendering {component}: The children of {component} must be a single text node. It cannot be a component, if statement, loop, or a fragment");
                    None
                }
            }
        }
        _ => {
            tracing::error!(
                "Error while rendering title: The children of title must be a single text node"
            );
            None
        }
    }
}

#[derive(Clone, Props, PartialEq)]
pub struct TitleProps {
    children: Element,
}

#[component]
pub fn Title(props: TitleProps) -> Element {
    let children = props.children;
    let Some(text) = extract_single_text_node(&children, "Title") else {
        return rsx! {};
    };

    // Update the title as it changes. NOTE: We don't use use_effect here because we need this to run on the server
    let document = use_hook(document);
    let last_text = use_hook(|| {
        // Set the title initially
        document.set_title(text.clone());
        Rc::new(RefCell::new(text.clone()))
    });

    // If the text changes, update the title
    let mut last_text = last_text.borrow_mut();
    if text != *last_text {
        document.set_title(text.clone());
        *last_text = text;
    }

    rsx! {}
}

#[derive(Clone, Props, PartialEq)]
pub struct MetaProps {
    pub name: String,
    pub charset: String,
    pub http_equiv: String,
    pub content: String,
}

impl MetaProps {
    fn attributes(&self) -> Vec<(&'static str, String)> {
        vec![
            ("name", self.name.clone()),
            ("charset", self.charset.clone()),
            ("http-equiv", self.http_equiv.clone()),
            ("content", self.content.clone()),
        ]
    }
}

#[component]
pub fn Meta(props: MetaProps) -> Element {
    use_update_warning(&props, "Meta {}");

    use_hook(|| {
        let document = document();
        document.create_meta(props);
    });

    rsx! {}
}

#[derive(Clone, Props, PartialEq)]
pub struct ScriptProps {
    pub children: Element,
    pub src: String,
    pub defer: bool,
    pub crossorigin: String,
    pub fetchpriority: String,
    pub integrity: String,
    pub nomodule: bool,
    pub nonce: String,
    pub referrerpolicy: String,
    pub r#type: String,
}

impl ScriptProps {
    fn attributes(&self) -> Vec<(&'static str, String)> {
        vec![
            ("defer", self.defer.to_string()),
            ("crossorigin", self.crossorigin.clone()),
            ("fetchpriority", self.fetchpriority.clone()),
            ("integrity", self.integrity.clone()),
            ("nomodule", self.nomodule.to_string()),
            ("nonce", self.nonce.clone()),
            ("referrerpolicy", self.referrerpolicy.clone()),
            ("type", self.r#type.clone()),
        ]
    }

    pub fn script_contents(&self) -> Option<String> {
        extract_single_text_node(&self.children, "Script")
    }
}

#[component]
pub fn Script(props: ScriptProps) -> Element {
    use_update_warning(&props, "Script {}");

    use_hook(|| {
        let document = document();
        document.create_script(props);
    });

    rsx! {}
}

#[derive(Clone, Props, PartialEq)]
pub struct StyleProps {
    // Allows React to de-duplicate styles that have the same href.
    pub href: String,
    pub media: String,
    pub nonce: String,
    pub title: String,
    pub children: Element,
}

impl StyleProps {
    fn attributes(&self) -> Vec<(&'static str, String)> {
        vec![
            ("href", self.href.clone()),
            ("media", self.media.clone()),
            ("nonce", self.nonce.clone()),
            ("title", self.title.clone()),
        ]
    }

    pub fn style_contents(&self) -> Option<String> {
        extract_single_text_node(&self.children, "Title")
    }
}

#[component]
pub fn Style(props: StyleProps) -> Element {
    use_update_warning(&props, "Style {}");

    use_hook(|| {
        let document = document();
        document.create_style(props);
    });

    rsx! {}
}

#[derive(Clone, Props, PartialEq)]
pub struct LinkProps {
    pub rel: String,
    pub media: String,
    pub title: String,
    pub disabled: bool,
    pub r#as: String,
    pub sizes: String,
    pub href: String,
    pub crossorigin: String,
    pub referrerpolicy: String,
    pub fetchpriority: String,
    pub hreflang: String,
    pub integrity: String,
    pub r#type: String,
    pub blocking: String,
}

impl LinkProps {
    fn attributes(&self) -> Vec<(&'static str, String)> {
        vec![
            ("rel", self.rel.clone()),
            ("media", self.media.clone()),
            ("title", self.title.clone()),
            ("disabled", self.disabled.to_string()),
            ("as", self.r#as.clone()),
            ("sizes", self.sizes.clone()),
            ("href", self.href.clone()),
            ("crossOrigin", self.crossorigin.clone()),
            ("referrerPolicy", self.referrerpolicy.clone()),
            ("fetchPriority", self.fetchpriority.clone()),
            ("hrefLang", self.hreflang.clone()),
            ("integrity", self.integrity.clone()),
            ("type", self.r#type.clone()),
            ("blocking", self.blocking.clone()),
        ]
    }
}

#[component]
pub fn Link(props: LinkProps) -> Element {
    use_update_warning(&props, "Link {}");

    use_hook(|| {
        let document = document();
        document.create_link(props);
    });

    rsx! {}
}
