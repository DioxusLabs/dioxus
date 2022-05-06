use log::error;

use super::{RouteContent, Segment};

/// A dynamic route.
///
/// It is recommended to construct this enum via [`DynamicRoute::none`], [`DynamicRoute::parameter`]
/// and [`DynamicRoute::fallback`].
#[derive(Clone)]
pub enum DynamicRoute {
    /// Indicates the absence of an actual dynamic route.
    None,
    /// A dynamic route making the value accessible as a parameter.
    Parameter {
        /// The name of the route. See [`DynamicRoute::name`] for more details.
        name: Option<&'static str>,
        /// The key for the value. See [`DynamicRoute::parameter`] for more details.
        key: &'static str,
        /// The content.
        content: RouteContent,
        /// The subsegment. See [`DynamicRoute::sub`] for more details.
        sub: Option<Box<Segment>>,
    },
    /// A fallback that is rendered when no other route matches.
    Fallback(RouteContent),
}

impl DynamicRoute {
    /// Create a new [`DynamicRoute::Parameter`] with the provided `key` and `content`.
    ///
    /// The `key` will be the key the corresponding value is accessible under, using [`use_route`].
    ///
    /// [`use_route`]: crate::hooks::use_route
    pub fn parameter(key: &'static str, content: RouteContent) -> Self {
        Self::Parameter {
            name: Default::default(),
            key,
            content,
            sub: Default::default(),
        }
    }

    /// Create a new [`DynamicRoute::Fallback`].
    pub fn fallback(content: RouteContent) -> Self {
        Self::Fallback(content)
    }

    /// Create a new [`DynamicRoute::None`].
    pub fn none() -> Self {
        Self::None
    }

    /// Add a name to a [`DynamicRoute::Parameter`].
    ///
    /// The name can be used for name based navigation. See [`NtName`] for more details. Make sure
    /// the name is unique among the routes passed to the [`Router`].
    ///
    /// # Panic
    /// - If `self` is not a [`DynamicRoute::Parameter`], but only in debug builds.
    /// - If the name was already set, but only in debug builds.
    ///
    /// [`NtName`]: crate::navigation::NavigationTarget::NtName
    /// [`Router`]: crate::components::Router
    pub fn name(mut self, name: &'static str) -> Self {
        if let DynamicRoute::Parameter {
            name: existing_name,
            key: _,
            content: _,
            sub: _,
        } = &mut self
        {
            if let Some(existing_name) = existing_name {
                error!(r#"name already set: "{existing_name}" to "{name}", later prevails"#);
                #[cfg(debug_assertions)]
                panic!(r#"name already set: "{existing_name}" to "{name}""#);
            }
            *existing_name = Some(name);
        } else {
            error!(r#"name can only be set for `DynamicRoute::Parameter`: "{name}""#);
            #[cfg(debug_assertions)]
            panic!(r#"name can only be set for `DynamicRoute::Parameter`: "{name}""#);
        }

        self
    }

    /// Add a subsegment to a [`DynamicRoute::Parameter`].
    ///
    /// # Panic
    /// - If `self` is not a [`DynamicRoute::Parameter`], but only in debug builds.
    /// - If the subsegment was already set, but only in debug builds.
    pub fn sub(mut self, sub: Segment) -> Self {
        if let DynamicRoute::Parameter {
            name: _,
            key: _,
            content: _,
            sub: existing_sub,
        } = &mut self
        {
            if existing_sub.is_some() {
                error!(r#"sub already set, later prevails"#);
                #[cfg(debug_assertions)]
                panic!(r#"sub already set"#);
            }
            *existing_sub = Some(Box::new(sub));
        } else {
            error!(r#"sub can only be set for `DynamicRoute::Parameter`"#);
            #[cfg(debug_assertions)]
            panic!(r#"sub can only be set for `DynamicRoute::Parameter`"#);
        }

        self
    }
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
