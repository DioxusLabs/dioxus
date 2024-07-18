// API inspired by Reacts implementation of head only elements. We use components here instead of elements to simplify internals.

use std::{
    cell::RefCell,
    collections::HashSet,
    rc::Rc,
    task::{Context, Poll},
};

use dioxus_core::{prelude::*, DynamicNode};
use dioxus_core_macro::*;
use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};

mod bindings;
#[allow(unused)]
pub use bindings::*;
mod eval;
pub use eval::*;

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
    format!(r#"{helpers};window.createElementInHead("{tag}", {attributes}, {children});"#)
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

    fn create_link(&self, props: head::LinkProps) {
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
    pub name: Option<String>,
    pub charset: Option<String>,
    pub http_equiv: Option<String>,
    pub content: Option<String>,
}

impl MetaProps {
    fn attributes(&self) -> Vec<(&'static str, String)> {
        let mut attributes = Vec::new();
        if let Some(name) = &self.name {
            attributes.push(("name", name.clone()));
        }
        if let Some(charset) = &self.charset {
            attributes.push(("charset", charset.clone()));
        }
        if let Some(http_equiv) = &self.http_equiv {
            attributes.push(("http-equiv", http_equiv.clone()));
        }
        if let Some(content) = &self.content {
            attributes.push(("content", content.clone()));
        }
        attributes
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
    /// Scripts are deduplicated by their src attribute
    pub src: Option<String>,
    pub defer: Option<bool>,
    pub crossorigin: Option<String>,
    pub fetchpriority: Option<String>,
    pub integrity: Option<String>,
    pub nomodule: Option<bool>,
    pub nonce: Option<String>,
    pub referrerpolicy: Option<String>,
    pub r#type: Option<String>,
}

impl ScriptProps {
    fn attributes(&self) -> Vec<(&'static str, String)> {
        let mut attributes = Vec::new();
        if let Some(defer) = &self.defer {
            attributes.push(("defer", defer.to_string()));
        }
        if let Some(crossorigin) = &self.crossorigin {
            attributes.push(("crossorigin", crossorigin.clone()));
        }
        if let Some(fetchpriority) = &self.fetchpriority {
            attributes.push(("fetchpriority", fetchpriority.clone()));
        }
        if let Some(integrity) = &self.integrity {
            attributes.push(("integrity", integrity.clone()));
        }
        if let Some(nomodule) = &self.nomodule {
            attributes.push(("nomodule", nomodule.to_string()));
        }
        if let Some(nonce) = &self.nonce {
            attributes.push(("nonce", nonce.clone()));
        }
        if let Some(referrerpolicy) = &self.referrerpolicy {
            attributes.push(("referrerpolicy", referrerpolicy.clone()));
        }
        if let Some(r#type) = &self.r#type {
            attributes.push(("type", r#type.clone()));
        }
        attributes
    }

    pub fn script_contents(&self) -> Option<String> {
        extract_single_text_node(&self.children, "Script")
    }
}

#[component]
pub fn Script(props: ScriptProps) -> Element {
    use_update_warning(&props, "Script {}");

    use_hook(|| {
        if let Some(src) = &props.src {
            if !should_insert_script(src) {
                return;
            }
        }

        let document = document();
        document.create_script(props);
    });

    rsx! {}
}

#[derive(Clone, Props, PartialEq)]
pub struct StyleProps {
    /// Styles are deduplicated by their href attribute
    pub href: Option<String>,
    pub media: Option<String>,
    pub nonce: Option<String>,
    pub title: Option<String>,
    pub children: Element,
}

impl StyleProps {
    fn attributes(&self) -> Vec<(&'static str, String)> {
        let mut attributes = Vec::new();
        if let Some(href) = &self.href {
            attributes.push(("href", href.clone()));
        }
        if let Some(media) = &self.media {
            attributes.push(("media", media.clone()));
        }
        if let Some(nonce) = &self.nonce {
            attributes.push(("nonce", nonce.clone()));
        }
        if let Some(title) = &self.title {
            attributes.push(("title", title.clone()));
        }
        attributes
    }

    pub fn style_contents(&self) -> Option<String> {
        extract_single_text_node(&self.children, "Title")
    }
}

#[component]
pub fn Style(props: StyleProps) -> Element {
    use_update_warning(&props, "Style {}");

    use_hook(|| {
        if let Some(href) = &props.href {
            if !should_insert_style(href) {
                return;
            }
        }
        let document = document();
        document.create_style(props);
    });

    rsx! {}
}

pub mod head {
    //! This module just contains the [`Link`] component which renders a `<link>` element in the head of the page. Note: This is different than the [Link](https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Link.html) component in dioxus router.

    use super::*;

    #[derive(Clone, Props, PartialEq)]
    pub struct LinkProps {
        pub rel: Option<String>,
        pub media: Option<String>,
        pub title: Option<String>,
        pub disabled: Option<bool>,
        pub r#as: Option<String>,
        pub sizes: Option<String>,
        /// Links are deduplicated by their href attribute
        pub href: Option<String>,
        pub crossorigin: Option<String>,
        pub referrerpolicy: Option<String>,
        pub fetchpriority: Option<String>,
        pub hreflang: Option<String>,
        pub integrity: Option<String>,
        pub r#type: Option<String>,
        pub blocking: Option<String>,
    }

    impl LinkProps {
        pub(crate)fn attributes(&self) -> Vec<(&'static str, String)> {
            let mut attributes = Vec::new();
            if let Some(rel) = &self.rel {
                attributes.push(("rel", rel.clone()));
            }
            if let Some(media) = &self.media {
                attributes.push(("media", media.clone()));
            }
            if let Some(title) = &self.title {
                attributes.push(("title", title.clone()));
            }
            if let Some(disabled) = &self.disabled {
                attributes.push(("disabled", disabled.to_string()));
            }
            if let Some(r#as) = &self.r#as {
                attributes.push(("as", r#as.clone()));
            }
            if let Some(sizes) = &self.sizes {
                attributes.push(("sizes", sizes.clone()));
            }
            if let Some(href) = &self.href {
                attributes.push(("href", href.clone()));
            }
            if let Some(crossorigin) = &self.crossorigin {
                attributes.push(("crossOrigin", crossorigin.clone()));
            }
            if let Some(referrerpolicy) = &self.referrerpolicy {
                attributes.push(("referrerPolicy", referrerpolicy.clone()));
            }
            if let Some(fetchpriority) = &self.fetchpriority {
                attributes.push(("fetchPriority", fetchpriority.clone()));
            }
            if let Some(hreflang) = &self.hreflang {
                attributes.push(("hrefLang", hreflang.clone()));
            }
            if let Some(integrity) = &self.integrity {
                attributes.push(("integrity", integrity.clone()));
            }
            if let Some(r#type) = &self.r#type {
                attributes.push(("type", r#type.clone()));
            }
            if let Some(blocking) = &self.blocking {
                attributes.push(("blocking", blocking.clone()));
            }
            attributes
        }
    }

    #[doc(alias = "<link>")]
    #[component]
    pub fn Link(props: LinkProps) -> Element {
        use_update_warning(&props, "Link {}");

        use_hook(|| {
            if let Some(href) = &props.href {
                if !should_insert_link(href) {
                    return;
                }
            }
            let document = document();
            document.create_link(props);
        });

        rsx! {}
    }
}

fn get_or_insert_root_context<T: Default + Clone + 'static>() -> T {
    match ScopeId::ROOT.has_context::<T>() {
        Some(context) => context,
        None => {
            let context = T::default();
            ScopeId::ROOT.provide_context(context.clone());
            context
        }
    }
}

#[derive(Default, Clone)]
struct LinkContext(DeduplicationContext);

fn should_insert_link(href: &str) -> bool {
    get_or_insert_root_context::<LinkContext>()
        .0
        .should_insert(href)
}

#[derive(Default, Clone)]
struct ScriptContext(DeduplicationContext);

fn should_insert_script(src: &str) -> bool {
    get_or_insert_root_context::<ScriptContext>()
        .0
        .should_insert(src)
}

#[derive(Default, Clone)]
struct StyleContext(DeduplicationContext);

fn should_insert_style(href: &str) -> bool {
    get_or_insert_root_context::<StyleContext>()
        .0
        .should_insert(href)
}

#[derive(Default, Clone)]
struct DeduplicationContext(Rc<RefCell<HashSet<String>>>);

impl DeduplicationContext {
    fn should_insert(&self, href: &str) -> bool {
        let mut set = self.0.borrow_mut();
        let present = set.contains(href);
        if !present {
            set.insert(href.to_string());
            true
        } else {
            false
        }
    }
}
