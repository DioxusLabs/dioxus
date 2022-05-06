use std::sync::Arc;

use dioxus_core::ScopeState;

use crate::route_definition::Segment;

/// A hook that makes constructing the [`Segment`] for a [`Router`] easier.
///
/// [`Router`]: crate::components::Router
pub fn use_segment(cx: &ScopeState, init: impl FnOnce() -> Segment) -> &'_ Arc<Segment> {
    cx.use_hook(|_| Arc::new(init()))
}
