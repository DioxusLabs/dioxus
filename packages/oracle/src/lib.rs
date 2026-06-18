//! A fast in-memory renderer for validating Dioxus mutation streams.
//!
//! `RendererOracle` implements [`dioxus_core::WriteMutations`] and maintains a
//! compact mock DOM. It is intended for tests and fuzzers that need renderer
//! semantics without webviews, JS bindings, layout, or serialization.

mod renderer;
#[cfg(test)]
mod sequence;
mod snapshot;
mod vdom_snapshot;

pub use renderer::{EditSummary, EventListenerTarget, OracleNodeId, RendererOracle};
pub use snapshot::{SnapshotAttr, SnapshotNode};
pub use vdom_snapshot::fresh_snapshot;

#[cfg(test)]
mod tests;
