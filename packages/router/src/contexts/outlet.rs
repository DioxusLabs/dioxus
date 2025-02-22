use dioxus_lib::prelude::*;

use crate::{routable::Routable, utils::use_router_internal::use_router_internal};

/// A context that manages nested routing levels for outlet components.
///
/// The outlet context keeps track of the current nesting level of routes and helps
/// manage the hierarchical structure of nested routes in the application.
///
/// # Type Parameters
///
/// * `R` - The routable type that implements the routing logic
///
/// # Fields
///
/// * `current_level` - The current nesting level of the route
/// * `_marker` - Phantom data to hold the generic type parameter
///
/// # Examples
///
/// ```rust
/// let outlet = OutletContext {
///     current_level: 1,
///     _marker: std::marker::PhantomData,
/// };
/// ```
pub struct OutletContext<R> {
    /// The current nesting level of the route in the outlet hierarchy.
    /// Level 0 represents the root route, and each nested route increases the level by 1.
    pub current_level: usize,
    /// Phantom data marker to hold the generic type parameter `R`.
    /// This field is not used at runtime and has zero size.
    pub _marker: std::marker::PhantomData<R>,
}

impl<R> Clone for OutletContext<R> {
    fn clone(&self) -> Self {
        OutletContext {
            current_level: self.current_level,
            _marker: std::marker::PhantomData,
        }
    }
}
/// Returns the current outlet context from the component hierarchy.
///
/// This hook retrieves the outlet context from the current component scope. If no context is found,
/// it creates a new context with a default level of 1.
///
/// # Type Parameters
///
/// * `R` - The routable type that implements the routing logic
///
/// # Returns
///
/// Returns an [`OutletContext<R>`] containing the current nesting level information.
///
/// # Examples
///
/// ```rust
/// let outlet_ctx = use_outlet_context::<MyRouter>();
/// println!("Current nesting level: {}", outlet_ctx.current_level);
/// ```
pub fn use_outlet_context<R: 'static>() -> OutletContext<R> {
    use_hook(|| {
        try_consume_context().unwrap_or(OutletContext::<R> {
            current_level: 1,
            _marker: std::marker::PhantomData,
        })
    })
}

impl<R> OutletContext<R> {
    pub(crate) fn render() -> Element
    where
        R: Routable + Clone,
    {
        let router = use_router_internal().expect("Outlet must be inside of a router");
        let outlet: OutletContext<R> = use_outlet_context();
        let current_level = outlet.current_level;
        provide_context({
            OutletContext::<R> {
                current_level: current_level + 1,
                _marker: std::marker::PhantomData,
            }
        });

        if let Some(error) = router.render_error() {
            return if current_level == 0 {
                error
            } else {
                VNode::empty()
            };
        }

        router.current::<R>().render(current_level)
    }
}
