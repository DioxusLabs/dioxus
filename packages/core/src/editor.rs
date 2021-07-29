//!
//!
//!
//!
//!
//!

use crate::innerlude::ScopeId;

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
