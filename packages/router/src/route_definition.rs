//! Several data types for defining what component to render for which path.
//!
//! TODO

use dioxus_core::Component;

use crate::navigation::InternalNavigationTarget;

/// TODO
#[derive(Clone)]
pub struct Segment {
    /// TODO
    pub index: Option<RouteTarget>,
    /// TODO
    pub dynamic: DynamicRoute,
    /// TDO
    pub fixed: Vec<(String, Route)>,
}

/// TODO
#[derive(Clone)]
pub struct Route {
    /// TODO
    pub name: Option<&'static str>,
    /// TODO
    pub content: RouteTarget,
    /// TODO
    pub sub: Option<Segment>,
}

/// TODO
#[derive(Clone)]
pub enum DynamicRoute {
    /// TODO
    None,
    /// TODO
    Variable {
        /// TODO
        name: Option<&'static str>,
        /// TODO
        key: &'static str,
        /// TODO
        content: RouteTarget,
        /// TODO
        sub: Option<Box<Segment>>,
    },
    /// TODO
    Fallback(RouteTarget),
}

/// TODO
#[derive(Clone)]
pub enum RouteTarget {
    /// TODO
    TComponent(Component),
    /// TODO
    TRedirect(InternalNavigationTarget),
}
