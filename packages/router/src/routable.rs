//! # Routable

#![allow(non_snake_case)]
use dioxus_lib::prelude::*;

use std::slice::Iter;
use std::{fmt::Debug, iter::FlatMap};
use std::{fmt::Display, str::FromStr};

mod hash;
mod query;
mod segments;
mod sitemap;
pub use hash::*;
pub use query::*;
pub use segments::*;
pub use sitemap::*;

/// The Routable trait is implemented for types that can be converted to and from a route and be rendered as a page.
///
/// A Routable object is something that can be:
/// 1. Converted from a route.
/// 2. Converted to a route.
/// 3. Rendered as a component.
///
/// This trait should generally be derived using the [`dioxus_router_macro::Routable`] macro which has more information about the syntax.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
///
/// fn App() -> Element {
///     rsx! {
///         Router::<Route> { }
///     }
/// }
///
/// // Routes are generally enums that derive `Routable`
/// #[derive(Routable, Clone, PartialEq, Debug)]
/// enum Route {
///     // Each enum has an associated url
///     #[route("/")]
///     Home {},
///
///     // Routes can include dynamic segments that are parsed from the url
///     #[route("/blog/:blog_id")]
///     Blog { blog_id: usize },
///
///     // Or query segments that are parsed from the url
///     #[route("/edit?:blog_id")]
///     Edit { blog_id: usize },
///
///     // Or hash segments that are parsed from the url
///     #[route("/hashtag/#:hash")]
///     Hash { hash: String },
/// }
///
/// // Each route variant defaults to rendering a component of the same name
/// #[component]
/// fn Home() -> Element {
///     rsx! {
///         h1 { "Home" }
///     }
/// }
///
/// // If the variant has dynamic parameters, those are passed to the component
/// #[component]
/// fn Blog(blog_id: usize) -> Element {
///     rsx! {
///         h1 { "Blog" }
///     }
/// }
///
/// #[component]
/// fn Edit(blog_id: usize) -> Element {
///     rsx! {
///         h1 { "Edit" }
///     }
/// }
///
/// #[component]
/// fn Hash(hash: String) -> Element {
///     rsx! {
///         h1 { "Hashtag #{hash}" }
///     }
/// }
/// ```
#[rustversion::attr(
    since(1.78.0),
    diagnostic::on_unimplemented(
        message = "`Routable` is not implemented for `{Self}`",
        label = "Route",
        note = "Routable should generally be derived using the `#[derive(Routable)]` macro."
    )
)]
pub trait Routable: Clone + Sized + 'static {
    /// The error that can occur when parsing a route.
    const SITE_MAP: &'static [SiteMapSegment];

    /// Render the route at the given level
    fn render(&self, level: usize) -> Element;

    /// Turn this route into a string so we can parse it again later
    ///
    /// The format here is expected to be a Path like `/about/123` or `https://example.com/about`
    fn serialize(&self) -> String;

    /// Turn a string into this route, or return an error if it can't be parsed
    ///
    /// The format here is expected to be a Path like `/about/123` or `https://example.com/about`
    fn deserialize(route: &str) -> Result<Self, Box<dyn std::error::Error>>;

    /// Get the parent route of this route.
    ///
    /// # Example
    /// ```rust
    /// use dioxus_router::prelude::*;
    /// use dioxus::prelude::*;
    ///
    /// #[component]
    /// fn Home() -> Element { VNode::empty() }
    /// #[component]
    /// fn About() -> Element { VNode::empty() }
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
        let as_str = self.serialize();
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

        Self::deserialize(&new_route).ok()
    }

    /// Checks if this route is a child of the given route.
    ///
    /// # Example
    /// ```rust
    /// use dioxus_router::prelude::*;
    /// use dioxus::prelude::*;
    ///
    /// #[component]
    /// fn Home() -> Element { VNode::empty() }
    /// #[component]
    /// fn About() -> Element { VNode::empty() }
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
        let self_str = self.serialize();
        let self_str = self_str.trim_matches('/');
        let other_str = other.serialize();
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

    /// Returns a flattened version of [`Self::SITE_MAP`].
    fn flatten_site_map<'a>() -> SiteMapFlattened<'a> {
        Self::SITE_MAP.iter().flat_map(SiteMapSegment::flatten)
    }

    /// Gets a list of all the static routes.
    /// Example static route: `#[route("/static/route")]`
    fn static_routes() -> Vec<Self> {
        Self::flatten_site_map()
            .filter_map(|segments| {
                let mut route = String::new();
                for segment in segments.iter() {
                    match segment {
                        SegmentType::Static(s) => {
                            route.push('/');
                            route.push_str(s)
                        }
                        SegmentType::Child => {}
                        _ => return None,
                    }
                }

                Self::deserialize(&route).ok()
            })
            .collect()
    }
}

/// An error that occurs when parsing a route.
#[derive(Debug, PartialEq)]
pub struct RouteParseError<E: Display> {
    /// The attempted routes that failed to match.
    pub attempted_routes: Vec<E>,
}

impl<E: Display + Debug> std::error::Error for RouteParseError<E> {}

impl<E: Display> Display for RouteParseError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Route did not match:\nAttempted Matches:\n")?;
        for (i, route) in self.attempted_routes.iter().enumerate() {
            writeln!(f, "{}) {route}", i + 1)?;
        }
        Ok(())
    }
}

/// Something that can be created from multiple route segments. This must be implemented for any type that is spread into the route segment like `#[route("/:..route_segments")]`.
///
///
/// **This trait is automatically implemented for any types that implement `FromIterator<impl Display>`.**
///
/// ```rust
/// use dioxus::prelude::*;
///
/// #[derive(Routable, Clone, PartialEq, Debug)]
/// enum Route {
///     // FromRouteSegments must be implemented for any types you use in the route segment
///     // When you spread the route, you can parse multiple values from the route
///     // This url will be in the format `/123/456/789`
///     #[route("/:..numeric_route_segments")]
///     Home {
///         numeric_route_segments: NumericRouteSegments,
///     },
/// }
///
/// // We can derive Default for NumericRouteSegments
/// // If the router fails to parse the route segment, it will use the default value instead
/// #[derive(Default, Clone, PartialEq, Debug)]
/// struct NumericRouteSegments {
///     numbers: Vec<i32>,
/// }
///
/// // Implement ToRouteSegments for NumericRouteSegments so that we can display the route segments
/// impl ToRouteSegments for NumericRouteSegments {
///     fn display_route_segments(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         for number in &self.numbers {
///             write!(f, "/{}", number)?;
///         }
///         Ok(())
///     }
/// }
///
/// // We also need to parse the route segments with `FromRouteSegments`
/// impl FromRouteSegments for NumericRouteSegments {
///     type Err = <i32 as std::str::FromStr>::Err;
///
///     fn from_route_segments(segments: &[&str]) -> Result<Self, Self::Err> {
///         let mut numbers = Vec::new();
///         for segment in segments {
///             numbers.push(segment.parse()?);
///         }
///         Ok(NumericRouteSegments { numbers })
///     }
/// }
///
/// # #[component]
/// # fn Home(numeric_route_segments: NumericRouteSegments) -> Element {
/// #     unimplemented!()
/// # }
/// ```
#[rustversion::attr(
    since(1.78.0),
    diagnostic::on_unimplemented(
        message = "`FromRouteSegments` is not implemented for `{Self}`",
        label = "spread route segments",
        note = "FromRouteSegments is automatically implemented for types that implement `FromIterator` with an `Item` type that implements `Display`. You need to either implement FromIterator or implement FromRouteSegments manually."
    )
)]
pub trait FromRouteSegments: Sized {
    /// The error that can occur when parsing route segments.
    type Err: std::fmt::Display;

    /// Create an instance of `Self` from route segments.
    ///
    /// NOTE: This method must parse the output of `ToRouteSegments::display_route_segments` into the type `Self`.
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
#[non_exhaustive]
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

impl SegmentType {
    /// Try to convert this segment into a static segment.
    pub fn to_static(&self) -> Option<&'static str> {
        match self {
            SegmentType::Static(s) => Some(*s),
            _ => None,
        }
    }

    /// Try to convert this segment into a dynamic segment.
    pub fn to_dynamic(&self) -> Option<&'static str> {
        match self {
            SegmentType::Dynamic(s) => Some(*s),
            _ => None,
        }
    }

    /// Try to convert this segment into a catch all segment.
    pub fn to_catch_all(&self) -> Option<&'static str> {
        match self {
            SegmentType::CatchAll(s) => Some(*s),
            _ => None,
        }
    }

    /// Try to convert this segment into a child segment.
    pub fn to_child(&self) -> Option<()> {
        match self {
            SegmentType::Child => Some(()),
            _ => None,
        }
    }
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
