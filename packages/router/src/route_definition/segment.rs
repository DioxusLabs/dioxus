use std::collections::{BTreeMap, HashSet};

use log::error;
use regex::Regex;
use urlencoding::encode;

use super::{ParameterRoute, Route, RouteContent};

/// A collection of routes for a single path segment.
///
/// A segment refers to the value between two `/` in the path. For example `/blog/1` contains two
/// segments: `["blog", "1"]`.
#[derive(Clone, Debug, Default)]
pub struct Segment {
    pub(crate) fallback: RouteContent,
    pub(crate) fixed: BTreeMap<String, Route>,
    pub(crate) index: RouteContent,
    pub(crate) matching: Vec<(Regex, ParameterRoute)>,
    pub(crate) parameter: Option<ParameterRoute>,
}

impl Segment {
    /// Add a _fallback_ route.
    ///
    /// A _fallback_ route acts like a `404.html` file (with some web servers). It is active, if
    /// there is no completely matching route for the specified path.
    ///
    /// # A single fallback route
    /// Consider the following example:
    ///
    /// ```rust
    /// # use dioxus_router::prelude::*;
    /// # use dioxus::prelude::*;
    /// # fn Index(cx: Scope) -> Element { unimplemented!() }
    /// # fn Fixed(cx: Scope) -> Element { unimplemented!() }
    /// # fn Fallback(cx: Scope) -> Element { unimplemented!() }
    /// Segment::new()
    ///     .index(Index as Component)
    ///     .fixed("fixed", Fixed as Component)
    ///     .fallback(Fallback as Component);
    /// ```
    ///
    /// This would make the following components active for the following paths:
    /// - `/` -> `Index`
    /// - `/fixed` -> `Fixed`
    /// - `/invalid` -> `Fallback`
    /// - `/fixed/invalid` -> `Fallback`
    ///
    /// # Nested fallback routes
    /// The fallback route of a nested [`Segment`] takes precedence over the fallback route of an
    /// outer [`Segment`].
    ///
    /// ```rust
    /// # use dioxus_router::prelude::*;
    /// # use dioxus::prelude::*;
    /// # fn Index(cx: Scope) -> Element { unimplemented!() }
    /// # fn Fallback(cx: Scope) -> Element { unimplemented!() }
    /// # fn Fixed(cx: Scope) -> Element { unimplemented!() }
    /// # fn NestedIndex(cx: Scope) -> Element { unimplemented!() }
    /// # fn NestedFallback(cx: Scope) -> Element { unimplemented!() }
    /// # fn NestedFixed(cx: Scope) -> Element { unimplemented!() }
    /// Segment::new()
    ///     .index(Index as Component)
    ///     .fixed("fixed", Route::new(Fixed as Component).nested(
    ///         Segment::new()
    ///             .index(NestedIndex as Component)
    ///             .fixed("nested", NestedFixed as Component)
    ///             .fallback(NestedFallback as Component)
    ///     ))
    ///     .fallback(Fallback as Component);
    /// ```
    ///
    /// This would make the following components active for the following paths:
    /// - `/` -> `Index`
    /// - `/fixed` -> `Fixed` & `NestedIndex`
    /// - `/fixed/nested` -> `Fixed` & `NestedFixed`
    /// - `/invalid` -> `Fallback`
    /// - `/fixed/invalid` -> `NestedFallback`
    /// - `/fixed/nested/invalid` -> `NestedFallback`
    ///
    /// # Panic
    /// If a _fallback_ route was already set, but only in debug builds.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus_router::prelude::*;
    /// Segment::new().fallback(());
    /// ```
    pub fn fallback(mut self, content: impl Into<RouteContent>) -> Self {
        if !self.fallback.is_empty() {
            error!("fallback route already set, later prevails");
            #[cfg(debug_assertions)]
            panic!("fallback route already set");
        }

        self.fallback = content.into();
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
    /// Segment::new().fixed("path", Route::new(()));
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
    /// Segment::new().index(());
    /// ```
    pub fn index(mut self, content: impl Into<RouteContent>) -> Self {
        if !self.index.is_empty() {
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
    /// Segment::new().matching(Regex::new(".*").unwrap(), ParameterRoute::new("key", ()));
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
    /// A _parameter_ route is like a _matching_ route with a `/./` `regex`. It is active if these
    /// conditions apply:
    /// 1. The [`Segment`] is specified by the path,
    /// 2. no _fixed_ route is active,
    /// 3. and no _matching_ route is active.
    ///
    /// The segments complete value will be provided as a parameter using the `key` specified in the
    /// `route`.
    ///
    /// # URL decoding
    /// - The segments value will be decoded when providing it as a parameter.
    ///
    /// # Panic
    /// - If a _parameter_ route  was already set, but only in debug builds.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus_router::prelude::*;
    /// Segment::new().parameter(ParameterRoute::new("key", ()));
    /// ```
    pub fn parameter(mut self, route: impl Into<ParameterRoute>) -> Self {
        if self.parameter.is_some() {
            error!("parameter route already set, later prevails");
            #[cfg(debug_assertions)]
            panic!("parameter route already set");
        }

        self.parameter = Some(route.into());
        self
    }

    /// Generate a sitemap.
    ///
    /// This function will create a `Vec` that contains all paths the router can handle. Segments
    /// accepting a parameter will be prefixed with `\`.
    ///
    /// All strings will be URL encoded. As `\` is not a valid characters for URLs, the escaping
    /// explained in the previous paragraph is unambiguous, but parameter URLs must be filtered out
    /// if valid URLs are required.
    pub fn sitemap(&self) -> Vec<String> {
        self.sitemap_internal(None, None)
    }

    /// Generate a sitemap with the provided parameters.
    ///
    /// This function will create a `Vec` that contains all paths the router can handle. Segments
    /// accepting a parameter will inserted once for every parameter. Duplicates are later removed.
    ///
    /// For a matching route to be included, a parameter must be valid.
    ///
    /// All strings will be URL encoded.
    pub fn sitemap_with_parameters(
        &self,
        params: &BTreeMap<&'static str, HashSet<String>>,
    ) -> HashSet<String> {
        self.sitemap_internal(Some(params), None)
            .into_iter()
            .collect()
    }

    /// Generate a full sitemap with the provided parameters / parent path.
    fn sitemap_internal(
        &self,
        params: Option<&BTreeMap<&'static str, HashSet<String>>>,
        parents: Option<&str>,
    ) -> Vec<String> {
        let parents = parents.unwrap_or("/");
        let mut res = Vec::new();

        // insert index
        if parents == "/" {
            res.push(String::from("/"));
        }

        // insert fixed routes
        for (name, route) in &self.fixed {
            let parents = format!("{parents}{}/", encode(name));
            res.push(parents.clone());

            if let Some(n) = &route.nested {
                res.append(&mut n.sitemap_internal(params, Some(&parents)));
            }
        }

        // insert matching routes
        for (regex, route) in &self.matching {
            match params {
                None => {
                    let parents = format!("{parents}\\{}/", encode(route.key));
                    res.push(parents.clone());

                    if let Some(n) = &route.nested {
                        res.append(&mut n.sitemap_internal(params, Some(&parents)));
                    }
                }
                Some(p) => {
                    if let Some(p) = p.get(route.key) {
                        for p in p {
                            if regex.is_match(p) {
                                let parents = format!("{parents}{}/", encode(p));
                                res.push(parents.clone());

                                if let Some(n) = &route.nested {
                                    res.append(&mut n.sitemap_internal(params, Some(&parents)));
                                }
                            }
                        }
                    }
                }
            }
        }

        // insert parameter route
        if let Some(route) = &self.parameter {
            match params {
                None => {
                    let parents = format!("{parents}\\{}/", encode(route.key));
                    res.push(parents.clone());

                    if let Some(n) = &route.nested {
                        res.append(&mut n.sitemap_internal(params, Some(&parents)));
                    }
                }
                Some(p) => {
                    if let Some(p) = p.get(route.key) {
                        for p in p {
                            let parents = format!("{parents}{}/", encode(p));
                            res.push(parents.clone());

                            if let Some(n) = &route.nested {
                                res.append(&mut n.sitemap_internal(params, Some(&parents)));
                            }
                        }
                    }
                }
            }
        }

        res
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::NavigationTarget;

    use super::*;
    use dioxus::prelude::*;

    #[test]
    fn fallback() {
        let s = Segment::new().fallback("test");

        let fallback_is_correct = match s.fallback {
            RouteContent::Redirect(NavigationTarget::InternalTarget(target)) => target == "test",
            _ => false,
        };
        assert!(fallback_is_correct);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = "fallback route already set"]
    fn fallback_panic_in_debug() {
        Segment::new()
            .fallback("test")
            .fallback(RouteContent::Empty);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn fallback_override_in_release() {
        let s = Segment::new()
            .fallback("test")
            .fallback(RouteContent::Empty);

        assert!(matches!(s.fallback, RouteContent::Empty));
    }

    #[test]
    fn fixed() {
        let s = Segment::new().fixed("test", Route::new(RouteContent::Empty));

        assert_eq!(s.fixed.len(), 1);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = r#"two fixed routes with identical path: "test""#]
    fn fixed_panic_in_debug() {
        Segment::new()
            .fixed("test", Route::new(RouteContent::Empty))
            .fixed("test", Route::new(RouteContent::Empty));
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn fixed_override_in_release() {
        let s = Segment::new()
            .fixed("test", Route::new(RouteContent::Component(TestComponent)))
            .fixed("test", Route::new(RouteContent::Empty));

        assert_eq!(s.fixed.len(), 1);
        let r = &s.fixed["test"];
        assert!(r.content.is_empty());
        assert!(r.name.is_none());
        assert!(r.nested.is_none());
    }

    #[test]
    fn index() {
        let s = Segment::new().index(RouteContent::Component(TestComponent));

        assert!(!s.index.is_empty());
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = "index route already set"]
    fn index_panic_in_debug() {
        Segment::new()
            .index(RouteContent::Component(TestComponent))
            .index(RouteContent::Empty);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn index_override_in_release() {
        let s = Segment::new()
            .index(RouteContent::Component(TestComponent))
            .index(RouteContent::Empty);

        assert!(s.index.is_empty());
    }

    #[test]
    fn matching() {
        let regex = Regex::new("").unwrap();
        let s = Segment::new().matching(regex, ParameterRoute::new("", RouteContent::Empty));

        assert_eq!(s.matching.len(), 1);
    }

    #[test]
    fn parameter() {
        let s = Segment::new().parameter(ParameterRoute::new("", "test"));

        let parameter_is_correct = match s.parameter {
            Some(ParameterRoute {
                name: _,
                key: _,
                content: RouteContent::Redirect(NavigationTarget::InternalTarget(target)),
                nested: _,
            }) => target == "test",
            _ => false,
        };
        assert!(parameter_is_correct);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = "parameter route already set"]
    fn parameter_panic_in_debug() {
        Segment::new()
            .parameter(ParameterRoute::new("", RouteContent::Empty))
            .parameter(ParameterRoute::new("", RouteContent::Empty));
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn parameter_override_in_release() {
        let s = Segment::new()
            .parameter(ParameterRoute::new(
                "",
                RouteContent::Component(TestComponent),
            ))
            .parameter(ParameterRoute::new("", RouteContent::Empty));

        let fallback_is_correct = match s.parameter {
            Some(p) => p.content.is_empty(),
            _ => false,
        };
        assert!(fallback_is_correct);
    }

    #[allow(non_snake_case)]
    fn TestComponent(_: Scope) -> Element {
        unimplemented!()
    }
}
