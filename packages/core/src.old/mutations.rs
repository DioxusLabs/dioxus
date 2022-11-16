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
    /// Remove all the children of the given element
    fn remove_children(&mut self, root: ElementId);

    /// Attach a new listener to the dom
    fn new_event_listener(&mut self, listener: &Listener, scope: ScopeId);
    /// Remove an existing listener from the dom
    fn remove_event_listener(&mut self, event: &'static str, root: ElementId);

    /// Set the text content of a node
    fn set_text(&mut self, text: &'a str, root: ElementId);
    /// Set an attribute on an element
    fn set_attribute(
        &mut self,
        name: &'static str,
        value: AttributeValue<'a>,
        namespace: Option<&'a str>,
        root: ElementId,
    );

    /// General statistics for doing things that extend outside of the renderer
    fn mark_dirty_scope(&mut self, scope: ScopeId);

    /// Save the current n nodes to the ID to be loaded later
    fn save(&mut self, id: &'static str, num: u32);
    /// Loads a set of saved nodes from the ID into a scratch space
    fn load(&mut self, id: &'static str, index: u32);
    /// Assign the element on the stack's descendent the given ID
    fn assign_id(&mut self, descendent: &'static [u8], id: ElementId);
    /// Replace the given element of the topmost element with the next m elements on the stack
    /// Is essentially a combination of assign_id and replace_with
    fn replace_descendant(&mut self, descendent: &'static [u8], m: u32);
}

/*
div {
    div {
        div {
            div {}
        }
    }
}

push_child(0)
push_child(1)
push_child(3)
push_child(4)
pop
pop

clone_node(0)
set_node(el, [1,2,3,4])
set_attribute("class", "foo")
append_child(1)
*/


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
pub struct Mutations<'a> {
    /// The list of edits that need to be applied for the RealDOM to match the VirtualDOM.
    pub edits: Vec<DomEdit<'a>>,

    /// The list of Scopes that were diffed, created, and removed during the Diff process.
    pub dirty_scopes: FxHashSet<ScopeId>,

    /// The list of nodes to connect to the RealDOM.
    pub refs: Vec<NodeRefMutation<'a>>,
}

impl Debug for Mutations<'_> {
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
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type")
)]
pub enum DomEdit<'bump> {
    /// Pop the topmost node from our stack and append them to the node
    /// at the top of the stack.
    AppendChildren {
        /// The parent to append nodes to.
        root: Option<u64>,

        /// The ids of the children to append.
        children: Vec<u64>,
    },

    /// Replace a given (single) node with a handful of nodes currently on the stack.
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
        value: AttributeValue<'bump>,

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
use DomEdit::*;

#[allow(unused)]
impl<'a> Mutations<'a> {
    pub(crate) fn new() -> Self {
        Self {
            edits: Vec::new(),
            refs: Vec::new(),
            dirty_scopes: Default::default(),
        }
    }

    pub(crate) fn replace_with(&mut self, root: Option<u64>, nodes: Vec<u64>) {
        self.edits.push(ReplaceWith { nodes, root });
    }

    pub(crate) fn insert_after(&mut self, root: Option<u64>, nodes: Vec<u64>) {
        self.edits.push(InsertAfter { nodes, root });
    }

    pub(crate) fn insert_before(&mut self, root: Option<u64>, nodes: Vec<u64>) {
        self.edits.push(InsertBefore { nodes, root });
    }

    pub(crate) fn append_children(&mut self, root: Option<u64>, children: Vec<u64>) {
        self.edits.push(AppendChildren { root, children });
    }

    // Remove Nodes from the dom
    pub(crate) fn remove(&mut self, id: Option<u64>) {
        self.edits.push(Remove { root: id });
    }

    // Create
    pub(crate) fn create_text_node(&mut self, text: &'a str, id: Option<u64>) {
        self.edits.push(CreateTextNode { text, root: id });
    }

    pub(crate) fn create_element(
        &mut self,
        tag: &'static str,
        ns: Option<&'static str>,
        id: Option<u64>,
        children: u32,
    ) {
        match ns {
            Some(ns) => self.edits.push(CreateElementNs {
                root: id,
                ns,
                tag,
                children,
            }),
            None => self.edits.push(CreateElement {
                root: id,
                tag,
                children,
            }),
        }
    }

    // placeholders are nodes that don't get rendered but still exist as an "anchor" in the real dom
    pub(crate) fn create_placeholder(&mut self, id: Option<u64>) {
        self.edits.push(CreatePlaceholder { root: id });
    }

    // events
    pub(crate) fn new_event_listener(&mut self, listener: &Listener, scope: ScopeId) {
        let Listener {
            event,
            mounted_node,
            ..
        } = listener;

        let element_id = Some(mounted_node.get().unwrap().into());

        self.edits.push(NewEventListener {
            scope,
            event_name: event,
            root: element_id,
        });
    }

    pub(crate) fn remove_event_listener(&mut self, event: &'static str, root: Option<u64>) {
        self.edits.push(RemoveEventListener { event, root });
    }

    // modify
    pub(crate) fn set_text(&mut self, text: &'a str, root: Option<u64>) {
        self.edits.push(SetText { text, root });
    }

    pub(crate) fn set_attribute(&mut self, attribute: &'a Attribute<'a>, root: Option<u64>) {
        let Attribute {
            value, attribute, ..
        } = attribute;

        self.edits.push(SetAttribute {
            field: attribute.name,
            value: value.clone(),
            ns: attribute.namespace,
            root,
        });
    }

    pub(crate) fn remove_attribute(&mut self, attribute: &Attribute, root: Option<u64>) {
        let Attribute { attribute, .. } = attribute;

        self.edits.push(RemoveAttribute {
            name: attribute.name,
            ns: attribute.namespace,
            root,
        });
    }

    pub(crate) fn mark_dirty_scope(&mut self, scope: ScopeId) {
        self.dirty_scopes.insert(scope);
    }

    pub(crate) fn clone_node(&mut self, id: Option<u64>, new_id: u64) {
        self.edits.push(CloneNode { id, new_id });
    }

    pub(crate) fn clone_node_children(&mut self, id: Option<u64>, new_ids: Vec<u64>) {
        self.edits.push(CloneNodeChildren { id, new_ids });
    }

    pub(crate) fn first_child(&mut self) {
        self.edits.push(FirstChild {});
    }

    pub(crate) fn next_sibling(&mut self) {
        self.edits.push(NextSibling {});
    }

    pub(crate) fn parent_node(&mut self) {
        self.edits.push(ParentNode {});
    }

    pub(crate) fn store_with_id(&mut self, id: u64) {
        self.edits.push(StoreWithId { id });
    }

    pub(crate) fn set_last_node(&mut self, id: u64) {
        self.edits.push(SetLastNode { id });
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
