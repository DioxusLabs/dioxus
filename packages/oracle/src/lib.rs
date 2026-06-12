//! A fast in-memory renderer for validating Dioxus mutation streams.
//!
//! `RendererOracle` implements [`dioxus_core::WriteMutations`] and maintains a
//! compact mock DOM. It is intended for tests and fuzzers that need renderer
//! semantics without webviews, JS bindings, layout, or serialization.

mod renderer;
mod sequence;
mod snapshot;
mod vdom_snapshot;

pub use renderer::{EditSummary, EventListenerTarget, OracleNodeId, RendererOracle};
pub use sequence::Sequence;
pub use snapshot::{SnapshotAttr, SnapshotNode};
pub use vdom_snapshot::{
    assert_fresh_snapshot_eq, assert_immediate_matches_fresh, assert_no_mutations, fresh_snapshot,
    panic_message, render_immediate_snapshot, vdom_snapshot,
};

/// Backwards-compatible name for callers that want a plain mock renderer.
pub type MockRenderer = RendererOracle;

/// Backwards-compatible name for the renderer's stable structural snapshot.
pub type Canonical = SnapshotNode;

#[cfg(test)]
mod tests;
