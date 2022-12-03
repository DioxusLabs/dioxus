use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

use either::Either;

use crate::{
    navigation::NavigationTarget, routes::ContentAtom, segments::NameMap, utils::resolve_target,
    Name,
};

/// The current state of the router.
#[derive(Debug)]
pub struct RouterState<T: Clone> {
    /// Whether there is a previous page to navigate back to.
    ///
    /// Even if this is [`true`], there might not be a previous page. However, it is nonetheless
    /// safe to tell the router to go back.
    pub can_go_back: bool,
    /// Whether there is a future page to navigate forward to.
    ///
    /// Even if this is [`true`], there might not be a future page. However, it is nonetheless safe
    /// to tell the router to go forward.
    pub can_go_forward: bool,

    /// The current path.
    pub path: String,
    /// The current query.
    pub query: Option<String>,
    /// The current prefix.
    pub prefix: Option<String>,

    /// The names of currently active routes.
    pub names: HashSet<Name>,
    /// The current path parameters.
    pub parameters: HashMap<Name, String>,
    pub(crate) name_map: Arc<NameMap>,

    /// The current main content.
    ///
    /// This should only be used by UI integration crates, and not by applications.
    pub content: Vec<ContentAtom<T>>,
    /// The current named content.
    ///
    /// This should only be used by UI integration crates, and not by applications.
    pub named_content: BTreeMap<Name, Vec<ContentAtom<T>>>,
}

impl<T: Clone> RouterState<T> {
    /// Get a parameter.
    ///
    /// ```rust
    /// # use dioxus_router_core::{RouterState, Name};
    /// let mut state = RouterState::<&'static str>::default();
    /// assert_eq!(state.parameter::<bool>(), None);
    ///
    /// // Do not do this! For illustrative purposes only!
    /// state.parameters.insert(Name::of::<bool>(), String::from("some parameter"));
    /// assert_eq!(state.parameter::<bool>(), Some("some parameter".to_string()));
    /// ```
    pub fn parameter<N: 'static>(&self) -> Option<String> {
        self.parameters.get(&Name::of::<N>()).cloned()
    }

    /// Get the `href` for the `target`.
    pub fn href(&self, target: &NavigationTarget) -> String {
        match resolve_target(&self.name_map, &target) {
            Either::Left(Either::Left(i)) => match &self.prefix {
                Some(p) => format!("{p}{i}"),
                None => i,
            },
            Either::Left(Either::Right(n)) => {
                // the following assert currently cannot trigger, as resolve_target (or more
                // precisely resolve_name, which is called by resolve_targe) will panic in debug
                debug_assert!(false, "requested href for unknown name or parameter: {n}");
                String::new()
            }
            Either::Right(e) => e,
        }
    }

    /// Check whether the `target` is currently active.
    ///
    /// # Normal mode
    /// 1. For internal targets wrapping an absolute path, the current path has to start with it.
    /// 2. For internal targets wrapping a relative path, it has to match the last current segment
    ///    exactly.
    /// 3. For named targets, the provided name needs to be active.
    /// 4. For external targets [`false`].
    ///
    /// # Exact mode
    /// 1. For internal targets, the current path must match the wrapped path exactly.
    /// 2. For named targets, the provided name needs to be active and all parameters need to match
    ///    exactly.
    /// 3. For external targets [`false`].
    pub fn is_at(&self, target: &NavigationTarget, exact: bool) -> bool {
        match target {
            NavigationTarget::Internal(i) => {
                if exact {
                    i == &self.path
                } else if i.starts_with('/') {
                    self.path.starts_with(i)
                } else if let Some((_, s)) = self.path.rsplit_once('/') {
                    s == i
                } else {
                    false
                }
            }
            NavigationTarget::Named {
                name,
                parameters,
                query: _,
            } => {
                if !self.names.contains(name) {
                    false
                } else if exact {
                    for (k, v) in parameters {
                        match self.parameters.get(k) {
                            Some(p) if p != v => return false,
                            None => return false,
                            _ => {}
                        }
                    }

                    true
                } else {
                    true
                }
            }
            NavigationTarget::External(_) => false,
        }
    }
}

// manual impl required because derive macro requires default for T unnecessarily
impl<T: Clone> Default for RouterState<T> {
    fn default() -> Self {
        Self {
            can_go_back: Default::default(),
            can_go_forward: Default::default(),
            path: Default::default(),
            query: Default::default(),
            prefix: Default::default(),
            names: Default::default(),
            parameters: Default::default(),
            name_map: Default::default(),
            content: Default::default(),
            named_content: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{navigation::named, prelude::RootIndex, routes::Segment, segments::NamedSegment};

    use super::*;

    #[test]
    fn href_internal() {
        let state = RouterState::<&str> {
            prefix: Some(String::from("/prefix")),
            ..Default::default()
        };

        assert_eq!(state.href(&"/test".into()), String::from("/prefix/test"))
    }

    #[test]
    fn href_named() {
        let state = RouterState::<&str> {
            name_map: Arc::new(NamedSegment::from_segment(&Segment::<&str>::empty())),
            prefix: Some(String::from("/prefix")),
            ..Default::default()
        };

        assert_eq!(state.href(&named::<RootIndex>()), String::from("/prefix/"))
    }

    #[test]
    #[should_panic = "named navigation to unknown name: bool"]
    #[cfg(debug_assertions)]
    fn href_named_debug() {
        let state = RouterState::<&str> {
            name_map: Arc::new(NamedSegment::from_segment(&Segment::<&str>::empty())),
            prefix: Some(String::from("/prefix")),
            ..Default::default()
        };

        state.href(&named::<bool>());
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn href_named_release() {
        let state = RouterState::<&str> {
            name_map: Arc::new(NamedSegment::from_segment(&Segment::<&str>::empty())),
            prefix: Some(String::from("/prefix")),
            ..Default::default()
        };

        assert_eq!(state.href(&named::<bool>()), String::new())
    }

    #[test]
    fn href_external() {
        let state = RouterState::<&str> {
            prefix: Some(String::from("/prefix")),
            ..Default::default()
        };

        assert_eq!(
            state.href(&"https://dioxuslabs.com/".into()),
            String::from("https://dioxuslabs.com/")
        )
    }

    #[test]
    fn is_at_internal_absolute() {
        let state = test_state();

        assert!(!state.is_at(&"/levels".into(), false));
        assert!(!state.is_at(&"/levels".into(), true));

        assert!(state.is_at(&"/test".into(), false));
        assert!(!state.is_at(&"/test".into(), true));

        assert!(state.is_at(&"/test/with/some/nested/levels".into(), false));
        assert!(state.is_at(&"/test/with/some/nested/levels".into(), true));
    }

    #[test]
    fn is_at_internal_relative() {
        let state = test_state();

        assert!(state.is_at(&"levels".into(), false));
        assert!(!state.is_at(&"levels".into(), true));

        assert!(!state.is_at(&"test".into(), false));
        assert!(!state.is_at(&"test".into(), true));

        assert!(!state.is_at(&"test/with/some/nested/levels".into(), false));
        assert!(!state.is_at(&"test/with/some/nested/levels".into(), true));
    }

    #[test]
    fn is_at_named() {
        let state = test_state();

        assert!(!state.is_at(&named::<RootIndex>(), false));
        assert!(!state.is_at(&named::<RootIndex>(), true));

        assert!(state.is_at(&named::<bool>(), false));
        assert!(state.is_at(&named::<bool>(), true));

        assert!(state.is_at(&named::<bool>().parameter::<bool>("test"), false));
        assert!(state.is_at(&named::<bool>().parameter::<bool>("test"), true));

        assert!(state.is_at(&named::<bool>().parameter::<i8>("test"), false));
        assert!(!state.is_at(&named::<bool>().parameter::<i8>("test"), true));
    }

    #[test]
    fn is_at_external() {
        let state = test_state();

        assert!(!state.is_at(&"https://dioxuslabs.com/".into(), false));
        assert!(!state.is_at(&"https://dioxuslabs.com/".into(), true));
    }

    fn test_state() -> RouterState<&'static str> {
        RouterState {
            path: String::from("/test/with/some/nested/levels"),
            names: {
                let mut r = HashSet::new();
                r.insert(Name::of::<bool>());
                r
            },
            parameters: {
                let mut r = HashMap::new();
                r.insert(Name::of::<bool>(), String::from("test"));
                r
            },
            ..Default::default()
        }
    }
}
