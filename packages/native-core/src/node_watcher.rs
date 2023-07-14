//! Helpers for watching for changes in the DOM tree.

use crate::{node::FromAnyValue, node_ref::AttributeMask, prelude::*};

/// A trait for watching for changes in the DOM tree.
pub trait NodeWatcher<V: FromAnyValue + Send + Sync> {
    /// Called after a node is added to the tree.
    fn on_node_added(&mut self, _node: NodeMut<V>) {}
    /// Called before a node is removed from the tree.
    fn on_node_removed(&mut self, _node: NodeMut<V>) {}
    /// Called after a node is moved to a new parent.
    fn on_node_moved(&mut self, _node: NodeMut<V>) {}
}

/// A trait for watching for changes to attributes of an element.
pub trait AttributeWatcher<V: FromAnyValue + Send + Sync> {
    /// Called before update_state is called on the RealDom
    fn on_attributes_changed(&self, _node: NodeMut<V>, _attributes: &AttributeMask) {}
}
