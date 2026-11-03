//! Shared template fill-order traversal.
//!
//! Static and dynamic attributes are visited before an element's children, matching runtime
//! `DynamicValues` node/attribute array order and the template anchor order.
//!
//! This module centralizes that single authoritative order - open, attributes, key, children,
//! close - so every consumer stays in lockstep with the typed
//! [`crate::TemplateBody`] view builder.

use crate::innerlude::*;

/// A consumer that fills template data while a body is walked in canonical fill order.
///
/// The default `visit_*` methods own the structural recursion and enforce the order. Returning
/// `None` aborts the traversal. The `'a` lifetime is the borrow of the body being walked, so
/// consumers may retain the node/attribute references they are handed.
pub trait FillOrderVisitor<'a> {
    /// Walk the roots or children in canonical fill order.
    fn visit_siblings(&mut self, nodes: &'a [BodyNode]) -> Option<()> {
        for node in nodes {
            self.visit_node(node)?;
        }
        Some(())
    }

    /// Walk a single node in canonical fill order.
    fn visit_node(&mut self, node: &'a BodyNode) -> Option<()> {
        match node {
            BodyNode::Element(element) => self.visit_element(element),
            BodyNode::Text(text) if text.is_static() => self.static_text(text),
            BodyNode::Text(_)
            | BodyNode::RawExpr(_)
            | BodyNode::Component(_)
            | BodyNode::ForLoop(_)
            | BodyNode::IfChain(_)
            | BodyNode::SyntheticBoundary(_) => self.dynamic_node(node),
        }
    }

    /// Walk a single element subtree in canonical fill order.
    fn visit_element(&mut self, element: &'a Element) -> Option<()> {
        self.open_element(element)?;

        // Static attributes are lowered immediately, into the op slots that precede an element's
        // children, so they must be emitted before the children are visited.
        for attr in &element.merged_attributes {
            if is_static_attribute(attr) {
                self.static_attribute(element, attr)?;
            }
        }

        for attr in &element.merged_attributes {
            if !is_static_attribute(attr) {
                self.dynamic_attribute(element, attr)?;
            }
        }

        if let Some(key) = element.key() {
            self.key(element, key)?;
        }

        self.visit_siblings(&element.children)?;

        self.close_element(element)
    }

    /// Called when an element is opened, before any of its attributes or children.
    fn open_element(&mut self, _element: &'a Element) -> Option<()> {
        Some(())
    }

    /// Called for each static-literal attribute, *before* the element's children.
    fn static_attribute(&mut self, _element: &'a Element, _attr: &'a Attribute) -> Option<()> {
        Some(())
    }

    /// Called for each non-static (dynamic) attribute before the element's children.
    fn dynamic_attribute(&mut self, element: &'a Element, attr: &'a Attribute) -> Option<()>;

    /// Called once per element after its attributes, with the element's key value if it has one.
    fn key(&mut self, _element: &'a Element, _key: &'a AttributeValue) -> Option<()> {
        Some(())
    }

    /// Called when an element is closed, after its attributes, key, and children.
    fn close_element(&mut self, _element: &'a Element) -> Option<()> {
        Some(())
    }

    /// Called for a static text node.
    fn static_text(&mut self, _text: &'a TextNode) -> Option<()> {
        Some(())
    }

    /// Called for a dynamic node (dynamic text, raw expr, component, control flow, boundary).
    fn dynamic_node(&mut self, node: &'a BodyNode) -> Option<()>;
}

/// Walk the roots of a body in canonical fill order, driving `visitor`.
pub fn visit_roots<'a, V: FillOrderVisitor<'a>>(
    visitor: &mut V,
    nodes: &'a [BodyNode],
) -> Option<()> {
    visitor.visit_siblings(nodes)
}

/// Walk a single element subtree in canonical fill order, driving `visitor`.
pub fn visit_element<'a, V: FillOrderVisitor<'a>>(
    visitor: &mut V,
    element: &'a Element,
) -> Option<()> {
    visitor.visit_element(element)
}

/// Whether an attribute is lowered as a static op (vs. a dynamic slot). Mirrors the
/// classification the hot-reload consumers use so static/dynamic callbacks partition cleanly.
fn is_static_attribute(attr: &Attribute) -> bool {
    attr.is_static_str_literal()
}

/// Whether any sibling from `start` onward contributes a static root node.
///
/// Determines whether a dynamic node is "followed by a static node" at its parent level, which
/// governs how its anchor is placed. Shared so the macro stats walk and the hot-reload template
/// builder stay byte-for-byte in lockstep.
pub fn siblings_have_static_node(nodes: &[BodyNode], start: usize) -> bool {
    nodes[start..].iter().any(node_has_static_root)
}

fn node_has_static_root(node: &BodyNode) -> bool {
    match node {
        BodyNode::Element(_) => true,
        BodyNode::Text(text) => text.is_static(),
        BodyNode::RawExpr(_)
        | BodyNode::Component(_)
        | BodyNode::ForLoop(_)
        | BodyNode::IfChain(_)
        | BodyNode::SyntheticBoundary(_) => false,
    }
}
