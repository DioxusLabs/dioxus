use std::collections::BTreeMap;

use log::error;
use regex::Regex;

use super::{DynamicRoute, ParameterRoute, Route, RouteContent};

/// A collection of routes for a single path segment.
///
/// A segment refers to the value between two `/` in the path. For example `/blog/1` contains two
/// segments: `["blog", "1"]`.
#[derive(Clone, Debug, Default)]
pub struct Segment {
    pub(crate) dynamic: DynamicRoute,
    pub(crate) fixed: BTreeMap<String, Route>,
    pub(crate) index: RouteContent,
    pub(crate) matching: Vec<(Regex, ParameterRoute)>,
}

impl Segment {
    /// Add a _fallback_ route.
    ///
    /// A _fallback_ route is similar to a _parameter_ route. It is active, if these conditions
    /// apply:
    /// 1. The [`Segment`] is specified by the path,
    /// 2. no _fixed_ route is active,
    /// 3. and no _matching_ route is active.
    ///
    /// The segments complete value will __not__ be provided as a parameter.
    ///
    /// A [`Segment`] can have __either__ a _fallback_ route or a _parameter_ route.
    ///
    /// # Interaction with a [`Router`] level `fallback`
    /// The [`Router`] allows you to provide some _fallback_ content. That content will be active if
    /// the router is unable to find an active route. Some examples:
    /// - If the path is `/invalid`, but the root segment (the [`Segment`] passed to the [`Router`])
    ///   has no active route.
    /// - If the path is `/level1/level2/invalid` but the `level2` segment has no nested segment.
    ///
    /// A _fallback_ route inhibits that behavior. In the example above, if a _fallback_ route is
    /// active on the `level2` segment, the [`Router`] fallback content will not be active.
    ///
    /// # Panic
    /// - If a _fallback_ route or _parameter_ route was already set, but only in debug builds.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus_router::prelude::*;
    /// Segment::new().fallback(RcNone);
    /// ```
    ///
    /// [`Router`]: crate::components::Router
    pub fn fallback(mut self, content: impl Into<RouteContent>) -> Self {
        if !self.dynamic.is_none() {
            error!("fallback or parameter route already set, later prevails");
            #[cfg(debug_assertions)]
            panic!("fallback or parameter route already set");
        }

        self.dynamic = DynamicRoute::Fallback(content.into());
        self
    }

    /// Add a _fixed_ route.
    ///
    /// A _fixed_ route acts like a static file or directory (with most web servers). It is active,
    /// if its _path_ matches the segments value exactly. Some examples:
    /// - If the path is `/`, no _fixed_ root is active.
    /// - If the path is `/test` or `/test/`, and the root segment (the [`Segment`] passed to the
    ///   [`Router`](crate::components::Router)) has a _fixed_ route with a `path` of `test`, that
    ///   route will be active.
    /// - If the path is `//`, and the root segment has a _fixed_ route with a `path` of `""` (empty
    ///   string), that route will be active.
    ///
    /// # URL decoding
    /// The segments value will be decoded when checking if the _fixed_ route is active.
    ///
    /// # Panic
    /// - If a _fixed_ route with the same `path` was already added, but only in debug builds.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus_router::prelude::*;
    /// Segment::new().fixed("path", Route::new(RcNone));
    /// ```
    pub fn fixed(mut self, path: &str, route: impl Into<Route>) -> Self {
        if self.fixed.insert(path.to_string(), route.into()).is_some() {
            error!(r#"two fixed routes with identical path: "{path}", later prevails"#);
            #[cfg(debug_assertions)]
            panic!(r#"two fixed routes with identical path: "{path}""#)
        }

        self
    }

    /// Add an _index_ route.
    ///
    /// The _index_ route acts like an `index.html` (with most web servers). It is active, if the
    /// [`Segment`] is the first to not be specified by the path. Some examples:
    /// - If the path is `/`, no segment is specified. The _index_ route of the root segment (the
    ///   [`Segment`] passed to the [`Router`](crate::components::Router)) is active.
    /// - If the path is `/test` or `/test/`, one segment is specified. If the root segment has an
    ///   active route, and that route has a nested [`Segment`], that [`Segment`]s _index_  route is
    ///   active.
    ///
    /// # Panic
    /// - If an _index_ route was already set, but only in debug builds.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus_router::prelude::*;
    /// Segment::new().index(RcNone);
    /// ```
    pub fn index(mut self, content: impl Into<RouteContent>) -> Self {
        if !self.index.is_rc_none() {
            error!("index route already set, later prevails");
            #[cfg(debug_assertions)]
            panic!("index route already set");
        }

        self.index = content.into();
        self
    }

    /// Add a _matching_ parameter route.
    ///
    /// A _matching_ route allows the application to accept a dynamic value, that matches a provided
    /// regular expression. A _matching_ parameter route is active if these conditions apply:
    /// 1. The [`Segment`] is specified by the path,
    /// 2. no _fixed_ route is active,
    /// 3. no prior _matching_ route (in order of addition) is active,
    /// 4. and the `regex` matches the segments value.
    ///
    /// The segments complete value will be provided as a parameter using the `key` specified in the
    /// `route`.
    ///
    /// # URL decoding
    /// - The segments value will be decoded when checking if the _matching_ route is active.
    /// - The segments value will be decoded when providing it as a parameter.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus_router::prelude::*;
    /// # use regex::Regex;
    /// Segment::new().matching(Regex::new(".*").unwrap(), ParameterRoute::new("key", RcNone));
    /// ```
    pub fn matching(mut self, regex: Regex, route: impl Into<ParameterRoute>) -> Self {
        self.matching.push((regex, route.into()));
        self
    }

    /// Create a new [`Segment`].
    pub fn new() -> Self {
        Default::default()
    }

    /// Add a _parameter_ route.
    ///
    /// A _parameter_ route is like a _matching_ route with an empty `regex`. It is active if these
    /// conditions apply:
    /// 1. The [`Segment`] is specified by the path,
    /// 2. no _fixed_ route is active,
    /// 3. and no _matching_ route is active.
    ///
    /// The segments complete value will be provided as a parameter using the `key` specified in the
    /// `route`.
    ///
    /// A [`Segment`] can have __either__ a _parameter_ route or a _fallback_ route.
    ///
    /// # URL decoding
    /// - The segments value will be decoded when providing it as a parameter.
    ///
    /// # Panic
    /// - If a _parameter_ route or _fallback_ route was already set, but only in debug builds.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus_router::prelude::*;
    /// Segment::new().parameter(ParameterRoute::new("key", RcNone));
    /// ```
    pub fn parameter(mut self, route: impl Into<ParameterRoute>) -> Self {
        if !self.dynamic.is_none() {
            error!("fallback or parameter route already set, later prevails");
            #[cfg(debug_assertions)]
            panic!("fallback or parameter route already set");
        }

        self.dynamic = DynamicRoute::Parameter(route.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::*;

    #[test]
    fn fallback() {
        let s = Segment::new().fallback(RouteContent::RcNone);

        let fallback_is_correct = match s.dynamic {
            DynamicRoute::Fallback(RouteContent::RcNone) => true,
            _ => false,
        };
        assert!(fallback_is_correct);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = "fallback or parameter route already set"]
    fn fallback_panic_in_debug() {
        Segment::new()
            .fallback(RouteContent::RcNone)
            .fallback(RouteContent::RcNone);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn fallback_override_in_release() {
        let s = Segment::new()
            .fallback(RouteContent::RcComponent(TestComponent))
            .fallback(RouteContent::RcNone);

        let fallback_is_correct = match s.dynamic {
            DynamicRoute::Fallback(RouteContent::RcNone) => true,
            _ => false,
        };
        assert!(fallback_is_correct);
    }

    #[test]
    fn fixed() {
        let s = Segment::new().fixed("test", Route::new(RouteContent::RcNone));

        assert_eq!(s.fixed.len(), 1);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = r#"two fixed routes with identical path: "test""#]
    fn fixed_panic_in_debug() {
        Segment::new()
            .fixed("test", Route::new(RouteContent::RcNone))
            .fixed("test", Route::new(RouteContent::RcNone));
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn fixed_override_in_release() {
        let s = Segment::new()
            .fixed("test", Route::new(RouteContent::RcComponent(TestComponent)))
            .fixed("test", Route::new(RouteContent::RcNone));

        assert_eq!(s.fixed.len(), 1);
        let r = &s.fixed["test"];
        assert!(r.content.is_rc_none());
        assert!(r.name.is_none());
        assert!(r.nested.is_none());
    }

    #[test]
    fn index() {
        let s = Segment::new().index(RouteContent::RcComponent(TestComponent));

        assert!(!s.index.is_rc_none());
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = "index route already set"]
    fn index_panic_in_debug() {
        Segment::new()
            .index(RouteContent::RcComponent(TestComponent))
            .index(RouteContent::RcNone);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn index_override_in_release() {
        let s = Segment::new()
            .index(RouteContent::RcComponent(TestComponent))
            .index(RouteContent::RcNone);

        assert!(s.index.is_rc_none());
    }

    #[test]
    fn matching() {
        let regex = Regex::new("").unwrap();
        let s = Segment::new().matching(regex, ParameterRoute::new("", RouteContent::RcNone));

        assert_eq!(s.matching.len(), 1);
    }

    #[test]
    fn parameter() {
        let s = Segment::new().parameter(ParameterRoute::new("", RouteContent::RcNone));

        let parameter_is_correct = match s.dynamic {
            DynamicRoute::Parameter(_) => true,
            _ => false,
        };
        assert!(parameter_is_correct);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = "fallback or parameter route already set"]
    fn parameter_panic_in_debug() {
        Segment::new()
            .parameter(ParameterRoute::new("", RouteContent::RcNone))
            .parameter(ParameterRoute::new("", RouteContent::RcNone));
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn parameter_override_in_release() {
        let s = Segment::new()
            .parameter(ParameterRoute::new(
                "",
                RouteContent::RcComponent(TestComponent),
            ))
            .parameter(ParameterRoute::new("", RouteContent::RcNone));

        let fallback_is_correct = match s.dynamic {
            DynamicRoute::Parameter(p) => p.content.is_rc_none(),
            _ => false,
        };
        assert!(fallback_is_correct);
    }

    #[allow(non_snake_case)]
    fn TestComponent(_: Scope) -> Element {
        unimplemented!()
    }
}
