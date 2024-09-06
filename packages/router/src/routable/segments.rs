use dioxus_lib::prelude::*;

use std::iter::FlatMap;
use std::slice::Iter;
use std::{fmt::Display, str::FromStr};

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
