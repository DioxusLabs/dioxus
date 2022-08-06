use std::str::FromStr;

use serde::Serialize;
use url::{ParseError, Url};

/// A target for the router to navigate to.
#[derive(Clone, Debug)]
pub enum NavigationTarget {
    /// Navigate to the specified path.
    ///
    /// If the path starts with a `/` it is treated as an absolute path. Otherwise it is treated as
    /// relative.
    InternalTarget(String),
    /// Navigate to the route with the corresponding name.
    ///
    /// If the router doesn't know the provided name, it will navigate to
    /// [`PATH_FOR_NAMED_NAVIGATION_FAILURE`](crate::PATH_FOR_NAMED_NAVIGATION_FAILURE).
    NamedTarget(
        /// The name of the target route.
        &'static str,
        /// A list of parameters.
        ///
        /// The contained values will be used to construct the actual path as needed.
        Vec<(&'static str, String)>,
        /// The query.
        Option<Query>,
    ),
    /// Navigate to an external page.
    ///
    /// If the [`HistoryProvider`] used by the [`Router`] doesn't support [`ExternalTarget`], the router
    /// will navigate to [`PATH_FOR_EXTERNAL_NAVIGATION_FAILURE`]. The URL the [`ExternalTarget`]
    /// provided will be provided in the query string as `url`.
    ///
    /// [`HistoryProvider`]: crate::history::HistoryProvider
    /// [`ExternalTarget`]: NavigationTarget::ExternalTarget
    /// [`PATH_FOR_EXTERNAL_NAVIGATION_FAILURE`]: crate::PATH_FOR_EXTERNAL_NAVIGATION_FAILURE
    /// [`Router`]: crate::components::Router
    ExternalTarget(String),
}

impl NavigationTarget {
    /// Returns [`true`] if the navigation target is [`ExternalTarget`].
    ///
    /// [`ExternalTarget`]: NavigationTarget::ExternalTarget
    #[must_use]
    pub fn is_external_target(&self) -> bool {
        matches!(self, Self::ExternalTarget(..))
    }
}

impl From<&'static str> for NavigationTarget {
    fn from(s: &'static str) -> Self {
        s.parse()
            .unwrap_or_else(|_| Self::InternalTarget(s.to_string()))
    }
}

impl FromStr for NavigationTarget {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Url::parse(s) {
            Ok(_) => Ok(Self::ExternalTarget(s.to_string())),
            Err(ParseError::RelativeUrlWithoutBase) => Ok(Self::InternalTarget(s.to_string())),
            Err(e) => Err(e),
        }
    }
}

/// A query string.
#[derive(Clone, Debug)]
pub enum Query {
    /// The query string is the provided string.
    QueryString(String),
    /// Construct a new query string from the provided key value pairs.
    QueryVec(Vec<(String, String)>),
}

impl Query {
    /// Create a [`Query`] from a [`Serialize`]able value.
    #[must_use]
    pub fn from_serde(query: impl Serialize) -> Result<Self, serde_urlencoded::ser::Error> {
        serde_urlencoded::to_string(query).map(|q| Self::QueryString(q))
    }
}

/// A specific path segment. Used to construct a path during named navigation.
#[derive(Clone, Debug)]
pub(crate) enum NamedNavigationSegment {
    /// A fixed path.
    Fixed(String),
    /// A parameter to be inserted.
    Parameter(&'static str),
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn nt_from_str_external() {
        let targets = vec!["https://dioxuslabs.com/", "ftp://dioxuslabs.com/"];

        for t in targets {
            let nt: NavigationTarget = t.parse().unwrap();

            assert!(nt.is_external_target());
            if let NavigationTarget::ExternalTarget(url) = nt {
                assert_eq!(url, t);
            }
        }
    }

    #[test]
    fn nt_from_str_internal() {
        let target = "/some/route";
        let nt: NavigationTarget = target.parse().unwrap();

        assert!(matches!(nt, NavigationTarget::InternalTarget(_)));
        if let NavigationTarget::InternalTarget(path) = nt {
            assert_eq!(path, target);
        }
    }
}
