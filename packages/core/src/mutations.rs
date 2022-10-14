//! Instructions returned by the VirtualDOM on how to modify the Real DOM.
//!
//! This module contains an internal API to generate these instructions.
//!
//! Beware that changing code in this module will break compatibility with
//! interpreters for these types of DomEdits.

use crate::innerlude::*;
use std::{any::Any, fmt::Debug};

/// ## Mutations
///
/// This method returns "mutations" - IE the necessary changes to get the RealDOM to match the VirtualDOM. It also
/// includes a list of NodeRefs that need to be applied and effects that need to be triggered after the RealDOM has
/// applied the edits.
///
/// Mutations are the only link between the RealDOM and the VirtualDOM.
pub struct Mutations<'a, E: Edits<'a>> {
    /// The list of edits that need to be applied for the RealDOM to match the VirtualDOM.
    pub edits: E,

    /// The list of Scopes that were diffed, created, and removed during the Diff process.
    pub dirty_scopes: FxHashSet<ScopeId>,

    /// The list of nodes to connect to the RealDOM.
    pub refs: Vec<NodeRefMutation<'a>>,
}

impl<'a, E: Debug + Edits<'a>> Debug for Mutations<'a, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mutations")
            .field("edits", &self.edits)
            .field("noderefs", &self.refs)
            .finish()
    }
}

/// A `DomEdit` represents a serialized form of the VirtualDom's trait-based API. This allows streaming edits across the
/// network or through FFI boundaries.
#[derive(Debug, PartialEq)]
pub enum DomEdit<'bump> {
    /// Append one or more nodes to a parent node.
    AppendChildren {
        /// The parent to append nodes to.
        root: Option<u64>,

        /// The ids of the children to append.
        children: Vec<u64>,
    },

    /// Replace a given (single) node with a handful of nodes.
    ReplaceWith {
        /// The ID of the node to be replaced.
        root: Option<u64>,

        /// The ids of the nodes to replace the root with.
        nodes: Vec<u64>,
    },

    /// Insert a number of nodes after a given node.
    InsertAfter {
        /// The ID of the node to insert after.
        root: Option<u64>,

        /// The ids of the nodes to insert after the target node.
        nodes: Vec<u64>,
    },

    /// Insert a number of nodes before a given node.
    InsertBefore {
        /// The ID of the node to insert before.
        root: Option<u64>,

        /// The ids of the nodes to insert before the target node.
        nodes: Vec<u64>,
    },

    /// Remove a particular node from the DOM
    Remove {
        /// The ID of the node to remove.
        root: Option<u64>,
    },

    /// Create a new purely-text node
    CreateTextNode {
        /// The ID the new node should have.
        root: Option<u64>,

        /// The textcontent of the node
        text: &'bump str,
    },

    /// Create a new purely-element node
    CreateElement {
        /// The ID the new node should have.
        root: Option<u64>,

        /// The tagname of the node
        tag: &'bump str,

        /// The number of children nodes that will follow this message.
        children: u32,
    },

    /// Create a new purely-comment node with a given namespace
    CreateElementNs {
        /// The ID the new node should have.
        root: Option<u64>,

        /// The namespace of the node
        tag: &'bump str,

        /// The namespace of the node (like `SVG`)
        ns: &'static str,

        /// The number of children nodes that will follow this message.
        children: u32,
    },

    /// Create a new placeholder node.
    /// In most implementations, this will either be a hidden div or a comment node.
    CreatePlaceholder {
        /// The ID the new node should have.
        root: Option<u64>,
    },

    /// Create a new Event Listener.
    NewEventListener {
        /// The name of the event to listen for.
        event_name: &'static str,

        /// The ID of the node to attach the listener to.
        scope: ScopeId,

        /// The ID of the node to attach the listener to.
        root: Option<u64>,
    },

    /// Remove an existing Event Listener.
    RemoveEventListener {
        /// The ID of the node to remove.
        root: Option<u64>,

        /// The name of the event to remove.
        event: &'static str,
    },

    /// Set the textcontent of a node.
    SetText {
        /// The ID of the node to set the textcontent of.
        root: Option<u64>,

        /// The textcontent of the node
        text: &'bump str,
    },

    /// Set the value of a node's attribute.
    SetAttribute {
        /// The ID of the node to set the attribute of.
        root: Option<u64>,

        /// The name of the attribute to set.
        field: &'static str,

        /// The value of the attribute.
        value: &'bump AttributeValue<'bump>,

        // value: &'bump str,
        /// The (optional) namespace of the attribute.
        /// For instance, "style" is in the "style" namespace.
        ns: Option<&'bump str>,
    },

    /// Remove an attribute from a node.
    RemoveAttribute {
        /// The ID of the node to remove.
        root: Option<u64>,

        /// The name of the attribute to remove.
        name: &'static str,

        /// The namespace of the attribute.
        ns: Option<&'bump str>,
    },

    /// Clones a node.
    CloneNode {
        /// The ID of the node to clone.
        id: Option<u64>,

        /// The ID of the new node.
        new_id: u64,
    },

    /// Clones the children of a node. (allows cloning fragments)
    CloneNodeChildren {
        /// The ID of the node to clone.
        id: Option<u64>,

        /// The ID of the new node.
        new_ids: Vec<u64>,
    },

    /// Navigates to the last node to the first child of the current node.
    FirstChild {},

    /// Navigates to the last node to the last child of the current node.
    NextSibling {},

    /// Navigates to the last node to the parent of the current node.
    ParentNode {},

    /// Stores the last node with a new id.
    StoreWithId {
        /// The ID of the node to store.
        id: u64,
    },

    /// Manually set the last node.
    SetLastNode {
        /// The ID to set the last node to.
        id: u64,
    },
}

use rustc_hash::FxHashSet;

/// Edits are a set of mutations that can be applied to a DOM.
#[allow(unused)]
pub trait Edits<'a>: Default {
    /// If any edits have been made.
    fn is_empty(&self) -> bool;

    /// Replace a given (single) node with a handful of nodes.
    fn replace_with(&mut self, root: Option<u64>, nodes: Vec<u64>);

    /// Insert a number of nodes after a given node.
    fn insert_after(&mut self, root: Option<u64>, nodes: Vec<u64>);

    /// Insert a number of nodes before a given node.
    fn insert_before(&mut self, root: Option<u64>, nodes: Vec<u64>);

    /// Append one or more nodes to a parent node.
    fn append_children(&mut self, root: Option<u64>, children: Vec<u64>);

    /// Remove a Node from the dom
    fn remove(&mut self, id: Option<u64>);

    /// Create a new-text node
    fn create_text_node(&mut self, text: &'a str, id: Option<u64>);

    /// Create a new-element node
    fn create_element(
        &mut self,
        tag: &'static str,
        ns: Option<&'static str>,
        id: Option<u64>,
        children: u32,
    );

    /// Create a new placeholder node
    fn create_placeholder(&mut self, id: Option<u64>);

    /// Create a new Event Listener.
    fn new_event_listener(&mut self, listener: &Listener, scope: ScopeId);

    /// Remove an existing Event Listener.
    fn remove_event_listener(&mut self, event: &'static str, root: Option<u64>);

    /// Set the textcontent of a node.
    fn set_text(&mut self, text: &'a str, root: Option<u64>);

    /// Set the value of a node's attribute.
    fn set_attribute(&mut self, attribute: &'a Attribute<'a>, root: Option<u64>);

    /// Remove an attribute from a node.
    fn remove_attribute(&mut self, attribute: &Attribute, root: Option<u64>);

    /// Clone a node
    fn clone_node(&mut self, id: Option<u64>, new_id: u64);

    /// Clone the children of a node.
    fn clone_node_children(&mut self, id: Option<u64>, new_ids: Vec<u64>);

    /// Navigates the last node to the first child of the current node.
    fn first_child(&mut self);

    /// Navigates the last node to the last child of the current node.
    fn next_sibling(&mut self);

    /// Navigates the last node to the parent of the current node.
    fn parent_node(&mut self);

    /// Stores the last node with a new id.
    fn store_with_id(&mut self, id: u64);

    /// Manually set the last node.
    fn set_last_node(&mut self, id: u64);
}

use DomEdit::*;

#[allow(unused)]
impl<'a> Edits<'a> for Vec<DomEdit<'a>> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn replace_with(&mut self, root: Option<u64>, nodes: Vec<u64>) {
        self.push(ReplaceWith { nodes, root });
    }

    fn insert_after(&mut self, root: Option<u64>, nodes: Vec<u64>) {
        self.push(InsertAfter { nodes, root });
    }

    fn insert_before(&mut self, root: Option<u64>, nodes: Vec<u64>) {
        self.push(InsertBefore { nodes, root });
    }

    fn append_children(&mut self, root: Option<u64>, children: Vec<u64>) {
        self.push(AppendChildren { root, children });
    }

    // Remove Nodes from the dom
    fn remove(&mut self, id: Option<u64>) {
        self.push(Remove { root: id });
    }

    // Create
    fn create_text_node(&mut self, text: &'a str, id: Option<u64>) {
        self.push(CreateTextNode { text, root: id });
    }

    fn create_element(
        &mut self,
        tag: &'static str,
        ns: Option<&'static str>,
        id: Option<u64>,
        children: u32,
    ) {
        match ns {
            Some(ns) => self.push(CreateElementNs {
                root: id,
                ns,
                tag,
                children,
            }),
            None => self.push(CreateElement {
                root: id,
                tag,
                children,
            }),
        }
    }

    // placeholders are nodes that don't get rendered but still exist as an "anchor" in the real dom
    fn create_placeholder(&mut self, id: Option<u64>) {
        self.push(CreatePlaceholder { root: id });
    }

    // events
    fn new_event_listener(&mut self, listener: &Listener, scope: ScopeId) {
        let Listener {
            event,
            mounted_node,
            ..
        } = listener;

        let element_id = Some(mounted_node.get().unwrap().into());

        self.push(NewEventListener {
            scope,
            event_name: event,
            root: element_id,
        });
    }

    fn remove_event_listener(&mut self, event: &'static str, root: Option<u64>) {
        self.push(RemoveEventListener { event, root });
    }

    // modify
    fn set_text(&mut self, text: &'a str, root: Option<u64>) {
        self.push(SetText { text, root });
    }

    fn set_attribute(&mut self, attribute: &'a Attribute<'a>, root: Option<u64>) {
        let Attribute {
            value, attribute, ..
        } = attribute;

        self.push(SetAttribute {
            field: attribute.name,
            value,
            ns: attribute.namespace,
            root,
        });
    }

    fn remove_attribute(&mut self, attribute: &Attribute, root: Option<u64>) {
        let Attribute { attribute, .. } = attribute;

        self.push(RemoveAttribute {
            name: attribute.name,
            ns: attribute.namespace,
            root,
        });
    }

    fn clone_node(&mut self, id: Option<u64>, new_id: u64) {
        self.push(CloneNode { id, new_id });
    }

    fn clone_node_children(&mut self, id: Option<u64>, new_ids: Vec<u64>) {
        self.push(CloneNodeChildren { id, new_ids });
    }

    fn first_child(&mut self) {
        self.push(FirstChild {});
    }

    fn next_sibling(&mut self) {
        self.push(NextSibling {});
    }

    fn parent_node(&mut self) {
        self.push(ParentNode {});
    }

    fn store_with_id(&mut self, id: u64) {
        self.push(StoreWithId { id });
    }

    fn set_last_node(&mut self, id: u64) {
        self.push(SetLastNode { id });
    }
}

#[allow(unused)]
impl<'a, E: Edits<'a>> Mutations<'a, E> {
    pub(crate) fn new() -> Self {
        Self {
            edits: E::default(),
            refs: Vec::new(),
            dirty_scopes: Default::default(),
        }
    }

    pub(crate) fn replace_with(&mut self, root: Option<u64>, nodes: Vec<u64>) {
        self.edits.replace_with(root, nodes);
    }

    pub(crate) fn insert_after(&mut self, root: Option<u64>, nodes: Vec<u64>) {
        self.edits.insert_after(root, nodes);
    }

    pub(crate) fn insert_before(&mut self, root: Option<u64>, nodes: Vec<u64>) {
        self.edits.insert_before(root, nodes);
    }

    pub(crate) fn append_children(&mut self, root: Option<u64>, children: Vec<u64>) {
        self.edits.append_children(root, children);
    }

    // Remove Nodes from the dom
    pub(crate) fn remove(&mut self, id: Option<u64>) {
        self.edits.remove(id);
    }

    // Create
    pub(crate) fn create_text_node(&mut self, text: &'a str, id: Option<u64>) {
        self.edits.create_text_node(text, id);
    }

    pub(crate) fn create_element(
        &mut self,
        tag: &'static str,
        ns: Option<&'static str>,
        id: Option<u64>,
        children: u32,
    ) {
        self.edits.create_element(tag, ns, id, children);
    }

    // placeholders are nodes that don't get rendered but still exist as an "anchor" in the real dom
    pub(crate) fn create_placeholder(&mut self, id: Option<u64>) {
        self.edits.create_placeholder(id);
    }

    // events
    pub(crate) fn new_event_listener(&mut self, listener: &Listener, scope: ScopeId) {
        self.edits.new_event_listener(listener, scope);
    }

    pub(crate) fn remove_event_listener(&mut self, event: &'static str, root: Option<u64>) {
        self.edits.remove_event_listener(event, root);
    }

    // modify
    pub(crate) fn set_text(&mut self, text: &'a str, root: Option<u64>) {
        self.edits.set_text(text, root);
    }

    pub(crate) fn set_attribute(&mut self, attribute: &'a Attribute<'a>, root: Option<u64>) {
        self.edits.set_attribute(attribute, root);
    }

    pub(crate) fn remove_attribute(&mut self, attribute: &Attribute, root: Option<u64>) {
        self.edits.remove_attribute(attribute, root);
    }

    pub(crate) fn mark_dirty_scope(&mut self, scope: ScopeId) {
        self.dirty_scopes.insert(scope);
    }

    pub(crate) fn clone_node(&mut self, id: Option<u64>, new_id: u64) {
        self.edits.clone_node(id, new_id);
    }

    pub(crate) fn clone_node_children(&mut self, id: Option<u64>, new_ids: Vec<u64>) {
        self.edits.clone_node_children(id, new_ids);
    }

    pub(crate) fn first_child(&mut self) {
        self.edits.first_child();
    }

    pub(crate) fn next_sibling(&mut self) {
        self.edits.next_sibling();
    }

    pub(crate) fn parent_node(&mut self) {
        self.edits.parent_node();
    }

    pub(crate) fn store_with_id(&mut self, id: u64) {
        self.edits.store_with_id(id);
    }

    pub(crate) fn set_last_node(&mut self, id: u64) {
        self.edits.set_last_node(id);
    }
}

// refs are only assigned once
pub struct NodeRefMutation<'a> {
    pub element: &'a mut Option<once_cell::sync::OnceCell<Box<dyn Any>>>,
    pub element_id: ElementId,
}

impl<'a> std::fmt::Debug for NodeRefMutation<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeRefMutation")
            .field("element_id", &self.element_id)
            .finish()
    }
}

impl<'a> NodeRefMutation<'a> {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.element
            .as_ref()
            .and_then(|f| f.get())
            .and_then(|f| f.downcast_ref::<T>())
    }
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.element
            .as_mut()
            .and_then(|f| f.get_mut())
            .and_then(|f| f.downcast_mut::<T>())
    }
}
