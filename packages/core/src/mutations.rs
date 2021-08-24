//! Instructions returned by the VirtualDOM on how to modify the Real DOM.
//!

use crate::innerlude::*;
use std::any::Any;

#[derive(Debug)]
pub struct Mutations<'a> {
    pub edits: Vec<DomEdit<'a>>,
    pub noderefs: Vec<NodeRefMutation<'a>>,
}

use DomEdit::*;

impl<'a> Mutations<'a> {
    pub fn new() -> Self {
        let edits = Vec::new();
        let noderefs = Vec::new();
        Self { edits, noderefs }
    }

    pub fn extend(&mut self, other: &mut Mutations) {}

    // Navigation
    pub(crate) fn push_root(&mut self, root: ElementId) {
        let id = root.as_u64();
        self.edits.push(PushRoot { id });
    }

    pub(crate) fn pop(&mut self) {
        self.edits.push(PopRoot {});
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

    // Remove Nodesfrom the dom
    pub(crate) fn remove(&mut self, id: u64) {
        self.edits.push(Remove { root: id });
    }

    // Create
    pub(crate) fn create_text_node(&mut self, text: &'a str, id: ElementId) {
        let id = id.as_u64();
        self.edits.push(CreateTextNode { text, id });
    }

    pub(crate) fn create_element(
        &mut self,
        tag: &'static str,
        ns: Option<&'static str>,
        id: ElementId,
    ) {
        let id = id.as_u64();
        match ns {
            Some(ns) => self.edits.push(CreateElementNs { id, ns, tag }),
            None => self.edits.push(CreateElement { id, tag }),
        }
    }
    // placeholders are nodes that don't get rendered but still exist as an "anchor" in the real dom
    pub(crate) fn create_placeholder(&mut self, id: ElementId) {
        let id = id.as_u64();
        self.edits.push(CreatePlaceholder { id });
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
            mounted_node_id: element_id,
        });
    }
    pub(crate) fn remove_event_listener(&mut self, event: &'static str) {
        self.edits.push(RemoveEventListener { event });
    }

    // modify
    pub(crate) fn set_text(&mut self, text: &'a str) {
        self.edits.push(SetText { text });
    }

    pub(crate) fn set_attribute(&mut self, attribute: &'a Attribute) {
        let Attribute {
            name,
            value,
            is_static,
            is_volatile,
            namespace,
        } = attribute;

        self.edits.push(SetAttribute {
            field: name,
            value,
            ns: *namespace,
        });
    }
    pub(crate) fn set_attribute_ns(&mut self, attribute: &'a Attribute, namespace: &'a str) {
        let Attribute {
            name,
            value,
            is_static,
            is_volatile,
            ..
        } = attribute;

        self.edits.push(SetAttribute {
            field: name,
            value,
            ns: Some(namespace),
        });
    }

    pub(crate) fn remove_attribute(&mut self, attribute: &Attribute) {
        let name = attribute.name;
        self.edits.push(RemoveAttribute { name });
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
