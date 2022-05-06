use log::error;

use super::{RouteContent, Segment};

/// A static route.
#[derive(Clone)]
pub struct Route {
    pub(crate) content: RouteContent,
    pub(crate) name: Option<&'static str>,
    pub(crate) sub: Option<Segment>,
}

impl Route {
    /// Create a new [`Route`] with the provided `content`.
    pub fn new(content: RouteContent) -> Self {
        Self {
            content,
            name: Default::default(),
            sub: Default::default(),
        }
    }

    /// Add a name.
    ///
    /// The name can be used for name based navigation. See [`NtName`] for more details. Make sure
    /// the name is unique among the routes passed to the [`Router`].
    ///
    /// # Panic
    /// If the name was already set, but only in debug builds.
    ///
    /// [`NtName`]: crate::navigation::NavigationTarget::NtName
    /// [`Router`]: crate::components::Router
    pub fn name(mut self, name: &'static str) -> Self {
        if let Some(existing_name) = self.name {
            error!(r#"name already set: "{existing_name}" to "{name}", later prevails"#);
            #[cfg(debug_assertions)]
            panic!(r#"name already set: "{existing_name}" to "{name}""#);
        }

        self.name = Some(name);
        self
    }

    /// Add a subsegment.
    ///
    /// # Panic
    /// If the subsegment was already set, but only in debug builds.
    pub fn sub(mut self, sub: Segment) -> Self {
        if self.sub.is_some() {
            error!(r#"sub already set, later prevails"#);
            #[cfg(debug_assertions)]
            panic!(r#"sub already set"#)
        }

        self.sub = Some(sub);
        self
    }
}
