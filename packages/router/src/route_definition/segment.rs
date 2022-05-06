use std::collections::BTreeMap;

use log::error;

use super::{DynamicRoute, Route, RouteContent};

/// A collection of routes for a single path segment.
///
/// A segment refers to the value between two `/` in the path. For example `/blog/1` contains two
/// segments: `["blog", "1"]`.
///
/// # What route is active when?
/// - The `index` is active if the path ended before this segment. For example, the `index` of the
///   root segment is active, if the path is `/`.
/// - A `fixed` route is active, if its path (the [`String`]) matches the current segment _exactly_.
///   For example, a route matching `"blog"` is active when the path is `/blog`.
/// - The `dynamic` route is active, if no `fixed` route is active.
///
/// # `index` vs. fixed route with empty path
/// At first glance it may seem that the `index` route and a fixed route with an empty may be the
/// same. However, this is not the case.
///
/// The index route is active when the current segment is not specified by the path. This means that
/// the `index` of the root segment is only active when the path is `/`.
///
/// A `fixed` route with an empty path is active when the current segment is empty. This means that
/// such a route on the root segment is active when the path starts with `//`.
///
/// # Note on `fixed` routes
/// When checking if the `fixed` route matches the current segment, no url en- or decoding is
/// performed. If your `fixed` route contains character that needs to be encoded, you have to encode
/// it in the path.
#[derive(Clone, Default)]
pub struct Segment {
    pub(crate) dynamic: DynamicRoute,
    pub(crate) fixed: BTreeMap<String, Route>,
    pub(crate) index: RouteContent,
}

impl Segment {
    /// Add a dynamic route.
    ///
    /// # Panic
    /// If a dynamic route was already set, but only in debug builds.
    pub fn dynamic(mut self, dynamic_route: DynamicRoute) -> Self {
        if !self.dynamic.is_none() {
            error!("dynamic route already set, later prevails");
            #[cfg(debug_assertions)]
            panic!("dynamic route already set");
        }

        self.dynamic = dynamic_route;
        self
    }

    /// Add a new fixed route.
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

    /// Add an index route.
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
}
