//! Types pertaining to navigation.

use std::{fmt::Display, str::FromStr};

use url::{ParseError, Url};

use crate::routable::Routable;

/// A target for the router to navigate to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationTarget<R: Routable> {
    /// An internal path that the router can navigate to by itself.
    ///
    /// ```rust
    /// # use dioxus_router_core::navigation::NavigationTarget;
    /// let explicit = NavigationTarget::Internal(String::from("/internal"));
    /// let implicit: NavigationTarget = "/internal".into();
    /// assert_eq!(explicit, implicit);
    /// ```
    Internal(R),
    /// An external target that the router doesn't control.
    ///
    /// ```rust
    /// # use dioxus_router_core::navigation::NavigationTarget;
    /// let explicit = NavigationTarget::External(String::from("https://dioxuslabs.com/"));
    /// let implicit: NavigationTarget = "https://dioxuslabs.com/".into();
    /// assert_eq!(explicit, implicit);
    /// ```
    External(String),
}

impl<R: Routable> From<R> for NavigationTarget<R> {
    fn from(value: R) -> Self {
        Self::Internal(value)
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
