//! Types relating to navigation.

/// An internal target for the router to navigate to.
#[derive(Clone)]
pub enum InternalNavigationTarget {
    /// Navigate to the specified path.
    IPath(String),
    /// Navigate to the route with the corresponding name.
    IName(
        /// The name of the target route.
        &'static str,
        /// A list of variables that can be inserted into the path needed to navigate to the route.
        Vec<(&'static str, String)>,
    ),
}

impl From<NavigationTarget> for InternalNavigationTarget {
    fn from(t: NavigationTarget) -> Self {
        match t {
            NavigationTarget::RPath(p) => Self::IPath(p),
            NavigationTarget::RName(n, v) => Self::IName(n, v),
            NavigationTarget::RExternal(_) => panic!(
                "NavigationTarget::RExternal cannot be converted to InternalNavigationTarget"
            ),
        }
    }
}

/// A target for the router to navigate to.
#[derive(Clone)]
pub enum NavigationTarget {
    /// Navigate to the specified path.
    RPath(String),
    /// Navigate to the route with the corresponding name.
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
