//! Helpers for watching for changes in the DOM tree.

use crate::{node::FromAnyValue, prelude::*};

/// A trait for watching for changes in the DOM tree.
pub trait NodeWatcher<V: FromAnyValue + Send + Sync> {
    /// Called after a node is added to the tree.
    fn on_node_added(&self, _node: NodeMut<V>) {}
    /// Called before a node is removed from the tree.
    fn on_node_removed(&self, _node: NodeMut<V>) {}
    /// Called after a node is moved to a new parent.
    fn on_node_moved(&self, _node: NodeMut<V>) {}
    // /// Called after the text content of a node is changed.
    // fn on_text_changed(&self, _node: NodeMut<V>) {}
    // /// Called after an attribute of an element is changed.
    // fn on_attribute_changed(&self, _node: NodeMut<V>, attribute: &str) {}
}
