use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
};

use dioxus_core::Component;

use crate::navigation::NavigationTarget;

/// The current routing information.
pub struct RouterState {
    /// Whether the service can handle external navigation targets.
    pub can_external: bool,

    /// Whether there is a prior path to go back to.
    ///
    /// This might be [`true`] even if there isn't.
    pub can_go_back: bool,

    /// Whether there is a later path to forward to.
    ///
    /// This might be [`true`] even if there isn't.
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
    pub(crate) fn new() -> Self {
        Self {
            can_external: Default::default(),
            can_go_back: Default::default(),
            can_go_forward: Default::default(),
            components: Default::default(),
            names: Default::default(),
            path: Default::default(),
            prefix: Default::default(),
            query: Default::default(),
            parameters: Default::default(),
        }
    }

    /// Checks if the provided `target` is currently active.
    ///
    /// # [`NtPath`](crate::navigation::NavigationTarget::NtPath)
    /// If the target is a path and `exact` is [`true`], the current path must match the `target`
    /// path exactly.
    ///
    /// If `exact` is [`false`] and the `target` path is absolute (starts with `/`), the current
    /// path must start with the `target` path.
    ///
    /// Otherwise, the last segment of the current path must match the `target` path.
    ///
    /// # [`NtName`](crate::navigation::NavigationTarget::NtName)
    /// The `target` name must be in the list of active names.
    ///
    /// If `exact` is [`true`], all `target` parameters must be matched by current parameters. The
    /// `target` is still active, even if the current parameters are more than the `target`
    /// parameters.
    ///
    /// The query is ignored.
    ///
    /// # [`NtExternal`](crate::navigation::NavigationTarget::NtExternal)
    /// Always [`false`].
    #[must_use]
    pub fn is_active(&self, target: &NavigationTarget, exact: bool) -> bool {
        match target {
            NavigationTarget::NtPath(path) => {
                if exact {
                    return &self.path == path;
                }

                // absolute path
                if path.starts_with('/') {
                    return self.path.starts_with(path);
                }

                // relative path
                if let Some(segment) = self.path.split('/').last() {
                    return segment == path;
                }

                false
            }
            NavigationTarget::NtName(name, vars, _) => {
                if !self.names.contains(name) {
                    return false;
                }

                // ensure specified vars match when exact
                if exact {
                    for (k, v) in vars {
                        match self.parameters.get(k) {
                            Some(val) => {
                                if val != v {
                                    return false;
                                }
                            }
                            None => return false,
                        }
                    }
                }

                true
            }
            NavigationTarget::NtExternal(_) => false,
        }
    }

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

// [`Component`] (in `components`) doesn't implement [`Debug`]
impl Debug for RouterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouterState")
            .field("can_external", &self.can_external)
            .field("can_go_back", &self.can_go_back)
            .field("can_go_forward", &self.can_go_forward)
            .field("names", &self.names)
            .field("path", &self.path)
            .field("prefix", &self.prefix)
            .field("query", &self.query)
            .field("parameters", &self.parameters)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::navigation::Query;

    #[test]
    fn is_active_external() {
        let state = test_state();

        assert!(!state.is_active(&NavigationTarget::NtExternal(String::from("test")), false));
        assert!(!state.is_active(&NavigationTarget::NtExternal(String::from("test")), true));
    }

    #[test]
    fn is_active_path_absolute() {
        let state = test_state();

        assert!(state.is_active(&NavigationTarget::NtPath(String::from("/test")), false));
        assert!(state.is_active(&NavigationTarget::NtPath(String::from("/test/nest")), false));
        assert!(!state.is_active(&NavigationTarget::NtPath(String::from("/invalid")), false));
    }

    #[test]
    fn is_active_path_exact() {
        let state = test_state();

        assert!(state.is_active(&NavigationTarget::NtPath(String::from("/test/nest")), true));
        assert!(!state.is_active(&NavigationTarget::NtPath(String::from("test/nest")), true));
    }

    #[test]
    fn is_active_path_relative() {
        let state = test_state();

        assert!(!state.is_active(&NavigationTarget::NtPath(String::from("test")), false));
        assert!(state.is_active(&NavigationTarget::NtPath(String::from("nest")), false));
    }

    #[test]
    fn is_active_name() {
        let state = test_state();

        assert!(state.is_active(
            &NavigationTarget::NtName("test", vec![], Query::QNone),
            false
        ));
        assert!(state.is_active(
            &NavigationTarget::NtName("nest", vec![], Query::QNone),
            false
        ));
        assert!(!state.is_active(
            &NavigationTarget::NtName("invalid", vec![], Query::QNone),
            false
        ));
    }

    #[test]
    fn is_active_name_exact() {
        let state = test_state();

        assert!(state.is_active(
            &NavigationTarget::NtName("test", vec![("test", String::from("test"))], Query::QNone),
            true
        ));
        assert!(!state.is_active(
            &NavigationTarget::NtName(
                "invalid",
                vec![("test", String::from("test"))],
                Query::QNone
            ),
            true
        ));
        assert!(!state.is_active(
            &NavigationTarget::NtName(
                "test",
                vec![("invalid", String::from("test"))],
                Query::QNone
            ),
            true
        ));
        assert!(!state.is_active(
            &NavigationTarget::NtName(
                "test",
                vec![("test", String::from("invalid"))],
                Query::QNone
            ),
            true
        ));
    }

    fn test_state() -> RouterState {
        RouterState {
            can_external: false,
            can_go_back: false,
            can_go_forward: false,
            components: (vec![], BTreeMap::new()),
            names: {
                let mut names = BTreeSet::new();
                names.insert("test");
                names.insert("nest");
                names
            },
            path: String::from("/test/nest"),
            prefix: String::from(""),
            query: None,
            parameters: {
                let mut parameters = BTreeMap::new();
                parameters.insert("test", String::from("test"));
                parameters
            },
        }
    }
}
