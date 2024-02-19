//! # Routable

#![allow(non_snake_case)]
use dioxus_lib::prelude::*;

use std::iter::FlatMap;
use std::slice::Iter;
use std::{fmt::Display, str::FromStr};

/// An error that occurs when parsing a route.
#[derive(Debug, PartialEq)]
pub struct RouteParseError<E: Display> {
    /// The attempted routes that failed to match.
    pub attempted_routes: Vec<E>,
}

impl<E: Display> Display for RouteParseError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Route did not match:\nAttempted Matches:\n")?;
        for (i, route) in self.attempted_routes.iter().enumerate() {
            writeln!(f, "{}) {route}", i + 1)?;
        }
        Ok(())
    }
}

/// Something that can be created from an entire query string.
///
/// This trait needs to be implemented if you want to turn a query string into a struct.
///
/// A working example can be found in the `examples` folder in the root package under `query_segments_demo`.
pub trait FromQuery {
    /// Create an instance of `Self` from a query string.
    fn from_query(query: &str) -> Self;
}

impl<T: for<'a> From<&'a str>> FromQuery for T {
    fn from_query(query: &str) -> Self {
        T::from(&*urlencoding::decode(query).expect("Failed to decode url encoding"))
    }
}

/// Something that can be created from a query argument.
///
/// This trait must be implemented for every type used within a query string in the router macro.
pub trait FromQueryArgument: Default {
    /// The error that can occur when parsing a query argument.
    type Err;

    /// Create an instance of `Self` from a query string.
    fn from_query_argument(argument: &str) -> Result<Self, Self::Err>;
}

impl<T: Default + FromStr> FromQueryArgument for T
where
    <T as FromStr>::Err: Display,
{
    type Err = <T as FromStr>::Err;

    fn from_query_argument(argument: &str) -> Result<Self, Self::Err> {
        let result = match urlencoding::decode(argument) {
            Ok(argument) => T::from_str(&argument),
            Err(err) => {
                tracing::error!("Failed to decode url encoding: {}", err);
                T::from_str(argument)
            }
        };
        match result {
            Ok(result) => Ok(result),
            Err(err) => {
                tracing::error!("Failed to parse query argument: {}", err);
                Err(err)
            }
        }
    }
}

/// Something that can be created from a route segment.
pub trait FromRouteSegment: Sized {
    /// The error that can occur when parsing a route segment.
    type Err;

    /// Create an instance of `Self` from a route segment.
    fn from_route_segment(route: &str) -> Result<Self, Self::Err>;
}

impl<T: FromStr> FromRouteSegment for T
where
    <T as FromStr>::Err: Display,
{
    type Err = <T as FromStr>::Err;

    fn from_route_segment(route: &str) -> Result<Self, Self::Err> {
        match urlencoding::decode(route) {
            Ok(segment) => T::from_str(&segment),
            Err(err) => {
                tracing::error!("Failed to decode url encoding: {}", err);
                T::from_str(route)
            }
        }
    }
}

#[test]
fn full_circle() {
    let route = "testing 1234 hello world";
    assert_eq!(String::from_route_segment(route).unwrap(), route);
}

/// Something that can be converted to route segments.
pub trait ToRouteSegments {
    /// Display the route segments.
    fn display_route_segments(self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

impl<I, T: Display> ToRouteSegments for I
where
    I: IntoIterator<Item = T>,
{
    fn display_route_segments(self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for segment in self {
            write!(f, "/")?;
            let segment = segment.to_string();
            match urlencoding::decode(&segment) {
                Ok(segment) => write!(f, "{}", segment)?,
                Err(err) => {
                    tracing::error!("Failed to decode url encoding: {}", err);
                    write!(f, "{}", segment)?
                }
            }
        }
        Ok(())
    }
}

#[test]
fn to_route_segments() {
    struct DisplaysRoute;

    impl Display for DisplaysRoute {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let segments = vec!["hello", "world"];
            segments.display_route_segments(f)
        }
    }

    assert_eq!(DisplaysRoute.to_string(), "/hello/world");
}

/// Something that can be created from route segments.
pub trait FromRouteSegments: Sized {
    /// The error that can occur when parsing route segments.
    type Err;

    /// Create an instance of `Self` from route segments.
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

/// A flattened version of [`Routable::SITE_MAP`].
/// This essentially represents a `Vec<Vec<SegmentType>>`, which you can collect it into.
type SiteMapFlattened<'a> = FlatMap<
    Iter<'a, SiteMapSegment>,
    Vec<Vec<SegmentType>>,
    fn(&SiteMapSegment) -> Vec<Vec<SegmentType>>,
>;

fn seg_strs_to_route<T>(segs_maybe: &Option<Vec<&str>>) -> Option<T>
where
    T: Routable,
{
    if let Some(str) = seg_strs_to_str(segs_maybe) {
        T::from_str(&str).ok()
    } else {
        None
    }
}

fn seg_strs_to_str(segs_maybe: &Option<Vec<&str>>) -> Option<String> {
    segs_maybe
        .as_ref()
        .map(|segs| String::from('/') + &segs.join("/"))
}

/// Something that can be:
/// 1. Converted from a route.
/// 2. Converted to a route.
/// 3. Rendered as a component.
///
/// This trait can be derived using the `#[derive(Routable)]` macro.
pub trait Routable: FromStr + Display + Clone + 'static {
    /// The error that can occur when parsing a route.
    const SITE_MAP: &'static [SiteMapSegment];

    /// Render the route at the given level
    fn render(&self, level: usize) -> Element;

    /// Checks if this route is a child of the given route.
    ///
    /// # Example
    /// ```rust
    /// use dioxus_router::prelude::*;
    /// use dioxus::prelude::*;
    ///
    /// #[component]
    /// fn Home() -> Element { None }
    /// #[component]
    /// fn About() -> Element { None }
    ///
    /// #[derive(Routable, Clone, PartialEq, Debug)]
    /// enum Route {
    ///     #[route("/")]
    ///     Home {},
    ///     #[route("/about")]
    ///     About {},
    /// }
    ///
    /// let route = Route::About {};
    /// let parent = Route::Home {};
    /// assert!(route.is_child_of(&parent));
    /// ```
    fn is_child_of(&self, other: &Self) -> bool {
        let self_str = self.to_string();
        let self_str = self_str.trim_matches('/');
        let other_str = other.to_string();
        let other_str = other_str.trim_matches('/');
        if other_str.is_empty() {
            return true;
        }
        let self_segments = self_str.split('/');
        let other_segments = other_str.split('/');
        for (self_seg, other_seg) in self_segments.zip(other_segments) {
            if self_seg != other_seg {
                return false;
            }
        }
        true
    }

    /// Get the parent route of this route.
    ///
    /// # Example
    /// ```rust
    /// use dioxus_router::prelude::*;
    /// use dioxus::prelude::*;
    ///
    /// #[component]
    /// fn Home() -> Element { None }
    /// #[component]
    /// fn About() -> Element { None }
    ///
    /// #[derive(Routable, Clone, PartialEq, Debug)]
    /// enum Route {
    ///     #[route("/home")]
    ///     Home {},
    ///     #[route("/home/about")]
    ///     About {},
    /// }
    ///
    /// let route = Route::About {};
    /// let parent = route.parent().unwrap();
    /// assert_eq!(parent, Route::Home {});
    /// ```
    fn parent(&self) -> Option<Self> {
        let as_str = self.to_string();
        let as_str = as_str.trim_matches('/');
        let segments = as_str.split('/');
        let segment_count = segments.clone().count();
        let new_route = segments
            .take(segment_count - 1)
            .fold(String::new(), |mut acc, segment| {
                acc.push('/');
                acc.push_str(segment);
                acc
            });

        Self::from_str(&new_route).ok()
    }

    /// Returns a flattened version of [`Self::SITE_MAP`].
    fn flatten_site_map<'a>() -> SiteMapFlattened<'a> {
        Self::SITE_MAP.iter().flat_map(SiteMapSegment::flatten)
    }

    /// Gets a list of all the static routes.
    /// Example static route: `#[route("/static/route")]`
    fn static_routes() -> Vec<Self> {
        Self::flatten_site_map()
            .filter_map(|route| {
                let route_if_static = &route
                    .iter()
                    .map(|segment| match segment {
                        SegmentType::Static(s) => Some(*s),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>();

                seg_strs_to_route(route_if_static)
            })
            .collect()
    }
}

/// A type erased map of the site structure.
#[derive(Debug, Clone, PartialEq)]
pub struct SiteMapSegment {
    /// The type of the route segment.
    pub segment_type: SegmentType,
    /// The children of the route segment.
    pub children: &'static [SiteMapSegment],
}

impl SiteMapSegment {
    /// Take a map of the site structure and flatten it into a vector of routes.
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

/// The type of a route segment.
#[derive(Debug, Clone, PartialEq)]
pub enum SegmentType {
    /// A static route segment.
    Static(&'static str),
    /// A dynamic route segment.
    Dynamic(&'static str),
    /// A catch all route segment.
    CatchAll(&'static str),
    /// A child router.
    Child,
}

impl Display for SegmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            SegmentType::Static(s) => write!(f, "/{}", s),
            SegmentType::Child => Ok(()),
            SegmentType::Dynamic(s) => write!(f, "/:{}", s),
            SegmentType::CatchAll(s) => write!(f, "/:..{}", s),
        }
    }
}
