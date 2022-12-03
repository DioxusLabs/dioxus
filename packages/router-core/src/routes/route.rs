use super::{RouteContent, Segment};
use crate::{
    prelude::{
        FailureExternalNavigation, FailureNamedNavigation, FailureRedirectionLimit, RootIndex,
    },
    Name,
};

/// A fixed route.
#[derive(Debug)]
pub struct Route<T: Clone> {
    pub(crate) content: Option<RouteContent<T>>,
    pub(crate) name: Option<Name>,
    pub(crate) nested: Option<Segment<T>>,
}

impl<T: Clone> Route<T> {
    /// Create a new [`Route`] without content.
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::Route;
    /// let r: Route<&'static str> = Route::empty();
    /// ```
    #[must_use]
    pub fn empty() -> Self {
        Self {
            content: None,
            name: None,
            nested: None,
        }
    }

    /// Create a new [`Route`] with some `content`.
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, Route};
    /// let r = Route::content(ContentAtom("some content"));
    /// ```
    #[must_use]
    pub fn content(content: impl Into<RouteContent<T>>) -> Self {
        Self {
            content: Some(content.into()),
            name: None,
            nested: None,
        }
    }

    /// Create a new [`Route`], possible with some `content`.
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, Route};
    /// let r = Route::new(Some(ContentAtom("some content")));
    /// ```
    #[must_use]
    pub fn new(content: Option<impl Into<RouteContent<T>>>) -> Self {
        match content {
            Some(c) => Self::content(c),
            None => Self::empty(),
        }
    }

    /// Add a name to a [`Route`].
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, Route};
    /// Route::content(ContentAtom("some content")).name::<bool>();
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
    /// 1. This function may only be called once per [`Route`]. In _debug mode_, the second call
    ///    will panic. In _release mode_, all calls after the first will be ignored.
    /// 2. If one of the forbidden names (see above) is used, this function will panic, even in
    ///    _release mode_.
    pub fn name<N: 'static>(mut self) -> Self {
        let new = Name::of::<N>();

        debug_assert!(
            self.name.is_none(),
            "name cannot be changed: {} to {new}",
            self.name.as_ref().unwrap(),
        );
        if new == Name::of::<RootIndex>()
            || new == Name::of::<FailureExternalNavigation>()
            || new == Name::of::<FailureNamedNavigation>()
            || new == Name::of::<FailureRedirectionLimit>()
        {
            panic!("forbidden name: {new}");
        }

        self.name.get_or_insert(new);
        self
    }

    /// Add a nested [`Segment`] to the [`Route`].
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, Route, Segment};
    /// Route::content(ContentAtom("some content")).nested(Segment::empty());
    /// ```
    ///
    /// # Error Handling
    /// This function may only be called once per [`Route`]. In _debug mode_, the second call will
    /// panic. In _release mode_, all calls after the first will be ignored.
    pub fn nested(mut self, nested: impl Into<Segment<T>>) -> Self {
        debug_assert!(self.nested.is_none(), "nested segment cannot be changed");
        self.nested.get_or_insert(nested.into());
        self
    }
}

impl<T: Clone, C: Into<RouteContent<T>>> From<C> for Route<T> {
    fn from(c: C) -> Self {
        Self::content(c)
    }
}

#[cfg(test)]
mod tests {
    use crate::routes::{test_content, ContentAtom};

    use super::*;

    #[test]
    fn empty() {
        let route = Route::<&str>::empty();

        assert!(route.content.is_none());
        assert!(route.name.is_none());
        assert!(route.nested.is_none());
    }

    #[test]
    fn content() {
        let route = Route::content(test_content());

        assert_eq!(route.content, Some(test_content()));
        assert!(route.name.is_none());
        assert!(route.nested.is_none());
    }

    #[test]
    fn new_empty() {
        let route = Route::<&str>::new(None::<&str>);

        assert!(route.content.is_none());
        assert!(route.name.is_none());
        assert!(route.nested.is_none());
    }

    #[test]
    fn new_content() {
        let route = Route::new(Some(test_content()));

        assert_eq!(route.content, Some(test_content()));
        assert!(route.name.is_none());
        assert!(route.nested.is_none());
    }

    #[test]
    fn name_initial() {
        let route = Route::<&str>::empty().name::<&str>();

        assert_eq!(route.name, Some(Name::of::<&str>()))
    }

    #[test]
    #[should_panic = "name cannot be changed: alloc::string::String to &str"]
    #[cfg(debug_assertions)]
    fn name_debug() {
        Route::<&str>::empty().name::<String>().name::<&str>();
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn name_release() {
        let route = Route::<&str>::empty().name::<String>().name::<&str>();

        assert_eq!(route.name, Some(Name::of::<String>()));
    }

    #[test]
    #[should_panic = "forbidden name: dioxus_router_core::prelude::RootIndex"]
    fn name_root_index() {
        Route::<&str>::empty().name::<RootIndex>();
    }

    #[test]
    #[should_panic = "forbidden name: dioxus_router_core::prelude::FailureExternalNavigation"]
    fn name_external_navigation() {
        Route::<&str>::empty().name::<FailureExternalNavigation>();
    }

    #[test]
    #[should_panic = "forbidden name: dioxus_router_core::prelude::FailureNamedNavigation"]
    fn name_named_navigation() {
        Route::<&str>::empty().name::<FailureNamedNavigation>();
    }

    #[test]
    #[should_panic = "forbidden name: dioxus_router_core::prelude::FailureRedirectionLimit"]
    fn name_redirection_limit() {
        Route::<&str>::empty().name::<FailureRedirectionLimit>();
    }

    #[test]
    fn nested_initial() {
        let route = Route::empty().nested(nested_segment());
        assert!(route.nested.is_some());

        let n = route.nested.unwrap();
        assert_eq!(n.index, nested_segment().index);
    }

    #[test]
    #[should_panic = "nested segment cannot be changed"]
    #[cfg(debug_assertions)]
    fn nested_debug() {
        Route::empty()
            .nested(nested_segment())
            .nested(nested_segment());
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn nested_release() {
        let route = Route::empty()
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
