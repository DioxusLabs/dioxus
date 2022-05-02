//! Public interface for the internal state of the router.

use std::collections::{BTreeMap, BTreeSet};

use dioxus_core::Component;

/// The current routing information.
#[derive(Default)]
pub struct CurrentRoute {
    /// Whether there is a prior path to go back to.
    pub can_go_back: bool,

    /// Whether there is a future path resulting from a "go back" operation that can be reapplied.
    pub can_go_forward: bool,

    /// The components specified by the active routes.
    pub(crate) components: Vec<(Component, BTreeMap<&'static str, Component>)>,

    /// The names of the currently active routes.
    pub names: BTreeSet<&'static str>,

    /// The current path.
    pub path: String,

    /// The current query string, if present.
    pub query: Option<String>,

    /// The variables read from the path as specified by the current routes.
    pub variables: BTreeMap<&'static str, String>,
}

impl CurrentRoute {
    /// Get the query parameters as a [`BTreeMap`].
    pub fn query_params(&self) -> Option<BTreeMap<String, String>> {
        if let Some(query) = &self.query {
            serde_urlencoded::from_str(query).ok()
        } else {
            None
        }
    }
}
