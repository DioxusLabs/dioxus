// #![cfg(feature = "hot_reload")]

use crate::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Result, Token,
};

// #[cfg(feature = "hot_reload")]
#[derive(Default, Debug)]
pub(crate) struct DynamicMapping {
    attribute_to_idx: std::collections::HashMap<AttributeType, Vec<usize>>,
    last_attribute_idx: usize,
    node_to_idx: std::collections::HashMap<BodyNode, Vec<usize>>,
    last_element_idx: usize,
}

// #[cfg(feature = "hot_reload")]
impl DynamicMapping {
    pub(crate) fn from(nodes: Vec<BodyNode>) -> Self {
        let mut new = Self::default();
        for node in nodes {
            new.add_node(node);
        }
        new
    }

    pub(crate) fn get_attribute_idx(&mut self, attr: &AttributeType) -> Option<usize> {
        self.attribute_to_idx
            .get_mut(attr)
            .and_then(|idxs| idxs.pop())
    }

    pub(crate) fn get_node_idx(&mut self, node: &BodyNode) -> Option<usize> {
        self.node_to_idx.get_mut(node).and_then(|idxs| idxs.pop())
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

    pub(crate) fn add_node(&mut self, node: BodyNode) {
        match node {
            BodyNode::Element(el) => {
                for attr in el.merged_attributes {
                    match &attr {
                        AttributeType::Named(ElementAttrNamed {
                            attr:
                                ElementAttr {
                                    value: ElementAttrValue::AttrLiteral(input),
                                    ..
                                },
                            ..
                        }) if input.is_static() => {}
                        _ => {
                            self.insert_attribute(attr);
                        }
                    }
                }

                for child in el.children {
                    self.add_node(child);
                }
            }

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
}
