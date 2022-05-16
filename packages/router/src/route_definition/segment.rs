use std::collections::BTreeMap;

use log::error;
use regex::Regex;

use super::{DynamicRoute, ParameterRoute, Route, RouteContent};

/// A collection of routes for a single path segment.
///
/// A segment refers to the value between two `/` in the path. For example `/blog/1` contains two
/// segments: `["blog", "1"]`.
///
/// # Note on _fixed_ and _matching_ routes
/// When checking if a _fixed_ or _matching_ route is active, no url en- or decoding is preformed.
/// If your _fixed_ or _matching_ route contains characters that need to be encoded, you have to
/// encode them in the value/regex as well.
#[derive(Clone, Default)]
pub struct Segment {
    pub(crate) dynamic: DynamicRoute,
    pub(crate) fixed: BTreeMap<String, Route>,
    pub(crate) index: RouteContent,
    pub(crate) matching: Vec<(Regex, ParameterRoute)>,
}

impl Segment {
    /// Add a _fallback_ route.
    ///
    /// The _fallback_ route is active if:
    /// - no _fixed_ route is
    /// - no _matching_ route is
    ///
    /// Mutually exclusive with a _parameter_ route.
    ///
    /// # Panic
    /// If a parameter route or fallback was already set, but only in debug builds.
    pub fn fallback(mut self, content: RouteContent) -> Self {
        if !self.dynamic.is_none() {
            error!("fallback or parameter route already set, later prevails");
            #[cfg(debug_assertions)]
            panic!("fallback or parameter route already set");
        }

        self.dynamic = DynamicRoute::Fallback(content);
        self
    }

    /// Add a _fixed_ route.
    ///
    /// A _fixed_ route is active, if it matches the path segment _exactly_.
    ///
    /// # Panic
    /// If a fixed route with the same `path` was already added, but only in debug builds.
    pub fn fixed(mut self, path: &str, route: Route) -> Self {
        if self.fixed.insert(path.to_string(), route).is_some() {
            error!(r#"two fixed routes with identical path: "{path}", later prevails"#);
            #[cfg(debug_assertions)]
            panic!(r#"two fixed routes with identical path: "{path}""#)
        }

        self
    }

    /// Add an _index_ route.
    ///
    /// The _index_ route is active if the [`Segment`] is the first to be not specified by the path.
    /// For example if the path is `/`, no segment is specified and the _index_ route of the root
    /// segment is active.
    ///
    /// # Panic
    /// If an index route was already set, but only in debug builds.
    pub fn index(mut self, content: RouteContent) -> Self {
        if !self.index.is_rc_none() {
            error!("index route already set, later prevails");
            #[cfg(debug_assertions)]
            panic!("index route already set");
        }

        self.index = content;
        self
    }

    /// Add a _matching_ parameter route.
    ///
    /// A _matching_ route is active if:
    /// - no _fixed_ route is
    /// - the segment matches the provided `regex`
    pub fn matching(mut self, regex: Regex, content: ParameterRoute) -> Self {
        self.matching.push((regex, content));
        self
    }

    /// Add a _parameter_ route.
    ///
    /// The _parameter_ route is active if:
    /// - no _fixed_ route is
    /// - no _matching_ route is
    ///
    /// Mutually exclusive with a _fallback_ route.
    ///
    /// # Panic
    /// If a parameter route or fallback was already set, but only in debug builds.
    pub fn parameter(mut self, parameter: ParameterRoute) -> Self {
        if !self.dynamic.is_none() {
            error!("fallback or parameter route already set, later prevails");
            #[cfg(debug_assertions)]
            panic!("fallback or parameter route already set");
        }

        self.dynamic = DynamicRoute::Parameter(parameter);
        self
    }
}
