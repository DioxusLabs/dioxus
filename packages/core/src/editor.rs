//!
//!
//!
//!
//!
//!

use crate::innerlude::ScopeId;

/// A `DomEdit` represents a serialzied form of the VirtualDom's trait-based API. This allows streaming edits across the
/// network or through FFI boundaries.
#[derive(Debug, PartialEq)]
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
        // the first n elements
        n: u32,

        // the last m elements
        m: u32,
    },
    InsertAfter {
        n: u32,
    },
    InsertBefore {
        n: u32,
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
impl DomEdit<'_> {
    pub fn is(&self, id: &'static str) -> bool {
        match self {
            DomEdit::InsertAfter { .. } => id == "InsertAfter",
            DomEdit::InsertBefore { .. } => id == "InsertBefore",
            DomEdit::PushRoot { .. } => id == "PushRoot",
            DomEdit::PopRoot => id == "PopRoot",
            DomEdit::AppendChildren { .. } => id == "AppendChildren",
            DomEdit::ReplaceWith { .. } => id == "ReplaceWith",
            DomEdit::Remove => id == "Remove",
            DomEdit::RemoveAllChildren => id == "RemoveAllChildren",
            DomEdit::CreateTextNode { .. } => id == "CreateTextNode",
            DomEdit::CreateElement { .. } => id == "CreateElement",
            DomEdit::CreateElementNs { .. } => id == "CreateElementNs",
            DomEdit::CreatePlaceholder { .. } => id == "CreatePlaceholder",
            DomEdit::NewEventListener { .. } => id == "NewEventListener",
            DomEdit::RemoveEventListener { .. } => id == "RemoveEventListener",
            DomEdit::SetText { .. } => id == "SetText",
            DomEdit::SetAttribute { .. } => id == "SetAttribute",
            DomEdit::RemoveAttribute { .. } => id == "RemoveAttribute",
        }
    }
}
