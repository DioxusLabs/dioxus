use super::{ParameterRoute, RouteContent};

/// A _fallback_ route or _parameter_ route.
///
/// Used internally by [`Segment`](crate::route_definition::Segment) to store either one.
#[derive(Clone)]
pub(crate) enum DynamicRoute {
    None,
    Parameter(ParameterRoute),
    Fallback(RouteContent),
}

impl DynamicRoute {
    /// Returns `true` if the dynamic route is [`None`].
    ///
    /// [`None`]: DynamicRoute::None
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl Default for DynamicRoute {
    fn default() -> Self {
        Self::None
    }
}
