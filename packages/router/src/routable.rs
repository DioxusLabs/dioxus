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

/// Something that can be created from an entire query string. This trait must be implemented for any type that is spread into the query segment like `#[route("/?:..query")]`.
///
///
/// **This trait is automatically implemented for any types that implement `From<&str>`.**
///
/// ```rust
/// use dioxus::prelude::*;
///
/// #[derive(Routable, Clone, PartialEq, Debug)]
/// enum Route {
///     // FromQuery must be implemented for any types you spread into the query segment
///     #[route("/?:..query")]
///     Home {
///         query: CustomQuery
///     },
/// }
///
/// #[derive(Default, Clone, PartialEq, Debug)]
/// struct CustomQuery {
///     count: i32,
/// }
///
/// // We implement From<&str> for CustomQuery so that FromQuery is implemented automatically
/// impl From<&str> for CustomQuery {
///     fn from(query: &str) -> Self {
///         CustomQuery {
///             count: query.parse().unwrap_or(0),
///         }
///     }
/// }
///
/// // We also need to implement Display for CustomQuery which will be used to format the query string into the URL
/// impl std::fmt::Display for CustomQuery {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{}", self.count)
///     }
/// }
///
/// # #[component]
/// # fn Home(query: CustomQuery) -> Element {
/// #     unimplemented!()
/// # }
/// ```
#[rustversion::attr(
    since(1.78.0),
    diagnostic::on_unimplemented(
        message = "`FromQuery` is not implemented for `{Self}`",
        label = "spread query",
        note = "FromQuery is automatically implemented for types that implement `From<&str>`. You need to either implement From<&str> or implement FromQuery manually."
    )
)]
pub trait FromQuery {
    /// Create an instance of `Self` from a query string.
    fn from_query(query: &str) -> Self;
}

impl<T: for<'a> From<&'a str>> FromQuery for T {
    fn from_query(query: &str) -> Self {
        T::from(query)
    }
}

/// Something that can be created from a query argument. This trait must be implemented for any type that is used as a query argument like `#[route("/?:query")]`.
///
/// **This trait is automatically implemented for any types that implement `FromStr` and `Default`.**
///
/// ```rust
/// use dioxus::prelude::*;
///
/// #[derive(Routable, Clone, PartialEq, Debug)]
/// enum Route {
///     // FromQuerySegment must be implemented for any types you use in the query segment
///     // When you don't spread the query, you can parse multiple values form the query
///     // This url will be in the format `/?query=123&other=456`
///     #[route("/?:query&:other")]
///     Home {
///         query: CustomQuery,
///         other: i32,
///     },
/// }
///
/// // We can derive Default for CustomQuery
/// // If the router fails to parse the query value, it will use the default value instead
/// #[derive(Default, Clone, PartialEq, Debug)]
/// struct CustomQuery {
///     count: i32,
/// }
///
/// // We implement FromStr for CustomQuery so that FromQuerySegment is implemented automatically
/// impl std::str::FromStr for CustomQuery {
///     type Err = <i32 as std::str::FromStr>::Err;
///
///     fn from_str(query: &str) -> Result<Self, Self::Err> {
///         Ok(CustomQuery {
///             count: query.parse()?,
///         })
///     }
/// }
///
/// // We also need to implement Display for CustomQuery which will be used to format the query string into the URL
/// impl std::fmt::Display for CustomQuery {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{}", self.count)
///     }
/// }
///
/// # #[component]
/// # fn Home(query: CustomQuery, other: i32) -> Element {
/// #     unimplemented!()
/// # }
/// ```
#[rustversion::attr(
    since(1.78.0),
    diagnostic::on_unimplemented(
        message = "`FromQueryArgument` is not implemented for `{Self}`",
        label = "query argument",
        note = "FromQueryArgument is automatically implemented for types that implement `FromStr` and `Default`. You need to either implement FromStr and Default or implement FromQueryArgument manually."
    )
)]
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
        match T::from_str(argument) {
            Ok(result) => Ok(result),
            Err(err) => {
                tracing::error!("Failed to parse query argument: {}", err);
                Err(err)
            }
        }
    }
}

/// Something that can be created from an entire hash fragment. This must be implemented for any type that is used as a hash fragment like `#[route("/#:hash_fragment")]`.
///
///
/// **This trait is automatically implemented for any types that implement `FromStr` and `Default`.**
///
/// # Example
///
/// ```rust
/// use dioxus::prelude::*;
///
/// #[derive(Routable, Clone)]
/// #[rustfmt::skip]
/// enum Route {
///     // State is stored in the url hash
///     #[route("/#:url_hash")]
///     Home {
///         url_hash: State,
///     },
/// }
///
/// #[component]
/// fn Home(url_hash: State) -> Element {
///     unimplemented!()
/// }
///
///
/// #[derive(Clone, PartialEq, Default)]
/// struct State {
///     count: usize,
///     other_count: usize
/// }
///
/// // The hash segment will be displayed as a string (this will be url encoded automatically)
/// impl std::fmt::Display for State {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{}-{}", self.count, self.other_count)
///     }
/// }
///
/// // We need to parse the hash fragment into a struct from the string (this will be url decoded automatically)
/// impl FromHashFragment for State {
///     fn from_hash_fragment(hash: &str) -> Self {
///         let Some((first, second)) = hash.split_once('-') else {
///             // URL fragment parsing shouldn't fail. You can return a default value if you want
///             return Default::default();
///         };
///
///         let first = first.parse().unwrap();
///         let second = second.parse().unwrap();
///
///         State {
///             count: first,
///             other_count: second,
///         }
///     }
/// }
/// ```
#[rustversion::attr(
    since(1.78.0),
    diagnostic::on_unimplemented(
        message = "`FromHashFragment` is not implemented for `{Self}`",
        label = "hash fragment",
        note = "FromHashFragment is automatically implemented for types that implement `FromStr` and `Default`. You need to either implement FromStr and Default or implement FromHashFragment manually."
    )
)]
pub trait FromHashFragment {
    /// Create an instance of `Self` from a hash fragment.
    fn from_hash_fragment(hash: &str) -> Self;
}

impl<T> FromHashFragment for T
where
    T: FromStr + Default,
    T::Err: std::fmt::Display,
{
    fn from_hash_fragment(hash: &str) -> Self {
        match T::from_str(hash) {
            Ok(value) => value,
            Err(err) => {
                tracing::error!("Failed to parse hash fragment: {}", err);
                Default::default()
            }
        }
    }
}

/// Something that can be created from a single route segment. This must be implemented for any type that is used as a route segment like `#[route("/:route_segment")]`.
///
///
/// **This trait is automatically implemented for any types that implement `FromStr` and `Default`.**
///
/// ```rust
/// use dioxus::prelude::*;
///
/// #[derive(Routable, Clone, PartialEq, Debug)]
/// enum Route {
///     // FromRouteSegment must be implemented for any types you use in the route segment
///     // When you don't spread the route, you can parse multiple values from the route
///     // This url will be in the format `/123/456`
///     #[route("/:route_segment_one/:route_segment_two")]
///     Home {
///         route_segment_one: CustomRouteSegment,
///         route_segment_two: i32,
///     },
/// }
///
/// // We can derive Default for CustomRouteSegment
/// // If the router fails to parse the route segment, it will use the default value instead
/// #[derive(Default, PartialEq, Clone, Debug)]
/// struct CustomRouteSegment {
///     count: i32,
/// }
///
/// // We implement FromStr for CustomRouteSegment so that FromRouteSegment is implemented automatically
/// impl std::str::FromStr for CustomRouteSegment {
///     type Err = <i32 as std::str::FromStr>::Err;
///
///     fn from_str(route_segment: &str) -> Result<Self, Self::Err> {
///         Ok(CustomRouteSegment {
///             count: route_segment.parse()?,
///         })
///     }
/// }
///
/// // We also need to implement Display for CustomRouteSegment which will be used to format the route segment into the URL
/// impl std::fmt::Display for CustomRouteSegment {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{}", self.count)
///     }
/// }
///
/// # #[component]
/// # fn Home(route_segment_one: CustomRouteSegment, route_segment_two: i32) -> Element {
/// #     unimplemented!()
/// # }
/// ```
#[rustversion::attr(
    since(1.78.0),
    diagnostic::on_unimplemented(
        message = "`FromRouteSegment` is not implemented for `{Self}`",
        label = "route segment",
        note = "FromRouteSegment is automatically implemented for types that implement `FromStr` and `Default`. You need to either implement FromStr and Default or implement FromRouteSegment manually."
    )
)]
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
        T::from_str(route)
    }
}

#[test]
fn full_circle() {
    let route = "testing 1234 hello world";
    assert_eq!(String::from_route_segment(route).unwrap(), route);
}

/// Something that can be converted into multiple route segments. This must be implemented for any type that is spread into the route segment like `#[route("/:..route_segments")]`.
///
///
/// **This trait is automatically implemented for any types that implement `IntoIterator<Item=impl Display>`.**
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
/// #[derive(Default, PartialEq, Clone, Debug)]
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
pub trait ToRouteSegments {
    /// Display the route segments with each route segment separated by a `/`. This should not start with a `/`.
    ///
    fn display_route_segments(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

// Implement ToRouteSegments for any type that can turn &self into an iterator of &T where T: Display
impl<I, T: Display> ToRouteSegments for I
where
    for<'a> &'a I: IntoIterator<Item = &'a T>,
{
    fn display_route_segments(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for segment in self {
            write!(f, "/")?;
            let segment = segment.to_string();
            let encoded = urlencoding::encode(&segment);
            write!(f, "{}", encoded)?;
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
///     // Routes can include dynamic segments that are parsed from the url
///     #[route("/blog/:blog_id")]
///     Blog { blog_id: usize },
///     // Or query segments that are parsed from the url
///     #[route("/edit?:blog_id")]
///     Edit { blog_id: usize },
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

                route.parse().ok()
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
