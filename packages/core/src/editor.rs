//!
//!
//!
//!
//!
//!

use crate::{innerlude::ScopeId, ElementId};

/// The `DomEditor` provides an imperative interface for the Diffing algorithm to plan out its changes.
///
/// However, the DomEditor only builds a change list - it does not apply them. In contrast with the "RealDom", the DomEditor
/// is cancellable and flushable. At any moment in time, Dioxus may choose to completely clear the edit list and start over.
///
/// This behavior is used in the cooperative scheduling algorithm.
pub struct DomEditor<'real, 'bump> {
    pub edits: &'real mut Vec<DomEdit<'bump>>,
}

use DomEdit::*;
impl<'real, 'bump> DomEditor<'real, 'bump> {
    pub fn new(edits: &'real mut Vec<DomEdit<'bump>>) -> Self {
        Self { edits }
    }

    // Navigation
    pub(crate) fn push_root(&mut self, root: ElementId) {
        let id = root.as_u64();
        self.edits.push(PushRoot { id });
    }

    #[inline]
    pub(crate) fn pop(&mut self) {
        self.edits.push(PopRoot {});
    }

    // Add Nodes to the dom
    // add m nodes from the stack
    #[inline]
    pub(crate) fn append_children(&mut self, many: u32) {
        self.edits.push(AppendChildren { many });
    }

    // replace the n-m node on the stack with the m nodes
    // ends with the last element of the chain on the top of the stack
    #[inline]
    pub(crate) fn replace_with(&mut self, many: u32) {
        self.edits.push(ReplaceWith { many });
    }

    // Remove Nodesfrom the dom
    #[inline]
    pub(crate) fn remove(&mut self) {
        self.edits.push(Remove);
    }

    #[inline]
    pub(crate) fn remove_all_children(&mut self) {
        self.edits.push(RemoveAllChildren);
    }

    // Create
    #[inline]
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
    pub(crate) fn new_event_listener(
        &mut self,
        event: &'static str,
        scope: ScopeId,
        element_id: usize,
        realnode: ElementId,
    ) {
        self.edits.push(NewEventListener {
            scope,
            event_name: event,
            element_id,
            mounted_node_id: realnode.as_u64(),
        });
    }

    #[inline]
    pub(crate) fn remove_event_listener(&mut self, event: &'static str) {
        self.edits.push(RemoveEventListener { event });
    }

    // modify
    #[inline]
    pub(crate) fn set_text(&mut self, text: &'bump str) {
        self.edits.push(SetText { text });
    }

    #[inline]
    pub(crate) fn set_attribute(
        &mut self,
        field: &'static str,
        value: &'bump str,
        ns: Option<&'static str>,
    ) {
        self.edits.push(SetAttribute { field, value, ns });
    }

    #[inline]
    pub(crate) fn remove_attribute(&mut self, name: &'static str) {
        self.edits.push(RemoveAttribute { name });
    }
}

/// A `DomEdit` represents a serialzied form of the VirtualDom's trait-based API. This allows streaming edits across the
/// network or through FFI boundaries.
#[derive(Debug)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type")
)]
pub enum DomEdit<'bump> {
    PushRoot {
        id: u64,
    },
    PopRoot,
    AppendChildren {
        many: u32,
    },
    ReplaceWith {
        many: u32,
    },
    Remove,
    RemoveAllChildren,
    CreateTextNode {
        text: &'bump str,
        id: u64,
    },
    CreateElement {
        tag: &'bump str,
        id: u64,
    },
    CreateElementNs {
        tag: &'bump str,
        id: u64,
        ns: &'static str,
    },
    CreatePlaceholder {
        id: u64,
    },
    NewEventListener {
        event_name: &'static str,
        scope: ScopeId,
        mounted_node_id: u64,
        element_id: usize,
    },
    RemoveEventListener {
        event: &'static str,
    },
    SetText {
        text: &'bump str,
    },
    SetAttribute {
        field: &'static str,
        value: &'bump str,
        ns: Option<&'bump str>,
    },
    RemoveAttribute {
        name: &'static str,
    },
}
