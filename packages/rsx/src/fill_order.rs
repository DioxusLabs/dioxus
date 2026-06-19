//! Shared template fill-order traversal.
//!
//! Two orderings have to be enforced together when walking an element, and they differ:
//!
//! - **Structural op tape:** a static attribute is lowered immediately, into the op slots that
//!   sit *before* an element's children (`create_static_prototype` reads attrs in
//!   `[children_start, first_child)` and children in `[first_child, end)`). So static attributes
//!   must be emitted *before* children.
//! - **Dynamic-value indices:** dynamic attributes are *deferred* and flushed at `close_element`,
//!   after every child anchor, so the renderer indexes them after the children's dynamic nodes.
//!   So dynamic attributes must be visited *after* children, or their node/attribute slots get
//!   transposed.
//!
//! This module centralizes that single authoritative order — open, static attributes, children,
//! dynamic attributes, key, close — so every consumer (hot-reload template builders, dynamic
//! pools) stays in lockstep with the typed [`crate::TemplateBody`] view builder.

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

    /// Called for each static-literal attribute, *before* the element's children, matching the
    /// op tape where static attributes precede child nodes.
    fn static_attribute(&mut self, _element: &'a Element, _attr: &'a Attribute) -> Option<()> {
        Some(())
    }

    /// Called for each non-static (dynamic) attribute, *after* the element's children, matching
    /// the deferred-flush order the renderer indexes dynamic values by.
    fn dynamic_attribute(&mut self, element: &'a Element, attr: &'a Attribute) -> Option<()>;

    /// Called once per element after its attributes, with the element's key value if it has one.
    fn key(&mut self, _element: &'a Element, _key: &'a AttributeValue) -> Option<()> {
        Some(())
    }

    /// Called when an element is closed, after its children, attributes, and key.
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

fn visit_element<'a, V: FillOrderVisitor<'a>>(
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

    // Children fill their dynamic slots before this element's own (deferred) dynamic attributes.
    visit_siblings(visitor, &element.children)?;

    // Dynamic attributes are deferred to `close_element` in the op tape, so their dynamic-value
    // indices come after every child anchor.
    for attr in &element.merged_attributes {
        if !is_static_attribute(attr) {
            visitor.dynamic_attribute(element, attr)?;
        }
    }

    if let Some(key) = element.key() {
        visitor.key(element, key)?;
    }

    visitor.close_element(element)
}

/// Whether an attribute is lowered as a static op (vs. a deferred dynamic slot). Mirrors the
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
