// #![cfg(feature = "hot_reload")]

use std::collections::HashMap;

use crate::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Result, Token,
};

/// A mapping between static nodes of a template that might change during hot reloading
///
/// If you move a div around in the template, the mapping will need to be updated to reflect the new location, however
/// the original div itself will not change (in identity, its contents could change).
#[derive(Default, Debug)]
pub(crate) struct DynamicMapping {
    attribute_to_idx: HashMap<AttributeType, Vec<usize>>,
    last_attribute_idx: usize,
    node_to_idx: HashMap<BodyNode, Vec<usize>>,
    last_element_idx: usize,
}

impl DynamicMapping {
    pub(crate) fn new(nodes: Vec<BodyNode>) -> Self {
        let mut new = Self::default();
        for node in nodes {
            new.add_node(node);
        }
        new
    }

    pub(crate) fn add_node(&mut self, node: BodyNode) {
        match node {
            // If the node is a static element, we just want to merge its attributes into the dynamic mapping
            BodyNode::Element(el) => {
                for attr in el.merged_attributes {
                    // If the attribute is a static string literal, we don't need to insert it since the attribute
                    // will be written out during the diffing phase (since it's static)
                    if !attr.is_static_str_literal() {
                        self.insert_attribute(attr);
                    }
                }

                for child in el.children {
                    self.add_node(child);
                }
            }

            // We skip
            BodyNode::Text(text) if text.is_static() => {}

            BodyNode::RawExpr(_)
            | BodyNode::Text(_)
            | BodyNode::ForLoop(_)
            | BodyNode::IfChain(_)
            | BodyNode::Component(_) => {
                self.insert_node(node);
            }
        }
    }

    pub(crate) fn insert_attribute(&mut self, attr: AttributeType) -> usize {
        let idx = self.last_attribute_idx;
        self.last_attribute_idx += 1;
        self.attribute_to_idx.entry(attr).or_default().push(idx);
        idx
    }

    pub(crate) fn insert_node(&mut self, node: BodyNode) -> usize {
        let idx = self.last_element_idx;
        self.last_element_idx += 1;
        self.node_to_idx.entry(node).or_default().push(idx);
        idx
    }

    pub(crate) fn get_attribute_idx(&mut self, attr: &AttributeType) -> Option<usize> {
        self.attribute_to_idx
            .get_mut(attr)
            .and_then(|idxs| idxs.pop())
    }

    pub(crate) fn get_node_idx(&mut self, node: &BodyNode) -> Option<usize> {
        self.node_to_idx.get_mut(node).and_then(|idxs| idxs.pop())
    }
}

impl<'a> TemplateRenderer<'a> {
    #[cfg(feature = "hot_reload")]
    pub fn update_template<Ctx: HotReloadingContext>(
        &mut self,
        previous_call: Option<CallBody>,
        location: &'static str,
    ) -> Option<Template> {
        use mapping::DynamicMapping;

        let mut mapping = previous_call.map(|call| DynamicMapping::new(call.roots));

        let mut context = DynamicContext::default();

        let mut roots = Vec::new();

        for (idx, root) in self.roots.iter().enumerate() {
            context.current_path.push(idx as u8);
            roots.push(context.update_node::<Ctx>(root, &mut mapping)?);
            context.current_path.pop();
        }

        Some(Template {
            name: location,
            roots: intern(roots.as_slice()),
            node_paths: intern(
                context
                    .node_paths
                    .into_iter()
                    .map(|path| intern(path.as_slice()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
            attr_paths: intern(
                context
                    .attr_paths
                    .into_iter()
                    .map(|path| intern(path.as_slice()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
        })
    }
}
