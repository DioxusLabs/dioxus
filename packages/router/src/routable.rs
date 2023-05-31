//! # Routable

#![allow(non_snake_case)]
use dioxus::prelude::*;

use std::{fmt::Display, str::FromStr};

/// An error that occurs when parsing a route
#[derive(Debug, PartialEq)]
pub struct RouteParseError<E: std::fmt::Display> {
    /// The attempted routes that failed to match
    pub attempted_routes: Vec<E>,
}

impl<E: std::fmt::Display> std::fmt::Display for RouteParseError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Route did not match:\nAttempted Matches:\n")?;
        for (i, route) in self.attempted_routes.iter().enumerate() {
            writeln!(f, "{}) {route}", i + 1)?;
        }
        Ok(())
    }
}

/// Something that can be created from a query string
pub trait FromQuery {
    /// Create an instance of `Self` from a query string
    fn from_query(query: &str) -> Self;
}

impl<T: for<'a> From<&'a str>> FromQuery for T {
    fn from_query(query: &str) -> Self {
        T::from(query)
    }
}

/// Something that can be created from a route segment
pub trait FromRouteSegment: Sized {
    /// The error that can occur when parsing a route segment
    type Err;

    /// Create an instance of `Self` from a route segment
    fn from_route_segment(route: &str) -> Result<Self, Self::Err>;
}

impl<T: FromStr> FromRouteSegment for T
where
    <T as FromStr>::Err: std::fmt::Display,
{
    type Err = <T as FromStr>::Err;

    fn from_route_segment(route: &str) -> Result<Self, Self::Err> {
        T::from_str(route)
    }
}

/// Something that can be converted to route segments
pub trait ToRouteSegments {
    /// Display the route segments
    fn display_route_segements(self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

impl<I, T: std::fmt::Display> ToRouteSegments for I
where
    I: IntoIterator<Item = T>,
{
    fn display_route_segements(self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for segment in self {
            write!(f, "/")?;
            write!(f, "{}", segment)?;
        }
        Ok(())
    }
}

/// Something that can be created from route segments
pub trait FromRouteSegments: Sized {
    /// The error that can occur when parsing route segments
    type Err;

    /// Create an instance of `Self` from route segments
    fn from_route_segments(segments: &[&str]) -> Result<Self, Self::Err>;
}

impl<I: std::iter::FromIterator<String>> FromRouteSegments for I {
    type Err = <String as FromRouteSegment>::Err;

    fn from_route_segments(segments: &[&str]) -> Result<Self, Self::Err> {
        segments
            .iter()
            .map(|s| String::from_route_segment(s))
            .collect()
    }
}

/// Something that can be:
/// 1) Converted from a route
/// 2) Converted to a route
/// 3) Rendered as a component
///
/// This trait can be derived using the `#[derive(Routable)]` macro
pub trait Routable: std::fmt::Display + std::str::FromStr + Clone + 'static {
    /// The error that can occur when parsing a route
    const SITE_MAP: &'static [SiteMapSegment];

    /// Render the route at the given level
    fn render<'a>(&self, cx: &'a ScopeState, level: usize) -> Element<'a>;
}

trait RoutableFactory {
    type Err: std::fmt::Display;
    type Routable: Routable + FromStr<Err = Self::Err>;
}

impl<R: Routable + FromStr> RoutableFactory for R
where
    <R as FromStr>::Err: std::fmt::Display,
{
    type Err = <R as FromStr>::Err;
    type Routable = R;
}

trait RouteRenderable: std::fmt::Display + 'static {
    fn render<'a>(&self, cx: &'a ScopeState, level: usize) -> Element<'a>;
}

impl<R: Routable> RouteRenderable for R
where
    <R as FromStr>::Err: std::fmt::Display,
{
    fn render<'a>(&self, cx: &'a ScopeState, level: usize) -> Element<'a> {
        self.render(cx, level)
    }
}

/// A type erased map of the site structurens
#[derive(Debug, Clone, PartialEq)]
pub struct SiteMapSegment {
    /// The type of the route segment
    pub segment_type: SegmentType,
    /// The children of the route segment
    pub children: &'static [SiteMapSegment],
}

impl SiteMapSegment {
    /// Take a map of the site structure and flatten it into a vector of routes
    pub fn flatten(&self) -> Vec<Vec<SegmentType>> {
        let mut routes = Vec::new();
        self.flatten_inner(&mut routes, Vec::new());
        routes
    }

    fn flatten_inner(&self, routes: &mut Vec<Vec<SegmentType>>, current: Vec<SegmentType>) {
        let mut current = current;
        current.push(self.segment_type.clone());
        if self.children.is_empty() {
            routes.push(current);
        } else {
            for child in self.children {
                child.flatten_inner(routes, current.clone());
            }
        }
    }
}

/// The type of a route segment
#[derive(Debug, Clone, PartialEq)]
pub enum SegmentType {
    /// A static route segment
    Static(&'static str),
    /// A dynamic route segment
    Dynamic(&'static str),
    /// A catch all route segment
    CatchAll(&'static str),
}

impl Display for SegmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            SegmentType::Static(s) => write!(f, "/{}", s),
            SegmentType::Dynamic(s) => write!(f, "/:{}", s),
            SegmentType::CatchAll(s) => write!(f, "/:...{}", s),
        }
    }
}
