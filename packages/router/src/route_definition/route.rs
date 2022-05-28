use log::error;

use super::{RouteContent, Segment};

/// A static route.
#[derive(Clone, Debug)]
pub struct Route {
    pub(crate) content: RouteContent,
    pub(crate) name: Option<&'static str>,
    pub(crate) nested: Option<Segment>,
}

impl Route {
    /// Create a new [`Route`] with the provided `content`.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus::prelude::*;
    /// Route::new(RcNone);
    /// ```
    pub fn new(content: RouteContent) -> Self {
        Self {
            content,
            name: Default::default(),
            nested: Default::default(),
        }
    }

    /// Add a name.
    ///
    /// The name can be used for name based navigation. See [`NtName`] for more details. Make sure
    /// the name is unique among the routes passed to the [`Router`].
    ///
    /// # Panic
    /// - If the name was already set, but only in debug builds.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus::prelude::*;
    /// Route::new(RcNone).name("name");
    /// ```
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

    /// Add a nested segment.
    ///
    /// # Panic
    /// - If a nested segment was already set, but only in debug builds.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus::prelude::*;
    /// Route::new(RcNone).nested(Segment::default());
    /// ```
    pub fn nested(mut self, nested: Segment) -> Self {
        if self.nested.is_some() {
            error!(r#"nested already set, later prevails"#);
            #[cfg(debug_assertions)]
            panic!(r#"nested already set"#)
        }

        self.nested = Some(nested);
        self
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(debug_assertions))]
    use crate::route_definition::DynamicRoute;

    use super::*;

    #[test]
    fn name() {
        let r = Route::new(RouteContent::RcNone).name("test");

        assert_eq!(r.name, Some("test"));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = r#"name already set: "test" to "test2""#]
    fn name_panic_in_debug() {
        Route::new(RouteContent::RcNone).name("test").name("test2");
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn name_override_in_release() {
        let p = Route::new(RouteContent::RcNone).name("test").name("test2");

        assert_eq!(p.name, Some("test2"));
    }

    #[test]
    fn nested() {
        let r = Route::new(RouteContent::RcNone).nested(Segment::new());

        assert!(r.nested.is_some());
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = "nested already set"]
    fn nested_panic_in_debug() {
        Route::new(RouteContent::RcNone)
            .nested(Segment::new())
            .nested(Segment::new());
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn nested_override_in_release() {
        let p = Route::new(RouteContent::RcNone)
            .nested(Segment::new())
            .nested(Segment::new().fallback(RouteContent::RcNone));

        let is_correct_nested = if let Some(nest) = p.nested {
            if let DynamicRoute::Fallback(RouteContent::RcNone) = nest.dynamic {
                true
            } else {
                false
            }
        } else {
            false
        };
        assert!(is_correct_nested);
    }
}
