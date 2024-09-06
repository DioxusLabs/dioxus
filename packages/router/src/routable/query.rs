use std::iter::FlatMap;
use std::slice::Iter;
use std::{fmt::Display, str::FromStr};

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
