//! Types relating to navigation.

/// An internal target for the router to navigate to.
#[derive(Clone)]
pub enum InternalNavigationTarget {
    /// Navigate to the specified path.
    ItPath(String),
    /// Navigate to the route with the corresponding name.
    ItName(
        /// The name of the target route.
        &'static str,
        /// A list of variables that can be inserted into the path needed to navigate to the route.
        Vec<(&'static str, String)>,
    ),
}

impl From<NavigationTarget> for InternalNavigationTarget {
    fn from(t: NavigationTarget) -> Self {
        match t {
            NavigationTarget::NtPath(p) => Self::ItPath(p),
            NavigationTarget::NtName(n, v) => Self::ItName(n, v),
            NavigationTarget::NtExternal(_) => panic!(
                "NavigationTarget::RExternal cannot be converted to InternalNavigationTarget"
            ),
        }
    }
}

/// A target for the router to navigate to.
#[derive(Clone)]
pub enum NavigationTarget {
    /// Navigate to the specified path.
    NtPath(String),
    /// Navigate to the route with the corresponding name.
    NtName(
        /// The name of the target route.
        &'static str,
        /// A list of variables that can be inserted into the path needed to navigate to the route.
        Vec<(&'static str, String)>,
    ),
    /// Navigate to an external page.
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

/// A specific path segment. Used to
#[derive(Clone)]
pub(crate) enum NamedNavigationSegment {
    Fixed(String),
    Variable(&'static str),
}
