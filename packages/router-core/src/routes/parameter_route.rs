use super::{RouteContent, Segment};
use crate::{
    prelude::{
        FailureExternalNavigation, FailureNamedNavigation, FailureRedirectionLimit, RootIndex,
    },
    Name,
};

/// A parameter route.
#[derive(Debug)]
pub struct ParameterRoute<T: Clone> {
    pub(crate) content: Option<RouteContent<T>>,
    pub(crate) name: Option<Name>,
    pub(crate) nested: Option<Segment<T>>,
    pub(crate) key: Name,
}

impl<T: Clone> ParameterRoute<T> {
    /// Create a new [`ParameterRoute`] with `N` as the key.
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::ParameterRoute;
    /// let route: ParameterRoute<&'static str> = ParameterRoute::empty::<bool>();
    /// ```
    ///
    /// **Note:** The dioxus-router-core documentation and tests mostly use standard Rust types. This is only
    /// for brevity. It is recommend to use types with descriptive keys, and create unit structs if
    /// needed.
    pub fn empty<N: 'static>() -> Self {
        Self {
            content: None,
            name: None,
            nested: None,
            key: Name::of::<N>(),
        }
    }

    /// Create a new [`ParameterRoute`] with `N` as the key and some `content`.
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, ParameterRoute};
    /// let route = ParameterRoute::content::<bool>(ContentAtom("some content"));
    /// ```
    ///
    /// **Note:** The dioxus-router-core documentation and tests mostly use standard Rust types. This is only
    /// for brevity. It is recommend to use types with descriptive names, and create unit structs if
    /// needed.
    pub fn content<N: 'static>(content: impl Into<RouteContent<T>>) -> Self {
        Self {
            content: Some(content.into()),
            name: None,
            nested: None,
            key: Name::of::<N>(),
        }
    }

    /// Create a new [`ParameterRoute`] with `N` as the key and possibly some `content`.
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, ParameterRoute};
    /// let route = ParameterRoute::new::<bool>(Some(ContentAtom("some content")));
    /// ```
    ///
    /// **Note:** The dioxus-router-core documentation and tests mostly use standard Rust types. This is only
    /// for brevity. It is recommend to use types with descriptive names, and create unit structs if
    /// needed.
    pub fn new<N: 'static>(content: Option<impl Into<RouteContent<T>>>) -> Self {
        match content {
            Some(c) => Self::content::<N>(c),
            None => Self::empty::<N>(),
        }
    }

    /// Add a name to a [`ParameterRoute`].
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, ParameterRoute};
    /// ParameterRoute::content::<bool>(ContentAtom("some content")).name::<bool>();
    /// ```
    ///
    /// Names must be unique within their top level [`Segment`]. [`RootIndex`],
    /// [`FailureExternalNavigation`], [`FailureNamedNavigation`] and [`FailureRedirectionLimit`]
    /// are reserved and **may not** be used.
    ///
    /// **Note:** The dioxus-router-core documentation and tests mostly use standard Rust types. This is only
    /// for brevity. It is recommend to use types with descriptive names, and create unit structs if
    /// needed.
    ///
    /// # Error Handling
    /// 1. This function may only be called once per [`ParameterRoute`]. In _debug mode_, the second
    ///    call will panic. In _release mode_, all calls after the first will be ignored.
    /// 2. If one of the forbidden names (see above) is used, this function will panic, even in
    ///    _release mode_.
    pub fn name<N: 'static>(mut self) -> Self {
        let new = Name::of::<N>();

        debug_assert!(
            self.name.is_none(),
            "name cannot be changed: {} to {new}",
            self.name.as_ref().unwrap(),
        );
        assert_ne!(new, Name::of::<RootIndex>(), "forbidden name: {new}");
        assert_ne!(
            new,
            Name::of::<FailureExternalNavigation>(),
            "forbidden name: {new}"
        );
        assert_ne!(
            new,
            Name::of::<FailureNamedNavigation>(),
            "forbidden name: {new}"
        );
        assert_ne!(
            new,
            Name::of::<FailureRedirectionLimit>(),
            "forbidden name: {new}"
        );

        self.name.get_or_insert(new);
        self
    }

    /// Add a nested [`Segment`] to the [`ParameterRoute`].
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, ParameterRoute, Segment};
    /// ParameterRoute::content::<bool>(ContentAtom("some content")).nested(Segment::empty());
    /// ```
    ///
    /// # Error Handling
    /// This function may only be called once per [`ParameterRoute`]. In _debug mode_, the second
    /// call will panic. In _release mode_, all calls after the first will be ignored.
    pub fn nested(mut self, nested: impl Into<Segment<T>>) -> Self {
        debug_assert!(self.nested.is_none(), "nested segment cannot be changed");
        self.nested.get_or_insert(nested.into());
        self
    }
}

impl<T: Clone, C: Into<RouteContent<T>>, N: 'static> From<(C, N)> for ParameterRoute<T> {
    fn from((c, _): (C, N)) -> Self {
        Self::content::<N>(c)
    }
}

#[cfg(test)]
mod tests {
    use crate::routes::{test_content, ContentAtom};

    use super::*;

    #[test]
    fn empty() {
        let p = ParameterRoute::<&str>::empty::<String>();

        assert!(p.content.is_none());
        assert!(p.name.is_none());
        assert!(p.nested.is_none());
        assert_eq!(p.key, Name::of::<String>());
    }

    #[test]
    fn content() {
        let p = ParameterRoute::content::<String>(test_content());

        assert_eq!(p.content, Some(test_content()));
        assert!(p.name.is_none());
        assert!(p.nested.is_none());
        assert_eq!(p.key, Name::of::<String>());
    }

    #[test]
    fn new_empty() {
        let p = ParameterRoute::<&str>::new::<String>(None::<String>);

        assert!(p.content.is_none());
        assert!(p.name.is_none());
        assert!(p.nested.is_none());
        assert_eq!(p.key, Name::of::<String>());
    }

    #[test]
    fn new_content() {
        let p = ParameterRoute::new::<String>(Some(test_content()));

        assert_eq!(p.content, Some(test_content()));
        assert!(p.name.is_none());
        assert!(p.nested.is_none());
        assert_eq!(p.key, Name::of::<String>());
    }

    #[test]
    fn name_initial() {
        let route = ParameterRoute::<&str>::empty::<String>().name::<&str>();

        assert_eq!(route.name, Some(Name::of::<&str>()))
    }

    #[test]
    #[should_panic = "name cannot be changed: alloc::string::String to &str"]
    #[cfg(debug_assertions)]
    fn name_debug() {
        ParameterRoute::<&str>::empty::<String>()
            .name::<String>()
            .name::<&str>();
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn name_release() {
        let route = ParameterRoute::<&str>::empty::<bool>()
            .name::<String>()
            .name::<&str>();

        assert_eq!(route.name, Some(Name::of::<String>()));
    }

    #[test]
    #[should_panic = "forbidden name: dioxus_router_core::prelude::RootIndex"]
    fn name_root_index() {
        ParameterRoute::<&str>::empty::<&str>().name::<RootIndex>();
    }

    #[test]
    #[should_panic = "forbidden name: dioxus_router_core::prelude::FailureExternalNavigation"]
    fn name_external_navigation() {
        ParameterRoute::<&str>::empty::<&str>().name::<FailureExternalNavigation>();
    }

    #[test]
    #[should_panic = "forbidden name: dioxus_router_core::prelude::FailureNamedNavigation"]
    fn name_named_navigation() {
        ParameterRoute::<&str>::empty::<&str>().name::<FailureNamedNavigation>();
    }

    #[test]
    #[should_panic = "forbidden name: dioxus_router_core::prelude::FailureRedirectionLimit"]
    fn name_redirection_limit() {
        ParameterRoute::<&str>::empty::<&str>().name::<FailureRedirectionLimit>();
    }

    #[test]
    fn nested_initial() {
        let route = ParameterRoute::empty::<bool>().nested(nested_segment());
        assert!(route.nested.is_some());

        let n = route.nested.unwrap();
        assert_eq!(n.index, nested_segment().index);
    }

    #[test]
    #[should_panic = "nested segment cannot be changed"]
    #[cfg(debug_assertions)]
    fn nested_debug() {
        ParameterRoute::empty::<bool>()
            .nested(nested_segment())
            .nested(nested_segment());
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn nested_release() {
        let route = ParameterRoute::empty::<bool>()
            .nested(nested_segment())
            .nested(Segment::empty());
        assert!(route.nested.is_some());

        let n = route.nested.unwrap();
        assert_eq!(n.index, nested_segment().index);
    }

    fn nested_segment() -> Segment<&'static str> {
        Segment::content(RouteContent::Content(ContentAtom("nested")))
    }
}
