use std::any::{type_name, TypeId};

use log::error;

use super::{RouteContent, Segment};

/// A static route.
#[derive(Debug)]
pub struct Route {
    pub(crate) content: RouteContent,
    pub(crate) name: Option<(TypeId, &'static str)>,
    pub(crate) nested: Option<Segment>,
}

impl Route {
    /// Create a new [`Route`] with the provided `content`.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus_router::prelude::*;
    /// Route::new(());
    /// ```
    pub fn new(content: impl Into<RouteContent>) -> Self {
        Self {
            content: content.into(),
            name: Default::default(),
            nested: Default::default(),
        }
    }

    /// Add a name.
    ///
    /// The name can be used for name based navigation. See [`NamedTarget`] for more details. Make
    /// sure the name is unique among the routes passed to the [`Router`].
    ///
    /// **IMPORTANT:** the actual value of the argument is ignored. The only important factor is its
    /// type.
    ///
    /// # Panic
    /// - If the name was already set, but only in debug builds.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus_router::prelude::*;
    /// struct Name;
    /// Route::new(()).name(Name);
    /// ```
    ///
    /// [`NamedTarget`]: crate::navigation::NavigationTarget::NamedTarget
    /// [`Router`]: crate::components::Router
    pub fn name<T: 'static>(mut self, _: T) -> Self {
        let name = type_name::<T>();
        if let Some((_, existing_name)) = self.name {
            error!(r#"name already set: "{existing_name}" to "{name}", later prevails"#);
            #[cfg(debug_assertions)]
            panic!(r#"name already set: "{existing_name}" to "{name}""#);
        }

        self.name = Some((TypeId::of::<T>(), name));
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
    /// Route::new(()).nested(Segment::default());
    /// ```
    pub fn nested(mut self, nested: impl Into<Segment>) -> Self {
        if self.nested.is_some() {
            error!(r#"nested already set, later prevails"#);
            #[cfg(debug_assertions)]
            panic!(r#"nested already set"#)
        }

        self.nested = Some(nested.into());
        self
    }
}

impl<T: Into<RouteContent>> From<T> for Route {
    fn from(c: T) -> Self {
        Self::new(c.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::named_tuple;
    #[cfg(not(debug_assertions))]
    use crate::navigation::NavigationTarget;

    struct Test;
    struct Test2;

    #[test]
    fn name() {
        let r = Route::new(RouteContent::Empty).name(Test);

        assert_eq!(r.name, Some(named_tuple(Test)));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = r#"name already set: "dioxus_router::route_definition::route::tests::Test" to "dioxus_router::route_definition::route::tests::Test2""#]
    fn name_panic_in_debug() {
        Route::new(RouteContent::Empty).name(Test).name(Test2);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn name_override_in_release() {
        let p = Route::new(RouteContent::Empty).name(Test).name(Test2);

        assert_eq!(p.name, Some(named_tuple(Test2)));
    }

    #[test]
    fn nested() {
        let r = Route::new(RouteContent::Empty).nested(Segment::new());

        assert!(r.nested.is_some());
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = "nested already set"]
    fn nested_panic_in_debug() {
        Route::new(RouteContent::Empty)
            .nested(Segment::new())
            .nested(Segment::new());
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn nested_override_in_release() {
        let p = Route::new(RouteContent::Empty)
            .nested(Segment::new())
            .nested(Segment::new().fallback("test"));

        let is_correct_nested = if let Some(nest) = p.nested {
            if let RouteContent::Redirect(NavigationTarget::InternalTarget(target)) = nest.fallback
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
