use std::any::Any;

use crate::innerlude::*;

/// The "Mutations" object holds the changes that need to be made to the DOM.
///
#[derive(Debug)]
pub struct Mutations<'s> {
    pub edits: Vec<DomEdit<'s>>,
    pub noderefs: Vec<NodeRefMutation<'s>>,
}
use DomEdit::*;

impl<'bump> Mutations<'bump> {
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
    // replace the n-m node on the stack with the m nodes
    // ends with the last element of the chain on the top of the stack
    pub(crate) fn replace_with(&mut self, n: u32, m: u32) {
        self.edits.push(ReplaceWith { n, m });
    }
    pub(crate) fn insert_after(&mut self, n: u32) {
        self.edits.push(InsertAfter { n });
    }
    pub(crate) fn insert_before(&mut self, n: u32) {
        self.edits.push(InsertBefore { n });
    }
    // Remove Nodesfrom the dom
    pub(crate) fn remove(&mut self) {
        self.edits.push(Remove);
    }
    // Create
    pub(crate) fn create_text_node(&mut self, text: &'bump str, id: ElementId) {
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
    pub(crate) fn set_text(&mut self, text: &'bump str) {
        self.edits.push(SetText { text });
    }
    pub(crate) fn set_attribute(&mut self, attribute: &'bump Attribute) {
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
    pub(crate) fn set_attribute_ns(&mut self, attribute: &'bump Attribute, namespace: &'bump str) {
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
    element: &'a mut Option<once_cell::sync::OnceCell<Box<dyn Any>>>,
    element_id: ElementId,
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
