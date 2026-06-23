#![doc = include_str!("../../docs/head.md")]

use std::{cell::RefCell, collections::HashSet, rc::Rc};

use dioxus_core::{Attribute, DynamicNode, Element, RenderError, Runtime, ScopeId, VNodeChild};
use dioxus_core_macro::*;
mod link;
pub use link::*;
mod stylesheet;
pub use stylesheet::*;
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
        use dioxus_core::use_hook;

        let cloned_value = value.clone();
        let initial = use_hook(move || value.clone());

        if initial != cloned_value {
            tracing::warn!("Changing the props of `{name}` is not supported ");
        }
    }
}

/// An error that can occur when extracting a single text node from a component
#[derive(Debug)]
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
    let mut children = vnode.children();
    let Some(child) = children.next() else {
        return Err(ExtractSingleTextNodeError::NonTemplate);
    };

    if children.next().is_some() {
        return Err(ExtractSingleTextNodeError::NonTemplate);
    }

    match (
        child,
        vnode.dynamic_node_values().len(),
        vnode.dynamic_attr_values().len(),
    ) {
        // rsx! { "static text" }
        (VNodeChild::Text(text), 0, 0) => Ok(text.text().to_string()),
        // rsx! { "title: {dynamic_text}" }
        (VNodeChild::Dynamic(anchor), 1, 0) => match anchor.nodes().next() {
            Some(slot) => match &*slot {
                DynamicNode::Text(text) => Ok(text.value.clone()),
                _ => Err(ExtractSingleTextNodeError::NonTextNode),
            },
            _ => Err(ExtractSingleTextNodeError::NonTemplate),
        },
        _ => Err(ExtractSingleTextNodeError::NonTemplate),
    }
}

#[cfg(test)]
mod tests {
    use dioxus_core::{DynamicNode, DynamicValues, VNode, VText};
    use dioxus_core_template::{TemplateRawTree, TemplateStorage};

    use super::*;

    #[test]
    fn extracts_static_text() {
        static TREE: TemplateRawTree = TemplateRawTree::StaticText("static text");
        static STORAGE: TemplateStorage<2, 1, 0> = TemplateStorage::build_from_tree(&TREE);
        let children = Ok(VNode::new(
            STORAGE.as_template(),
            DynamicValues::from_parts(None, Box::new([]), Box::new([])),
        ));

        assert_eq!(
            extract_single_text_node(&children).expect("static text should extract"),
            "static text"
        );
    }

    #[test]
    fn extracts_formatted_text() {
        static TREE: TemplateRawTree = TemplateRawTree::DynamicNode;
        static STORAGE: TemplateStorage<1, 1, 1> = TemplateStorage::build_from_tree(&TREE);
        let children = Ok(VNode::new(
            STORAGE.as_template(),
            DynamicValues::from_parts(
                None,
                Box::new([DynamicNode::Text(VText::new("title: dynamic text"))]),
                Box::new([]),
            ),
        ));

        assert_eq!(
            extract_single_text_node(&children).expect("formatted text should extract"),
            "title: dynamic text"
        );
    }
}

fn get_or_insert_root_context<T: Default + Clone + 'static>() -> T {
    let rt = Runtime::current();
    match rt.has_context::<T>(ScopeId::ROOT) {
        Some(context) => context,
        None => {
            let context = T::default();
            rt.provide_context(ScopeId::ROOT, context.clone());
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

/// Extend a list of string attributes with a list of dioxus attribute
pub(crate) fn extend_attributes(
    attributes: &mut Vec<(&'static str, String)>,
    additional_attributes: &[Attribute],
) {
    for additional_attribute in additional_attributes {
        let attribute_value_as_string = match &additional_attribute.value {
            dioxus_core::AttributeValue::Text(v) => v.to_string(),
            dioxus_core::AttributeValue::Float(v) => v.to_string(),
            dioxus_core::AttributeValue::Int(v) => v.to_string(),
            dioxus_core::AttributeValue::Bool(v) => v.to_string(),
            dioxus_core::AttributeValue::Listener(_) | dioxus_core::AttributeValue::Any(_) => {
                tracing::error!(
                    "document::* elements do not support event listeners or any value attributes. Expected displayable attribute, found {:?}",
                    additional_attribute.value
                );
                continue;
            }
            dioxus_core::AttributeValue::None => {
                continue;
            }
        };
        attributes.push((additional_attribute.name, attribute_value_as_string));
    }
}
