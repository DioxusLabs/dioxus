//! A fast in-memory renderer for validating Dioxus mutation streams.
//!
//! `RendererOracle` implements [`dioxus_core::WriteMutations`] and maintains a
//! compact mock DOM. It is intended for tests and fuzzers that need renderer
//! semantics without webviews, JS bindings, layout, or serialization.

mod diagnostics;
mod renderer;
mod snapshot;
mod vdom_snapshot;

pub use diagnostics::panic_message;
pub use renderer::{EditSummary, OracleNodeId, RendererOracle};
pub use snapshot::{SnapshotAttr, SnapshotNode};

#[cfg(test)]
mod tests;
