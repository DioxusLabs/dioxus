use std::{collections::BTreeMap, fmt::Debug};

use crate::{navigation::NavigationTarget, Name};

use super::ContentAtom;

/// The content of a route.
#[derive(Clone)]
pub enum RouteContent<T: Clone> {
    /// Some actual content.
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, RouteContent};
    /// let explicit = RouteContent::Content(ContentAtom("content"));
    /// let implicit: RouteContent<_> = ContentAtom("content").into();
    /// assert_eq!(explicit, implicit);
    /// ```
    Content(ContentAtom<T>),
    /// A redirect to another location.
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::RouteContent;
    /// let explicit = RouteContent::<&'static str>::Redirect("/some_path".into());
    /// let implicit: RouteContent<&'static str> = "/some_path".into();
    /// assert_eq!(explicit, implicit);
    /// ```
    Redirect(NavigationTarget),
    /// Multiple content.
    ///
    /// This may contain some main content, and named content.
    ///
    /// ```rust
    /// # use std::collections::BTreeMap;
    /// # use dioxus_router_core::{Name, routes::{ContentAtom, multi, RouteContent}};
    /// let explicit = RouteContent::MultiContent{
    ///     main: Some(ContentAtom("main")),
    ///     named: {
    ///         let mut r = BTreeMap::new();
    ///         r.insert(Name::of::<u8>(), ContentAtom("first"));
    ///         r.insert(Name::of::<u16>(), ContentAtom("second"));
    ///         r
    ///     }
    /// };
    /// let implicit = multi(Some(ContentAtom("main")))
    ///     .add_named::<u8>(ContentAtom("first"))
    ///     .add_named::<u16>(ContentAtom("second"));
    /// assert_eq!(explicit, implicit);
    /// ```
    MultiContent {
        /// The main content.
        main: Option<ContentAtom<T>>,
        /// Named content.
        named: BTreeMap<Name, ContentAtom<T>>,
    },
}

impl<T: Clone> RouteContent<T> {
    /// Create a new [`RouteContent::MultiContent`].
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, multi, RouteContent};
    /// let content = multi(Some(ContentAtom("main")))
    ///     .add_named::<u8>(ContentAtom("first"))
    ///     .add_named::<u16>(ContentAtom("second"));
    /// ```
    pub fn multi(main: Option<ContentAtom<T>>) -> Self {
        Self::MultiContent {
            main,
            named: BTreeMap::new(),
        }
    }

    /// Add some named content to a [`RouteContent::MultiContent`].
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, multi, RouteContent};
    /// let content = multi(Some(ContentAtom("main")))
    ///     .add_named::<u8>(ContentAtom("first"))
    ///     .add_named::<u16>(ContentAtom("second"));
    /// ```
    ///
    /// **Note:** The dioxus-router-core documentation and tests mostly use standard Rust types. This is only
    /// for brevity. It is recommend to use types with descriptive names, and create unit structs if
    /// needed, like this.
    ///
    /// # Error Handling
    /// An error occurs if `self` is any other [`RouteContent`] variant then
    /// [`RouteContent::MultiContent`]. In _debug mode_, this will trigger a panic. In _release
    /// mode_ nothing will happen.
    pub fn add_named<N: 'static>(mut self, content: ContentAtom<T>) -> Self {
        debug_assert!(
            matches!(self, Self::MultiContent { main: _, named: _ }),
            "add_named only available for MultiContent"
        );

        if let Self::MultiContent { main: _, named } = &mut self {
            let name = Name::of::<N>();
            debug_assert!(
                !named.contains_key(&name),
                "name not unique within MultiContent: {name}"
            );
            named.entry(name).or_insert(content);
        }

        self
    }
}

/// Create a new [`RouteContent::MultiContent`].
///
/// ```rust
/// # use dioxus_router_core::routes::{ContentAtom, multi, RouteContent};
/// let content = multi(Some(ContentAtom("main")))
///     .add_named::<u8>(ContentAtom("first"))
///     .add_named::<u16>(ContentAtom("second"));
/// ```
///
/// This is a shortcut for [`RouteContent`]s `multi` method.
pub fn multi<T: Clone>(main: Option<ContentAtom<T>>) -> RouteContent<T> {
    RouteContent::multi(main)
}

#[cfg(test)]
pub(crate) fn test_content() -> RouteContent<&'static str> {
    RouteContent::Content(ContentAtom("test content"))
}

impl<T: Clone> From<ContentAtom<T>> for RouteContent<T> {
    fn from(c: ContentAtom<T>) -> Self {
        Self::Content(c)
    }
}

impl<T: Clone, N: Into<NavigationTarget>> From<N> for RouteContent<T> {
    fn from(nt: N) -> Self {
        Self::Redirect(nt.into())
    }
}

impl<T: Clone> Debug for RouteContent<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Content(_) => f.debug_tuple("Content").finish(),
            Self::Redirect(nt) => f.debug_tuple("Target").field(nt).finish(),
            Self::MultiContent { main: _, named } => {
                f.debug_tuple("MultiContent").field(&named.keys()).finish()
            }
        }
    }
}

impl<T: Clone + PartialEq> PartialEq for RouteContent<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Content(l0), Self::Content(r0)) => l0 == r0,
            (Self::Redirect(l), Self::Redirect(r)) => l == r,
            (
                Self::MultiContent {
                    main: lm,
                    named: ln,
                },
                Self::MultiContent {
                    main: rm,
                    named: rn,
                },
            ) => lm == rm && ln == rn,
            _ => false,
        }
    }
}

impl<T: Clone + Eq> Eq for RouteContent<T> {}

#[cfg(test)]
mod tests {
    use crate::{navigation::named, Name};

    use super::*;

    #[test]
    fn content_from_content() {
        assert_eq!(
            Into::<RouteContent<_>>::into(ContentAtom("test content")),
            test_content()
        )
    }

    #[test]
    fn content_from_target() {
        assert_eq!(
            Into::<RouteContent<_>>::into(named::<bool>()),
            RouteContent::<&str>::Redirect(NavigationTarget::Named {
                name: Name::of::<bool>(),
                parameters: Default::default(),
                query: None
            })
        )
    }

    #[test]
    fn content_from_string() {
        let internal = "/test";
        assert_eq!(
            Into::<RouteContent<&str>>::into(internal.to_string()),
            RouteContent::Redirect(internal.into())
        );

        let external = "https://dioxuslabs.com/";
        assert_eq!(
            Into::<RouteContent<&str>>::into(external.to_string()),
            RouteContent::Redirect(external.into())
        )
    }

    #[test]
    fn content_from_str() {
        let internal = "/test";
        assert_eq!(
            Into::<RouteContent<&str>>::into(internal),
            RouteContent::Redirect(internal.into())
        );

        let external = "https://dioxuslabs.com/";
        assert_eq!(
            Into::<RouteContent<&str>>::into(external),
            RouteContent::Redirect(external.into())
        )
    }

    #[test]
    fn multi() {
        let c = RouteContent::multi(Some(ContentAtom("test")));
        match c {
            RouteContent::MultiContent { main, named } => {
                assert_eq!(main, Some(ContentAtom("test")));
                assert!(named.is_empty());
            }
            _ => panic!("wrong kind"),
        };
    }

    #[test]
    fn multi_add() {
        let c = RouteContent::multi(None)
            .add_named::<u8>(ContentAtom("1"))
            .add_named::<u16>(ContentAtom("2"));

        match c {
            RouteContent::MultiContent { main, named } => {
                assert!(main.is_none());
                assert_eq!(named, {
                    let mut r = BTreeMap::new();
                    r.insert(Name::of::<u8>(), ContentAtom("1"));
                    r.insert(Name::of::<u16>(), ContentAtom("2"));
                    r
                });
            }
            _ => panic!("wrong kind"),
        };
    }

    #[test]
    #[should_panic = "add_named only available for MultiContent"]
    #[cfg(debug_assertions)]
    fn multi_add_wrong_kind_debug() {
        RouteContent::Content(ContentAtom("1")).add_named::<u8>(ContentAtom("2"));
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn multi_add_wrong_kind_release() {
        assert_eq!(
            RouteContent::Content(ContentAtom("1")).add_named::<u8>(ContentAtom("2")),
            RouteContent::Content(ContentAtom("1"))
        );
    }

    #[test]
    #[should_panic = "name not unique within MultiContent: u8"]
    #[cfg(debug_assertions)]
    fn multi_add_duplicate_debug() {
        RouteContent::multi(None)
            .add_named::<u8>(ContentAtom("1"))
            .add_named::<u8>(ContentAtom("2"));
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn multi_add_duplicate_release() {
        let c = RouteContent::multi(None)
            .add_named::<u8>(ContentAtom("1"))
            .add_named::<u8>(ContentAtom("2"));

        match c {
            RouteContent::MultiContent { main, named } => {
                assert!(main.is_none());
                assert_eq!(named, {
                    let mut r = BTreeMap::new();
                    r.insert(Name::of::<u8>(), ContentAtom("1"));
                    r
                });
            }
            _ => panic!("wrong kind"),
        };
    }
}
