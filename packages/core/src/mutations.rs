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
    /// Push the given root node onto our stack.
    PushRoot {
        /// The ID of the root node to push.
        root: u64,
    },

    /// Pop the topmost node from our stack and append them to the node
    /// at the top of the stack.
    AppendChildren {
        /// How many nodes should be popped from the stack.
        /// The node remaining on the stack will be the target for the append.
        many: u32,
    },

    /// Replace a given (single) node with a handful of nodes currently on the stack.
    ReplaceWith {
        /// The ID of the node to be replaced.
        root: u64,

        /// How many nodes should be popped from the stack to replace the target node.
        m: u32,
    },

    /// Insert a number of nodes after a given node.
    InsertAfter {
        /// The ID of the node to insert after.
        root: u64,

        /// How many nodes should be popped from the stack to insert after the target node.
        n: u32,
    },

    /// Insert a number of nodes before a given node.
    InsertBefore {
        /// The ID of the node to insert before.
        root: u64,

        /// How many nodes should be popped from the stack to insert before the target node.
        n: u32,
    },

    /// Remove a particular node from the DOM
    Remove {
        /// The ID of the node to remove.
        root: u64,
    },

    /// Create a new purely-text node
    CreateTextNode {
        /// The ID the new node should have.
        root: u64,

        /// The textcontent of the node
        text: &'bump str,
    },

    /// Create a new purely-element node
    CreateElement {
        /// The ID the new node should have.
        root: u64,

        /// The tagname of the node
        tag: &'bump str,
    },

    /// Create a new purely-comment node with a given namespace
    CreateElementNs {
        /// The ID the new node should have.
        root: u64,

        /// The namespace of the node
        tag: &'bump str,

        /// The namespace of the node (like `SVG`)
        ns: &'static str,
    },

    /// Create a new placeholder node.
    /// In most implementations, this will either be a hidden div or a comment node.
    CreatePlaceholder {
        /// The ID the new node should have.
        root: u64,
    },

    /// Create a new purely-text node in a template
    CreateTextNodeTemplate {
        /// The ID the new node should have.
        root: u64,

        /// The textcontent of the noden
        text: &'bump str,

        /// If the id of the node must be kept in the refrences
        locally_static: bool,
    },

    /// Create a new purely-element node in a template
    CreateElementTemplate {
        /// The ID the new node should have.
        root: u64,

        /// The tagname of the node
        tag: &'bump str,

        /// If the id of the node must be kept in the refrences
        locally_static: bool,

        /// If any children of this node must be kept in the references
        fully_static: bool,
    },

    /// Create a new purely-comment node with a given namespace in a template
    CreateElementNsTemplate {
        /// The ID the new node should have.
        root: u64,

        /// The namespace of the node
        tag: &'bump str,

        /// The namespace of the node (like `SVG`)
        ns: &'static str,

        /// If the id of the node must be kept in the refrences
        locally_static: bool,

        /// If any children of this node must be kept in the references
        fully_static: bool,
    },

    /// Create a new placeholder node.
    /// In most implementations, this will either be a hidden div or a comment node. in a template
    /// Always both locally and fully static
    CreatePlaceholderTemplate {
        /// The ID the new node should have.
        root: u64,
    },

    /// Create a new Event Listener.
    NewEventListener {
        /// The name of the event to listen for.
        event_name: &'static str,

        /// The ID of the node to attach the listener to.
        scope: ScopeId,

        /// The ID of the node to attach the listener to.
        root: u64,
    },

    /// Remove an existing Event Listener.
    RemoveEventListener {
        /// The ID of the node to remove.
        root: u64,

        /// The name of the event to remove.
        event: &'static str,
    },

    /// Set the textcontent of a node.
    SetText {
        /// The ID of the node to set the textcontent of.
        root: u64,

        /// The textcontent of the node
        text: &'bump str,
    },

    /// Set the value of a node's attribute.
    SetAttribute {
        /// The ID of the node to set the attribute of.
        root: u64,

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
        root: u64,

        /// The name of the attribute to remove.
        name: &'static str,

        /// The namespace of the attribute.
        ns: Option<&'bump str>,
    },

    /// Manually pop a root node from the stack.
    PopRoot {},

    /// Enter a TemplateRef tree
    EnterTemplateRef {
        /// The ID of the node to enter.
        root: u64,
    },

    /// Exit a TemplateRef tree
    ExitTemplateRef {},

    /// Create a refrence to a template node.
    CreateTemplateRef {
        /// The ID of the new template refrence.
        id: u64,

        /// The ID of the template the node is refrencing.
        template_id: u64,
    },

    /// Create a new templete.
    /// IMPORTANT: When adding nodes to a templete, id's will reset to zero, so they must be allocated on a different stack.
    /// It is recommended to use Cow<NativeNode>.
    CreateTemplate {
        /// The ID of the new template.
        id: u64,
    },

    /// Finish a templete
    FinishTemplate {
        /// The number of root nodes in the template
        len: u32,
    },
}

use fxhash::FxHashSet;
use DomEdit::*;

impl<'a> Mutations<'a> {
    pub(crate) fn new() -> Self {
        Self {
            edits: Vec::new(),
            refs: Vec::new(),
            dirty_scopes: Default::default(),
        }
    }

    // Navigation
    pub(crate) fn push_root(&mut self, root: impl Into<u64>) {
        let id = root.into();
        self.edits.push(PushRoot { root: id });
    }

    // Navigation
    pub(crate) fn pop_root(&mut self) {
        self.edits.push(PopRoot {});
    }

    pub(crate) fn replace_with(&mut self, root: impl Into<u64>, m: u32) {
        let root = root.into();
        self.edits.push(ReplaceWith { m, root });
    }

    pub(crate) fn insert_after(&mut self, root: impl Into<u64>, n: u32) {
        let root = root.into();
        self.edits.push(InsertAfter { n, root });
    }

    pub(crate) fn insert_before(&mut self, root: impl Into<u64>, n: u32) {
        let root = root.into();
        self.edits.push(InsertBefore { n, root });
    }

    pub(crate) fn append_children(&mut self, n: u32) {
        self.edits.push(AppendChildren { many: n });
    }

    // Remove Nodes from the dom
    pub(crate) fn remove(&mut self, id: impl Into<u64>) {
        self.edits.push(Remove { root: id.into() });
    }

    // Create
    pub(crate) fn create_text_node(&mut self, text: &'a str, id: impl Into<u64>) {
        let id = id.into();
        self.edits.push(CreateTextNode { text, root: id });
    }

    pub(crate) fn create_element(
        &mut self,
        tag: &'static str,
        ns: Option<&'static str>,
        id: impl Into<u64>,
    ) {
        let id = id.into();
        match ns {
            Some(ns) => self.edits.push(CreateElementNs { root: id, ns, tag }),
            None => self.edits.push(CreateElement { root: id, tag }),
        }
    }

    // placeholders are nodes that don't get rendered but still exist as an "anchor" in the real dom
    pub(crate) fn create_placeholder(&mut self, id: impl Into<u64>) {
        let id = id.into();
        self.edits.push(CreatePlaceholder { root: id });
    }

    // Create
    pub(crate) fn create_text_node_template(
        &mut self,
        text: &'a str,
        id: impl Into<u64>,
        locally_static: bool,
    ) {
        let id = id.into();
        self.edits.push(CreateTextNodeTemplate {
            text,
            root: id,
            locally_static,
        });
    }

    pub(crate) fn create_element_template(
        &mut self,
        tag: &'static str,
        ns: Option<&'static str>,
        id: impl Into<u64>,
        locally_static: bool,
        fully_static: bool,
    ) {
        let id = id.into();
        match ns {
            Some(ns) => self.edits.push(CreateElementNsTemplate {
                root: id,
                ns,
                tag,
                locally_static,
                fully_static,
            }),
            None => self.edits.push(CreateElementTemplate {
                root: id,
                tag,
                locally_static,
                fully_static,
            }),
        }
    }

    // placeholders are nodes that don't get rendered but still exist as an "anchor" in the real dom
    pub(crate) fn create_placeholder_template(&mut self, id: impl Into<u64>) {
        let id = id.into();
        self.edits.push(CreatePlaceholderTemplate { root: id });
    }

    // events
    pub(crate) fn new_event_listener(&mut self, listener: &Listener, scope: ScopeId) {
        let Listener {
            event,
            mounted_node,
            ..
        } = listener;

        let element_id = match mounted_node.get().unwrap() {
            GlobalNodeId::TemplateId {
                template_ref_id: _,
                template_node_id,
            } => template_node_id.into(),
            GlobalNodeId::VNodeId(id) => id.into(),
        };

        self.edits.push(NewEventListener {
            scope,
            event_name: event,
            root: element_id,
        });
    }

    pub(crate) fn remove_event_listener(&mut self, event: &'static str, root: impl Into<u64>) {
        self.edits.push(RemoveEventListener {
            event,
            root: root.into(),
        });
    }

    // modify
    pub(crate) fn set_text(&mut self, text: &'a str, root: impl Into<u64>) {
        let root = root.into();
        self.edits.push(SetText { text, root });
    }

    pub(crate) fn set_attribute(&mut self, attribute: &'a Attribute<'a>, root: impl Into<u64>) {
        let root = root.into();
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

    pub(crate) fn remove_attribute(&mut self, attribute: &Attribute, root: impl Into<u64>) {
        let root = root.into();
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

    pub(crate) fn create_templete(&mut self, id: impl Into<u64>) {
        self.edits.push(CreateTemplate { id: id.into() });
    }

    pub(crate) fn finish_templete(&mut self, len: u32) {
        self.edits.push(FinishTemplate { len });
    }

    pub(crate) fn create_template_ref(&mut self, id: impl Into<u64>, template_id: u64) {
        self.edits.push(CreateTemplateRef {
            id: id.into(),
            template_id,
        })
    }

    pub(crate) fn enter_template_ref(&mut self, id: impl Into<u64>) {
        self.edits.push(EnterTemplateRef { root: id.into() });
    }

    pub(crate) fn exit_template_ref(&mut self) {
        if let Some(&DomEdit::EnterTemplateRef { .. }) = self.edits.last() {
            self.edits.pop();
        } else {
            self.edits.push(ExitTemplateRef {});
        }
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
