use fxhash::FxHashMap;

use crate::innerlude::{VNode, VText};

/// A Patch encodes an operation that modifies a real DOM element.
///
/// To update the real DOM that a user sees you'll want to first diff your
/// old virtual dom and new virtual dom.
///
/// This diff operation will generate `Vec<Patch>` with zero or more patches that, when
/// applied to your real DOM, will make your real DOM look like your new virtual dom.
///
/// Each Patch has a u32 node index that helps us identify the real DOM node that it applies to.
///
/// Our old virtual dom's nodes are indexed depth first, as shown in this illustration
/// (0 being the root node, 1 being it's first child, 2 being it's first child's first child).
///
/// ```text
///             .─.
///            ( 0 )
///             `┬'
///         ┌────┴──────┐
///         │           │
///         ▼           ▼
///        .─.         .─.
///       ( 1 )       ( 4 )
///        `┬'         `─'
///    ┌────┴───┐       ├─────┬─────┐
///    │        │       │     │     │
///    ▼        ▼       ▼     ▼     ▼
///   .─.      .─.     .─.   .─.   .─.
///  ( 2 )    ( 3 )   ( 5 ) ( 6 ) ( 7 )
///   `─'      `─'     `─'   `─'   `─'                  
/// ```
///
/// The patching process is tested in a real browser in crates/virtual-dom-rs/tests/diff_patch.rs

pub enum Patch<'a> {
    /// Append a vector of child nodes to a parent node id.
    AppendChildren(NodeIdx, Vec<&'a VNode<'a>>),

    /// For a `node_i32`, remove all children besides the first `len`
    TruncateChildren(NodeIdx, usize),

    /// Replace a node with another node. This typically happens when a node's tag changes.
    /// ex: <div> becomes <span>
    Replace(NodeIdx, &'a VNode<'a>),

    /// Add attributes that the new node has that the old node does not
    AddAttributes(NodeIdx, FxHashMap<&'a str, &'a str>),

    /// Remove attributes that the old node had that the new node doesn't
    RemoveAttributes(NodeIdx, Vec<&'a str>),

    /// Change the text of a Text node.
    ChangeText(NodeIdx, &'a VText<'a>),
}

type NodeIdx = usize;

impl<'a> Patch<'a> {
    /// Every Patch is meant to be applied to a specific node within the DOM. Get the
    /// index of the DOM node that this patch should apply to. DOM nodes are indexed
    /// depth first with the root node in the tree having index 0.
    pub fn node_idx(&self) -> usize {
        match self {
            Patch::AppendChildren(node_idx, _) => *node_idx,
            Patch::TruncateChildren(node_idx, _) => *node_idx,
            Patch::Replace(node_idx, _) => *node_idx,
            Patch::AddAttributes(node_idx, _) => *node_idx,
            Patch::RemoveAttributes(node_idx, _) => *node_idx,
            Patch::ChangeText(node_idx, _) => *node_idx,
        }
    }
}

pub struct PatchList<'a> {
    patches: Vec<Patch<'a>>,
}
