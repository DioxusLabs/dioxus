//! Instructions returned by the VirtualDOM on how to modify the Real DOM.
//!
//! This module contains an internal API to generate these instructions.
//!
//! Beware that changing code in this module will break compatibility with
//! interpreters for these types of DomEdits.

use crate::innerlude::*;

/// A renderer for Dioxus to modify during diffing
///
/// The renderer should implement a Stack Machine. IE each call to the below methods are modifications to the renderer's
/// internal stack for creating and modifying nodes.
///
/// Dioxus guarantees that the stack is always in a valid state.
pub trait Renderer<'a> {
    /// Load this element onto the stack
    fn push_root(&mut self, root: ElementId);
    /// Pop the topmost element from the stack
    fn pop_root(&mut self);
    /// Replace the given element with the next m elements on the stack
    fn replace_with(&mut self, root: ElementId, m: u32);
    /// Insert the next m elements on the stack after the given element
    fn insert_after(&mut self, root: ElementId, n: u32);
    /// Insert the next m elements on the stack before the given element
    fn insert_before(&mut self, root: ElementId, n: u32);
    /// Append the next n elements on the stack to the n+1 element on the stack
    fn append_children(&mut self, n: u32);

    /// Create a new element with the given text and ElementId
    fn create_text_node(&mut self, text: &'a str, root: ElementId);
    /// Create an element with the given tag name, optional namespace, and ElementId
    /// Note that namespaces do not cascade down the tree, so the renderer must handle this if it implements namespaces
    fn create_element(&mut self, tag: &'static str, ns: Option<&'static str>, id: ElementId);
    /// Create a hidden element to be used later for replacement.
    /// Used in suspense, lists, and other places where we need to hide a node before it is ready to be shown.
    /// This is up to the renderer to implement, but it should not be visible to the user.
    fn create_placeholder(&mut self, id: ElementId);

    /// Remove the targeted node from the DOM
    fn remove(&mut self, root: ElementId);
    /// Remove an attribute from an existing element
    fn remove_attribute(&mut self, attribute: &Attribute, root: ElementId);

    /// Attach a new listener to the dom
    fn new_event_listener(&mut self, listener: &Listener, scope: ScopeId);
    /// Remove an existing listener from the dom
    fn remove_event_listener(&mut self, event: &'static str, root: ElementId);

    /// Set the text content of a node
    fn set_text(&mut self, text: &'a str, root: ElementId);
    /// Set an attribute on an element
    fn set_attribute(&mut self, attribute: &'a Attribute<'a>, root: ElementId);

    /// Save the current n nodes to the ID to be loaded later
    fn save(&mut self, id: &str, num: u32);
    /// Loads a set of saved nodes from the ID
    fn load(&mut self, id: &str);

    // General statistics for doing things that extend outside of the renderer

    ///
    fn mark_dirty_scope(&mut self, scope: ScopeId);
}
