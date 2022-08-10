use std::{any::TypeId, str::FromStr};

use serde::Serialize;
use url::{ParseError, Url};

use crate::helpers::named_tuple;

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
    /// If the router doesn't know the provided name, it will navigate to `/` and show some fallback
    /// content. The default error message can be replaced by setting the
    /// `fallback_named_navigation` prop on the [`Router`] component.
    ///
    /// [`Router`]: crate::components::Router
    NamedTarget(
        /// The name (type id and readable name) of the target route.
        (TypeId, &'static str),
        /// A list of parameters.
        ///
        /// The contained values will be used to construct the actual path as needed.
        Vec<(&'static str, String)>,
        /// The query.
        Option<Query>,
    ),
    /// Navigate to an external page.
    ///
    /// If the [`HistoryProvider`] used by the [`Router`] doesn't support [`ExternalTarget`], the
    /// router will navigate to `/` and show some fallback content. The URL of the target is
    /// provided via the `url` parameter. The default error message can be replaced by setting the
    /// `fallback_external_navigation` prop on the [`Router`].
    ///
    /// [`HistoryProvider`]: crate::history::HistoryProvider
    /// [`ExternalTarget`]: NavigationTarget::ExternalTarget
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

impl From<String> for NavigationTarget {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl From<&str> for NavigationTarget {
    fn from(s: &str) -> Self {
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

impl<T, P> From<(T, P)> for NavigationTarget
where
    T: 'static,
    P: IntoIterator<Item = (&'static str, String)>,
{
    fn from((name, parameters): (T, P)) -> Self {
        Self::NamedTarget(named_tuple(name), parameters.into_iter().collect(), None)
    }
}

impl<T, P, Q> From<(T, P, Q)> for NavigationTarget
where
    T: 'static,
    P: IntoIterator<Item = (&'static str, String)>,
    Q: Into<Query>,
{
    fn from((name, parameters, query): (T, P, Q)) -> Self {
        (name, parameters, Some(query)).into()
    }
}

impl<T, P, Q> From<(T, P, Option<Q>)> for NavigationTarget
where
    T: 'static,
    P: IntoIterator<Item = (&'static str, String)>,
    Q: Into<Query>,
{
    fn from((name, parameters, query): (T, P, Option<Q>)) -> Self {
        Self::NamedTarget(
            named_tuple(name),
            parameters.into_iter().collect(),
            query.map(Into::into),
        )
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
    pub fn from_serde(query: impl Serialize) -> Result<Self, serde_urlencoded::ser::Error> {
        serde_urlencoded::to_string(query).map(Self::QueryString)
    }
}

impl From<String> for Query {
    fn from(query: String) -> Self {
        Self::QueryString(query)
    }
}

impl From<&str> for Query {
    fn from(query: &str) -> Self {
        query.to_string().into()
    }
}

impl<T> From<Vec<(T, T)>> for Query
where
    T: Into<String>,
{
    fn from(params: Vec<(T, T)>) -> Self {
        Self::QueryVec(
            params
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
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
