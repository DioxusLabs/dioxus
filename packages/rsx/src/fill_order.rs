//! Shared template fill-order traversal.
//!
//! Static and dynamic attributes are visited before an element's children, matching runtime
//! `DynamicValues` order and the template anchor order.
//!
//! This module centralizes that single authoritative order — open, attributes, key, children,
//! close — so every consumer stays in lockstep with the typed
//! [`crate::TemplateBody`] view builder.

use crate::innerlude::*;

/// A consumer that fills template data while a body is walked in canonical fill order.
///
/// The driver ([`visit_roots`]) owns the structural recursion and enforces the order. Returning
/// `None` aborts the traversal. The `'a` lifetime is the borrow of the body being walked, so
/// consumers may retain the node/attribute references they are handed.
pub trait FillOrderVisitor<'a> {
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
    ///
    /// `following_static_at_parent` reports whether a later sibling at the same level contributes
    /// a static root, which the op tape needs to decide anchor placement.
    fn dynamic_node(&mut self, node: &'a BodyNode, following_static_at_parent: bool) -> Option<()>;
}

/// Walk the roots of a body in canonical fill order, driving `visitor`.
pub fn visit_roots<'a, V: FillOrderVisitor<'a>>(
    visitor: &mut V,
    nodes: &'a [BodyNode],
) -> Option<()> {
    visit_siblings(visitor, nodes)
}

fn visit_siblings<'a, V: FillOrderVisitor<'a>>(
    visitor: &mut V,
    nodes: &'a [BodyNode],
) -> Option<()> {
    for (index, node) in nodes.iter().enumerate() {
        visit_node(visitor, node, siblings_have_static_node(nodes, index + 1))?;
    }
    Some(())
}

fn visit_node<'a, V: FillOrderVisitor<'a>>(
    visitor: &mut V,
    node: &'a BodyNode,
    following_static_at_parent: bool,
) -> Option<()> {
    match node {
        BodyNode::Element(element) => visit_element(visitor, element),
        BodyNode::Text(text) if text.is_static() => visitor.static_text(text),
        BodyNode::Text(_)
        | BodyNode::RawExpr(_)
        | BodyNode::Component(_)
        | BodyNode::ForLoop(_)
        | BodyNode::IfChain(_)
        | BodyNode::SyntheticBoundary(_) => visitor.dynamic_node(node, following_static_at_parent),
    }
}

/// Walk a single element subtree in canonical fill order, driving `visitor`.
pub fn visit_element<'a, V: FillOrderVisitor<'a>>(
    visitor: &mut V,
    element: &'a Element,
) -> Option<()> {
    visitor.open_element(element)?;

    // Static attributes are lowered immediately, into the op slots that precede an element's
    // children, so they must be emitted before the children are visited.
    for attr in &element.merged_attributes {
        if is_static_attribute(attr) {
            visitor.static_attribute(element, attr)?;
        }
    }

    for attr in &element.merged_attributes {
        if !is_static_attribute(attr) {
            visitor.dynamic_attribute(element, attr)?;
        }
    }

    if let Some(key) = element.key() {
        visitor.key(element, key)?;
    }

    visit_siblings(visitor, &element.children)?;

    visitor.close_element(element)
}

/// Whether an attribute is lowered as a static op (vs. a dynamic slot). Mirrors the
/// classification the hot-reload consumers use so static/dynamic callbacks partition cleanly.
fn is_static_attribute(attr: &Attribute) -> bool {
    attr.is_static_str_literal()
}

fn siblings_have_static_node(nodes: &[BodyNode], start: usize) -> bool {
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
