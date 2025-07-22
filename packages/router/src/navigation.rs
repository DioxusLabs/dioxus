//! Types pertaining to navigation.

use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use url::{ParseError, Url};

use crate::{
    components::child_router::consume_child_route_mapping, hooks::try_router, routable::Routable,
};

impl<R: Routable> From<R> for NavigationTarget {
    fn from(value: R) -> Self {
        // If this is a child route, map it to the root route first
        let mapping = consume_child_route_mapping();
        match mapping.as_ref() {
            Some(mapping) => NavigationTarget::Internal(mapping.format_route_as_root_route(value)),
            // Otherwise, just use the internal route
            None => NavigationTarget::Internal(value.to_string()),
        }
    }
}

/// A target for the router to navigate to.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NavigationTarget<R = String> {
    /// An internal path that the router can navigate to by itself.
    ///
    /// ```rust
    /// # use dioxus::prelude::*;
    /// # use dioxus_router::navigation::NavigationTarget;
    /// # #[component]
    /// # fn Index() -> Element {
    /// #     unreachable!()
    /// # }
    /// #[derive(Clone, Routable, PartialEq, Debug)]
    /// enum Route {
    ///     #[route("/")]
    ///     Index {},
    /// }
    /// let explicit = NavigationTarget::Internal(Route::Index {});
    /// let implicit: NavigationTarget::<Route> = "/".parse().unwrap();
    /// assert_eq!(explicit, implicit);
    /// ```
    Internal(R),
    /// An external target that the router doesn't control.
    ///
    /// ```rust
    /// # use dioxus::prelude::*;
    /// # use dioxus_router::navigation::NavigationTarget;
    /// # #[component]
    /// # fn Index() -> Element {
    /// #     unreachable!()
    /// # }
    /// #[derive(Clone, Routable, PartialEq, Debug)]
    /// enum Route {
    ///     #[route("/")]
    ///     Index {},
    /// }
    /// let explicit = NavigationTarget::<Route>::External(String::from("https://dioxuslabs.com/"));
    /// let implicit: NavigationTarget::<Route> = "https://dioxuslabs.com/".parse().unwrap();
    /// assert_eq!(explicit, implicit);
    /// ```
    External(String),
}

impl<R: Routable> From<&str> for NavigationTarget<R> {
    fn from(value: &str) -> Self {
        value
            .parse()
            .unwrap_or_else(|_| Self::External(value.to_string()))
    }
}

impl<R: Routable> From<&String> for NavigationTarget<R> {
    fn from(value: &String) -> Self {
        value.as_str().into()
    }
}

impl<R: Routable> From<String> for NavigationTarget<R> {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

impl<R: Routable> From<R> for NavigationTarget<R> {
    fn from(value: R) -> Self {
        Self::Internal(value)
    }
}

impl From<&str> for NavigationTarget {
    fn from(value: &str) -> Self {
        match try_router() {
            Some(router) => match router.internal_route(value) {
                true => NavigationTarget::Internal(value.to_string()),
                false => NavigationTarget::External(value.to_string()),
            },
            None => NavigationTarget::External(value.to_string()),
        }
    }
}

impl From<String> for NavigationTarget {
    fn from(value: String) -> Self {
        match try_router() {
            Some(router) => match router.internal_route(&value) {
                true => NavigationTarget::Internal(value),
                false => NavigationTarget::External(value),
            },
            None => NavigationTarget::External(value.to_string()),
        }
    }
}

impl<R: Routable> From<NavigationTarget<R>> for NavigationTarget {
    fn from(value: NavigationTarget<R>) -> Self {
        match value {
            NavigationTarget::Internal(r) => r.into(),
            NavigationTarget::External(s) => Self::External(s),
        }
    }
}

impl<R: Routable> Display for NavigationTarget<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NavigationTarget::Internal(r) => write!(f, "{}", r),
            NavigationTarget::External(s) => write!(f, "{}", s),
        }
    }
}

/// An error that can occur when parsing a [`NavigationTarget`].
pub enum NavigationTargetParseError<R: Routable> {
    /// A URL that is not valid.
    InvalidUrl(ParseError),
    /// An internal URL that is not valid.
    InvalidInternalURL(<R as FromStr>::Err),
}

impl<R: Routable> Debug for NavigationTargetParseError<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NavigationTargetParseError::InvalidUrl(e) => write!(f, "Invalid URL: {}", e),
            NavigationTargetParseError::InvalidInternalURL(_) => {
                write!(f, "Invalid internal URL")
            }
        }
    }
}

impl<R: Routable> FromStr for NavigationTarget<R> {
    type Err = NavigationTargetParseError<R>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Url::parse(s) {
            Ok(_) => Ok(Self::External(s.to_string())),
            Err(ParseError::RelativeUrlWithoutBase) => {
                Ok(Self::Internal(R::from_str(s).map_err(|e| {
                    NavigationTargetParseError::InvalidInternalURL(e)
                })?))
            }
            Err(e) => Err(NavigationTargetParseError::InvalidUrl(e)),
        }
    }
}
