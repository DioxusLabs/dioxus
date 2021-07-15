//! Serialization
//! -------------
//!
//!
//!
//!
//!
//!

use crate::innerlude::ScopeIdx;
use serde::{Deserialize, Serialize};

/// A `DomEdit` represents a serialzied form of the VirtualDom's trait-based API. This allows streaming edits across the
/// network or through FFI boundaries.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
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
        event: &'static str,
        scope: ScopeIdx,
        node: u64,
        idx: usize,
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
