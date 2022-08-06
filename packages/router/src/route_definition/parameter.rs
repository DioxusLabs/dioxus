use dioxus::prelude::*;
use log::error;

use crate::navigation::NavigationTarget;

use super::{RouteContent, Segment};

/// A route that treats its actual value as a parameter.
#[derive(Clone, Debug)]
pub struct ParameterRoute {
    pub(crate) name: Option<&'static str>,
    pub(crate) key: &'static str,
    pub(crate) content: RouteContent,
    pub(crate) nested: Option<Box<Segment>>,
}

impl ParameterRoute {
    /// Create a new [`ParameterRoute`] with the provided `key` and `content`.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus_router::prelude::*;
    /// ParameterRoute::new("key", RcNone);
    /// ```
    pub fn new(key: &'static str, content: impl Into<RouteContent>) -> Self {
        Self {
            content: content.into(),
            name: Default::default(),
            key,
            nested: Default::default(),
        }
    }

    /// Add a name.
    ///
    /// The name can be used for name based navigation. See [`NamedTarget`] for more details. Make
    /// sure the name is unique among the routes passed to the [`Router`].
    ///
    /// # Panic
    /// - If the name was already set, but only in debug builds.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus_router::prelude::*;
    /// ParameterRoute::new("key", RcNone).name("name");
    /// ```
    ///
    /// [`NamedTarget`]: crate::navigation::NavigationTarget::NamedTarget
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

    /// Add a nested segment.
    ///
    /// # Panic
    /// - If a nested segment was already set, but only in debug builds.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus_router::prelude::*;
    /// ParameterRoute::new("key", RcNone).nested(Segment::new());
    /// ```
    pub fn nested(mut self, nested: impl Into<Segment>) -> Self {
        if self.nested.is_some() {
            error!("nested already set, later prevails");
            #[cfg(debug_assertions)]
            panic!("nested already set");
        }

        self.nested = Some(Box::new(nested.into()));
        self
    }
}

impl From<(&'static str, Component)> for ParameterRoute {
    fn from((key, c): (&'static str, Component)) -> Self {
        Self::new(key, c)
    }
}

impl From<(&'static str, NavigationTarget)> for ParameterRoute {
    fn from((key, nt): (&'static str, NavigationTarget)) -> Self {
        Self::new(key, nt)
    }
}

impl From<(&'static str, RouteContent)> for ParameterRoute {
    fn from((key, rc): (&'static str, RouteContent)) -> Self {
        Self::new(key, rc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name() {
        let r = ParameterRoute::new("", RouteContent::RcNone).name("test");

        assert_eq!(r.name, Some("test"));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = r#"name already set: "test" to "test2""#]
    fn name_panic_in_debug() {
        ParameterRoute::new("", RouteContent::RcNone)
            .name("test")
            .name("test2");
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn name_override_in_release() {
        let p = ParameterRoute::new("", RouteContent::RcNone)
            .name("test")
            .name("test2");

        assert_eq!(p.name, Some("test2"));
    }

    #[test]
    fn nested() {
        let r = ParameterRoute::new("", RouteContent::RcNone).nested(Segment::new());

        assert!(r.nested.is_some());
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = "nested already set"]
    fn nested_panic_in_debug() {
        ParameterRoute::new("", RouteContent::RcNone)
            .nested(Segment::new())
            .nested(Segment::new());
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn nested_override_in_release() {
        let p = ParameterRoute::new("", RouteContent::RcNone)
            .nested(Segment::new())
            .nested(Segment::new().fallback("test"));

        let is_correct_nested = if let Some(nest) = p.nested {
            if let RouteContent::RcRedirect(NavigationTarget::InternalTarget(target)) =
                nest.fallback
            {
                target == "test"
            } else {
                false
            }
        } else {
            false
        };
        assert!(is_correct_nested);
    }
}
