#![doc = include_str!("../../docs/head.md")]

use std::{cell::RefCell, collections::HashSet, rc::Rc};

use dioxus_core::{prelude::*, DynamicNode};
use dioxus_core_macro::*;

mod link;
pub use link::*;
mod meta;
pub use meta::*;
mod script;
pub use script::*;
mod style;
pub use style::*;
mod title;
pub use title::*;

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

/// An error that can occur when extracting a single text node from a component
pub enum ExtractSingleTextNodeError<'a> {
    /// The node contained an render error, so we can't extract the text node
    RenderError(&'a RenderError),
    /// There was only one child, but it wasn't a text node
    NonTextNode,
    /// There is multiple child nodes
    NonTemplate,
}

impl ExtractSingleTextNodeError<'_> {
    /// Log a warning depending on the error
    pub fn log(&self, component: &str) {
        match self {
            ExtractSingleTextNodeError::RenderError(err) => {
                tracing::error!("Error while rendering {component}: {err}");
            }
            ExtractSingleTextNodeError::NonTextNode => {
                tracing::error!(
                    "Error while rendering {component}: The children of {component} must be a single text node"
                );
            }
            ExtractSingleTextNodeError::NonTemplate => {
                tracing::error!(
                    "Error while rendering {component}: The children of {component} must be a single text node"
                );
            }
        }
    }
}

fn extract_single_text_node(children: &Element) -> Result<String, ExtractSingleTextNodeError<'_>> {
    let vnode = match children {
        Element::Ok(vnode) => vnode,
        Element::Err(err) => {
            return Err(ExtractSingleTextNodeError::RenderError(err));
        }
    };
    // The title's children must be in one of two forms:
    // 1. rsx! { "static text" }
    // 2. rsx! { "title: {dynamic_text}" }
    match vnode.template {
        // rsx! { "static text" }
        Template {
            roots: &[TemplateNode::Text { text }],
            node_paths: &[],
            attr_paths: &[],
            ..
        } => Ok(text.to_string()),
        // rsx! { "title: {dynamic_text}" }
        Template {
            roots: &[TemplateNode::Dynamic { id }],
            node_paths: &[&[0]],
            attr_paths: &[],
            ..
        } => {
            let node = &vnode.dynamic_nodes[id];
            match node {
                DynamicNode::Text(text) => Ok(text.value.clone()),
                _ => Err(ExtractSingleTextNodeError::NonTextNode),
            }
        }
        _ => Err(ExtractSingleTextNodeError::NonTemplate),
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
