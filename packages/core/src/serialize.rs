//! Serialization
//! -------------
//!
//!
//!
//!
//!
//!

use crate::{innerlude::ScopeIdx, RealDomNode};
use serde::{Deserialize, Serialize};

/// A `DomEdit` represents a serialzied form of the VirtualDom's trait-based API. This allows streaming edits across the
/// network or through FFI boundaries.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DomEdit<'bump> {
    PushRoot {
        root: RealDomNode,
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
        id: RealDomNode,
    },
    CreateElement {
        tag: &'bump str,
        id: RealDomNode,
    },
    CreateElementNs {
        tag: &'bump str,
        id: RealDomNode,
        ns: &'static str,
    },
    CreatePlaceholder {
        id: RealDomNode,
    },
    NewEventListener {
        event: &'static str,
        scope: ScopeIdx,
        node: RealDomNode,
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
