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

    /// Create a new [`Segment`].
    pub fn new() -> Self {
        Default::default()
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

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_core::{Element, Scope};

    #[test]
    fn fallback() {
        let s = Segment::new().fallback(RouteContent::RcNone);

        let fallback_is_correct = match s.dynamic {
            DynamicRoute::Fallback(RouteContent::RcNone) => true,
            _ => false,
        };
        assert!(fallback_is_correct);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn fallback_panic_in_debug() {
        Segment::new()
            .fallback(RouteContent::RcNone)
            .fallback(RouteContent::RcNone);
    }

    #[test]
    #[cfg(not(debug_assertions))]
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

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn fixed_panic_in_debug() {
        Segment::new()
            .fixed("test", Route::new(RouteContent::RcNone))
            .fixed("test", Route::new(RouteContent::RcNone));
    }

    #[test]
    #[cfg(not(debug_assertions))]
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

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn index_panic_in_debug() {
        Segment::new()
            .index(RouteContent::RcComponent(TestComponent))
            .index(RouteContent::RcNone);
    }

    #[test]
    #[cfg(not(debug_assertions))]
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

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn parameter_panic_in_debug() {
        Segment::new()
            .parameter(ParameterRoute::new("", RouteContent::RcNone))
            .parameter(ParameterRoute::new("", RouteContent::RcNone));
    }

    #[test]
    #[cfg(not(debug_assertions))]
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
