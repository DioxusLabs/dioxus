//! Instructions returned by the VirtualDOM on how to modify the Real DOM.
//!
//! This module contains an internal API to generate these instructions.

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
    pub edits: Vec<DomEdit<'a>>,
    pub dirty_scopes: FxHashSet<ScopeId>,
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
    PushRoot {
        root: u64,
    },
    PopRoot,

    AppendChildren {
        many: u32,
    },

    // // save a possibly-fragment node as a template
    // SaveAsTemplate {
    //     many: u32,
    // },

    // "Root" refers to the item directly
    // it's a waste of an instruction to push the root directly
    ReplaceWith {
        root: u64,
        m: u32,
    },
    InsertAfter {
        root: u64,
        n: u32,
    },
    InsertBefore {
        root: u64,
        n: u32,
    },
    Remove {
        root: u64,
    },
    CreateTextNode {
        text: &'bump str,
        root: u64,
    },
    CreateElement {
        tag: &'bump str,
        root: u64,
    },
    CreateElementNs {
        tag: &'bump str,
        root: u64,
        ns: &'static str,
    },
    CreatePlaceholder {
        root: u64,
    },
    NewEventListener {
        event_name: &'static str,
        scope: ScopeId,
        root: u64,
    },
    RemoveEventListener {
        root: u64,
        event: &'static str,
    },
    SetText {
        root: u64,
        text: &'bump str,
    },
    SetAttribute {
        root: u64,
        field: &'static str,
        value: &'bump str,
        ns: Option<&'bump str>,
    },
    RemoveAttribute {
        root: u64,
        name: &'static str,
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
    pub(crate) fn push_root(&mut self, root: ElementId) {
        let id = root.as_u64();
        self.edits.push(PushRoot { root: id });
    }

    pub(crate) fn replace_with(&mut self, root: ElementId, m: u32) {
        let root = root.as_u64();
        self.edits.push(ReplaceWith { m, root });
    }

    pub(crate) fn insert_after(&mut self, root: ElementId, n: u32) {
        let root = root.as_u64();
        self.edits.push(InsertAfter { n, root });
    }

    pub(crate) fn insert_before(&mut self, root: ElementId, n: u32) {
        let root = root.as_u64();
        self.edits.push(InsertBefore { n, root });
    }

    // Remove Nodes from the dom
    pub(crate) fn remove(&mut self, id: u64) {
        self.edits.push(Remove { root: id });
    }

    // Create
    pub(crate) fn create_text_node(&mut self, text: &'a str, id: ElementId) {
        let id = id.as_u64();
        self.edits.push(CreateTextNode { text, root: id });
    }

    pub(crate) fn create_element(
        &mut self,
        tag: &'static str,
        ns: Option<&'static str>,
        id: ElementId,
    ) {
        let id = id.as_u64();
        match ns {
            Some(ns) => self.edits.push(CreateElementNs { root: id, ns, tag }),
            None => self.edits.push(CreateElement { root: id, tag }),
        }
    }
    // placeholders are nodes that don't get rendered but still exist as an "anchor" in the real dom
    pub(crate) fn create_placeholder(&mut self, id: ElementId) {
        let id = id.as_u64();
        self.edits.push(CreatePlaceholder { root: id });
    }

    // events
    pub(crate) fn new_event_listener(&mut self, listener: &Listener, scope: ScopeId) {
        let Listener {
            event,
            mounted_node,
            ..
        } = listener;

        let element_id = mounted_node.get().unwrap().as_u64();

        self.edits.push(NewEventListener {
            scope,
            event_name: event,
            root: element_id,
        });
    }
    pub(crate) fn remove_event_listener(&mut self, event: &'static str, root: u64) {
        self.edits.push(RemoveEventListener { event, root });
    }

    // modify
    pub(crate) fn set_text(&mut self, text: &'a str, root: u64) {
        self.edits.push(SetText { text, root });
    }

    pub(crate) fn set_attribute(&mut self, attribute: &'a Attribute, root: u64) {
        let Attribute {
            name,
            value,
            namespace,
            ..
        } = attribute;

        self.edits.push(SetAttribute {
            field: name,
            value,
            ns: *namespace,
            root,
        });
    }

    pub(crate) fn remove_attribute(&mut self, attribute: &Attribute, root: u64) {
        let name = attribute.name;
        self.edits.push(RemoveAttribute { name, root });
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
