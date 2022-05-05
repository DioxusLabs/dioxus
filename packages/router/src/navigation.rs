/// A target for the router to navigate to.
#[derive(Clone)]
pub enum NavigationTarget {
    /// Navigate to the specified path.
    ///
    /// If the path starts with a `/` it is treated as an absolute path. Otherwise it is treated as
    /// relative.
    NtPath(String),
    /// Navigate to the route with the corresponding name.
    ///
    /// If the router doesn't know the provided name, it will navigate to
    /// [`PATH_FOR_NAMED_NAVIGATION_FAILURE`].
    ///
    /// [`PATH_FOR_NAMED_NAVIGATION_FAILURE`]: crate::PATH_FOR_NAMED_NAVIGATION_FAILURE
    NtName(
        /// The name of the target route.
        &'static str,
        /// A list of parameters.
        ///
        /// The contained values will be used to construct the actual path as needed.
        Vec<(&'static str, String)>,
        /// The query string.
        Query,
    ),
    /// Navigate to an external page.
    ///
    /// If the [`HistoryProvider`] used by the [`Router`] doesn't support [`NtExternal`], the router
    /// will navigate to [`PATH_FOR_NAMED_NAVIGATION_FAILURE`]. The URL the [`NtExternal`] provided
    /// will be provided in the query string as `url`.
    ///
    /// [`HistoryProvider`]: crate::history::HistoryProvider
    /// [`NtExternal`]: NavigationTarget::NtExternal
    /// [`PATH_FOR_NAMED_NAVIGATION_FAILURE`]: crate::PATH_FOR_NAMED_NAVIGATION_FAILURE
    /// [`Router`]: crate::components::Router
    NtExternal(String),
}

impl NavigationTarget {
    /// Returns `true` if the navigation target is [`NtExternal`].
    ///
    /// [`NtExternal`]: NavigationTarget::NtExternal
    #[must_use]
    pub fn is_nt_external(&self) -> bool {
        matches!(self, Self::NtExternal(..))
    }
}

/// A query string.
#[derive(Clone)]
pub enum Query {
    /// No query string.
    QNone,
    /// The query string is the provided string.
    QString(Option<String>),
    /// Construct a new query string from the provided key value pairs.
    QVec(Vec<(String, String)>),
}

/// A specific path segment. Used to construct a path during named navigation.
#[derive(Clone)]
pub(crate) enum NamedNavigationSegment {
    /// A fixed path.
    Fixed(String),
    /// A parameter to be inserted.
    Parameter(&'static str),
}
