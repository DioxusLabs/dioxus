//! Types pertaining to navigation.

use std::{collections::HashMap, str::FromStr};

use url::{ParseError, Url};

/// A target for the router to navigate to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationTarget {
    /// An internal path that the router can navigate to by itself.
    ///
    /// ```rust
    /// # use dioxus_router_core::navigation::NavigationTarget;
    /// let explicit = NavigationTarget::Internal(String::from("/internal"));
    /// let implicit: NavigationTarget = "/internal".into();
    /// assert_eq!(explicit, implicit);
    /// ```
    Internal(String),
    /// An internal target that the router can navigate to by itself.
    ///
    /// ```rust
    /// # use std::collections::HashMap;
    /// # use dioxus_router_core::{Name, navigation::{named, NavigationTarget}};
    /// let mut parameters = HashMap::new();
    /// parameters.insert(Name::of::<bool>(), String::from("some parameter"));
    ///
    /// let explicit = NavigationTarget::Named {
    ///     name: Name::of::<bool>(),
    ///     parameters,
    ///     query: Some("some=query".into())
    /// };
    ///
    /// let implicit = named::<bool>().parameter::<bool>("some parameter").query("some=query");
    ///
    /// assert_eq!(explicit, implicit);
    /// ```
    ///
    /// It will automatically find the route with the matching name, insert all required parameters
    /// and add the query.
    ///
    /// **Note:** The dioxus-router-core documentation and tests mostly use standard Rust types. This is only
    /// for brevity. It is recommend to use types with descriptive names, and create unit structs if
    /// needed.
    Named {
        /// The name of the [`Route`](crate::routes::Route) or
        /// [`ParameterRoute`](crate::routes::ParameterRoute) to navigate to.
        ///
        /// **Note:** The dioxus-router-core documentation and tests mostly use standard Rust types. This is
        /// only for brevity. It is recommend to use types with descriptive names, and create unit
        /// structs if needed.
        name: Name,
        /// The parameters required to get to the specified route.
        parameters: HashMap<Name, String>,
        /// A query to add to the route.
        query: Option<Query>,
    },
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

impl NavigationTarget {
    /// Create a new [`NavigationTarget::Named`] with `N` as the name.
    ///
    /// ```rust
    /// # use dioxus_router_core::navigation::NavigationTarget;
    /// let target = NavigationTarget::named::<bool>();
    /// ```
    ///
    /// **Note:** The dioxus-router-core documentation and tests mostly use standard Rust types. This is only
    /// for brevity. It is recommend to use types with descriptive names, and create unit structs if
    /// needed.
    pub fn named<N: 'static>() -> Self {
        Self::Named {
            name: Name::of::<N>(),
            parameters: HashMap::new(),
            query: None,
        }
    }

    /// Add a parameter to a [`NavigationTarget::Named`].
    ///
    /// ```rust
    /// # use dioxus_router_core::navigation::NavigationTarget;
    /// let target = NavigationTarget::named::<bool>().parameter::<bool>("some parameter");
    /// ```
    ///
    /// **Note:** The dioxus-router-core documentation and tests mostly use standard Rust types. This is only
    /// for brevity. It is recommend to use types with descriptive names, and create unit structs if
    /// needed.
    ///
    /// # Error Handling
    /// 1. An error occurs if `self` is any other [`NavigationTarget`] variant than
    ///    [`NavigationTarget::Named`]. In _debug mode_ this will trigger a panic. In _release mode_
    ///    nothing will happen.
    /// 2. Parameters need to be unique within the [`NavigationTarget`]. In _debug mode_ the
    ///    second call with a duplicate name will panic. In _release mode_, all calls after the
    ///    first will be ignored.
    pub fn parameter<N: 'static>(mut self, value: impl Into<String>) -> Self {
        let n = Name::of::<N>();

        if let Self::Named {
            name,
            parameters,
            query: _,
        } = &mut self
        {
            debug_assert!(
                !parameters.contains_key(&n),
                "duplicate parameter: {name} - {n}",
            );
            parameters.entry(n).or_insert_with(|| value.into());
        } else {
            #[cfg(debug_assertions)]
            panic!("parameter only available for named target: {n}");
        }

        self
    }

    /// Add a parameter to a [`NavigationTarget::Named`].
    ///
    /// ```rust
    /// # use dioxus_router_core::navigation::NavigationTarget;
    /// let target = NavigationTarget::named::<bool>().query("some=query");
    /// ```
    ///
    /// # Error Handling
    /// 1. An error occurs if `self` is any other [`NavigationTarget`] variant than
    ///    [`NavigationTarget::Named`]. In _debug mode_ this will trigger a panic. In _release mode_
    ///    nothing will happen.
    /// 2. This function may only be called once per [`NavigationTarget`]. In _debug mode_, the
    ///    second call will panic. In _release mode_, all calls after the first will be ignored.
    pub fn query(mut self, query: impl Into<Query>) -> Self {
        if let Self::Named {
            name,
            parameters: _,
            query: q,
        } = &mut self
        {
            debug_assert!(q.is_none(), "query cannot be changed: {name}",);
            q.get_or_insert(query.into());
        } else {
            #[cfg(debug_assertions)]
            panic!("query only available for named target",);
        }

        self
    }
}

/// Create a new [`NavigationTarget::Named`] with `N` as the name.
///
/// ```rust
/// # use dioxus_router_core::navigation::named;
/// let target = named::<bool>();
/// ```
///
/// **Note:** The dioxus-router-core documentation and tests mostly use standard Rust types. This is only
/// for brevity. It is recommend to use types with descriptive names, and create unit structs if
/// needed.
///
/// This is a shortcut for [`NavigationTarget`]s `named` function.
pub fn named<T: 'static>() -> NavigationTarget {
    NavigationTarget::named::<T>()
}

impl FromStr for NavigationTarget {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Url::parse(s) {
            Ok(_) => Ok(Self::External(s.to_string())),
            Err(ParseError::RelativeUrlWithoutBase) => Ok(Self::Internal(s.to_string())),
            Err(e) => Err(e),
        }
    }
}

impl<T: Into<String>> From<T> for NavigationTarget {
    fn from(v: T) -> Self {
        let v = v.into();
        v.clone().parse().unwrap_or(Self::Internal(v))
    }
}

/// A representation of a query string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Query {
    /// A basic query string.
    ///
    /// ```rust
    /// # use dioxus_router_core::navigation::Query;
    /// let explicit = Query::Single(String::from("some=query"));
    /// let implicit: Query = "some=query".into();
    /// assert_eq!(explicit, implicit);
    /// ```
    Single(String),
    /// A query separated into key-value-pairs.
    ///
    /// ```rust
    /// # use dioxus_router_core::navigation::Query;
    /// let explicit = Query::List(vec![(String::from("some"), String::from("query"))]);
    /// let implicit: Query = vec![("some", "query")].into();
    /// assert_eq!(explicit, implicit);
    /// ```
    #[cfg(feature = "serde")]
    List(Vec<(String, String)>),
}

impl Query {
    /// Create a [`Query`] from a [`Serialize`](serde::Serialize)able object.
    #[cfg(feature = "serde")]
    pub fn from_serde(query: impl serde::Serialize) -> Result<Self, serde_urlencoded::ser::Error> {
        serde_urlencoded::to_string(query).map(|q| Self::Single(q))
    }
}

impl From<String> for Query {
    fn from(v: String) -> Self {
        Self::Single(v)
    }
}

impl From<&str> for Query {
    fn from(v: &str) -> Self {
        v.to_string().into()
    }
}

#[cfg(feature = "serde")]
impl<T: Into<String>> From<Vec<(T, T)>> for Query {
    fn from(v: Vec<(T, T)>) -> Self {
        Self::List(v.into_iter().map(|(a, b)| (a.into(), b.into())).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_enum() {
        assert_eq!(
            NavigationTarget::named::<bool>(),
            NavigationTarget::Named {
                name: Name::of::<bool>(),
                parameters: HashMap::new(),
                query: None
            }
        )
    }

    #[test]
    fn named_func() {
        assert_eq!(
            named::<bool>(),
            NavigationTarget::Named {
                name: Name::of::<bool>(),
                parameters: HashMap::new(),
                query: None
            }
        )
    }

    #[test]
    fn parameter() {
        assert_eq!(
            named::<bool>()
                .parameter::<i32>("integer")
                .parameter::<u32>("unsigned"),
            NavigationTarget::Named {
                name: Name::of::<bool>(),
                parameters: {
                    let mut r = HashMap::new();
                    r.insert(Name::of::<i32>(), "integer".to_string());
                    r.insert(Name::of::<u32>(), "unsigned".to_string());
                    r
                },
                query: None
            }
        )
    }

    #[test]
    #[should_panic = "duplicate parameter: bool - i32"]
    #[cfg(debug_assertions)]
    fn parameter_duplicate_debug() {
        named::<bool>()
            .parameter::<i32>("integer")
            .parameter::<i32>("duplicate");
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn parameter_duplicate_release() {
        assert_eq!(
            named::<bool>()
                .parameter::<i32>("integer")
                .parameter::<i32>("duplicate"),
            NavigationTarget::Named {
                name: Name::of::<bool>(),
                parameters: {
                    let mut r = HashMap::new();
                    r.insert(Name::of::<i32>(), "integer".to_string());
                    r
                },
                query: None
            }
        );
    }

    #[test]
    #[should_panic = "parameter only available for named target: i32"]
    #[cfg(debug_assertions)]
    fn parameter_internal_debug() {
        NavigationTarget::from("/test").parameter::<i32>("integer");
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn parameter_internal_release() {
        assert_eq!(
            NavigationTarget::from("/test").parameter::<i32>("integer"),
            NavigationTarget::from("/test")
        );
    }

    #[test]
    #[should_panic = "parameter only available for named target: i32"]
    #[cfg(debug_assertions)]
    fn parameter_external_debug() {
        NavigationTarget::from("https://dioxuslabs.com/").parameter::<i32>("integer");
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn parameter_external_release() {
        assert_eq!(
            NavigationTarget::from("https://dioxuslabs.com/").parameter::<i32>("integer"),
            NavigationTarget::from("https://dioxuslabs.com/")
        );
    }

    #[test]
    fn query() {
        assert_eq!(
            named::<bool>().query("test"),
            NavigationTarget::Named {
                name: Name::of::<bool>(),
                parameters: HashMap::new(),
                query: Some(Query::Single("test".to_string()))
            }
        )
    }

    #[test]
    #[should_panic = "query cannot be changed: bool"]
    #[cfg(debug_assertions)]
    fn query_multiple_debug() {
        named::<bool>().query("test").query("other");
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn query_multiple_release() {
        assert_eq!(
            named::<bool>().query("test").query("other"),
            NavigationTarget::Named {
                name: Name::of::<bool>(),
                parameters: HashMap::new(),
                query: Some(Query::Single("test".to_string()))
            }
        )
    }

    #[test]
    #[should_panic = "query only available for named target"]
    #[cfg(debug_assertions)]
    fn query_internal_debug() {
        NavigationTarget::from("/test").query("test");
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn query_internal_release() {
        assert_eq!(
            NavigationTarget::from("/test").query("test"),
            NavigationTarget::from("/test")
        );
    }

    #[test]
    #[should_panic = "query only available for named target"]
    #[cfg(debug_assertions)]
    fn query_external_debug() {
        NavigationTarget::from("https://dioxuslabs.com/").query("test");
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn query_external_release() {
        assert_eq!(
            NavigationTarget::from("https://dioxuslabs.com/").query("test"),
            NavigationTarget::from("https://dioxuslabs.com/")
        );
    }

    #[test]
    fn target_parse_internal() {
        assert_eq!(
            "/test".parse::<NavigationTarget>(),
            Ok(NavigationTarget::Internal("/test".to_string()))
        );
    }

    #[test]
    fn target_parse_external() {
        assert_eq!(
            "https://dioxuslabs.com/".parse::<NavigationTarget>(),
            Ok(NavigationTarget::External(
                "https://dioxuslabs.com/".to_string()
            ))
        )
    }

    #[test]
    fn target_from_str_internal() {
        assert_eq!(
            NavigationTarget::from("/test"),
            NavigationTarget::Internal("/test".to_string())
        );
    }

    #[test]
    fn target_from_str_external() {
        assert_eq!(
            NavigationTarget::from("https://dioxuslabs.com/"),
            NavigationTarget::External("https://dioxuslabs.com/".to_string())
        )
    }

    #[test]
    fn target_from_string_internal() {
        assert_eq!(
            NavigationTarget::from("/test".to_string()),
            NavigationTarget::Internal("/test".to_string())
        );
    }

    #[test]
    fn target_from_string_external() {
        assert_eq!(
            NavigationTarget::from("https://dioxuslabs.com/".to_string()),
            NavigationTarget::External("https://dioxuslabs.com/".to_string())
        )
    }

    #[test]
    fn query_from_string() {
        assert_eq!(
            Query::from("test".to_string()),
            Query::Single("test".to_string())
        )
    }

    #[test]
    fn query_from_str() {
        assert_eq!(Query::from("test"), Query::Single("test".to_string()));
    }

    #[test]
    #[cfg(feature = "serde")]
    fn query_form_vec() {
        assert_eq!(
            Query::from(vec![("test", "1234")]),
            Query::List(vec![("test".to_string(), "1234".to_string())])
        )
    }
}
