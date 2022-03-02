/// A `RouteContext` is a context that is provided by [`Route`](fn.Route.html) components.
///
/// This signals to all child [`Route`] and [`Link`] components that they are
/// currently nested under this route.
#[derive(Debug, Clone)]
pub struct RouteContext {
    /// The `declared_route` is the sub-piece of the route that matches this pattern.
    ///
    ///
    /// It follows this pattern:
    /// ```
    /// "name/:id"
    /// ```
    pub declared_route: String,

    /// The `total_route` is the full route that matches this pattern.
    ///
    ///
    /// It follows this pattern:
    /// ```
    /// "/level0/level1/:id"
    /// ```
    pub total_route: String,
}
