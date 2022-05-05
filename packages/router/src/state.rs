use std::collections::{BTreeMap, BTreeSet};

use dioxus_core::Component;

/// The current routing information.
#[derive(Default)]
pub struct RouterState {
    /// Whether the service can handle external navigation targets.
    pub can_external: bool,

    /// Whether there is a prior path to go back to.
    pub can_go_back: bool,

    /// Whether there is a later path to forward to.
    pub can_go_forward: bool,

    /// The components specified by the active routes.
    pub(crate) components: (Vec<Component>, BTreeMap<&'static str, Vec<Component>>),

    /// The names of the currently active routes.
    pub names: BTreeSet<&'static str>,

    /// The current path.
    pub path: String,

    /// The current prefix.
    pub prefix: String,

    /// The current query string, if present.
    pub query: Option<String>,

    /// The parameters read from the path as specified by the current routes.
    pub parameters: BTreeMap<&'static str, String>,
}

impl RouterState {
    /// Get the query parameters as a [`BTreeMap`].
    #[must_use]
    pub fn query_params(&self) -> Option<BTreeMap<String, String>> {
        if let Some(query) = &self.query {
            serde_urlencoded::from_str(query).ok()
        } else {
            None
        }
    }
}
