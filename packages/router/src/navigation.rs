//! Types relating to navigation.

/// A target for the router to navigate to.
#[derive(Clone)]
pub enum NavigationTarget {
    /// Navigate to the specified path
    RPath(String),
    /// Name to the route with the corresponding name.
    RName(
        /// The name of the target route.
        &'static str,
        /// A list of variables that can be inserted into the path needed to navigate to the route.
        Vec<(&'static str, String)>,
    ),
    /// Navigate to an external page.
    RExternal(String),
}

impl NavigationTarget {
    /// Returns `true` if the navigation target is [`RExternal`].
    ///
    /// [`RExternal`]: NavigationTarget::RExternal
    #[must_use]
    pub fn is_rexternal(&self) -> bool {
        matches!(self, Self::RExternal(..))
    }
}

#[derive(Clone)]
pub(crate) enum NamedNavigationSegment {
    Fixed(String),
    Variable(&'static str),
}
