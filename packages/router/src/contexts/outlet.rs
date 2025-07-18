use dioxus_core::{provide_context, try_consume_context, use_hook, Element, VNode};

use crate::{routable::Routable, utils::use_router_internal::use_router_internal};

/// A context that manages nested routing levels for outlet components.
///
/// The outlet context keeps track of the current nesting level of routes and helps
/// manage the hierarchical structure of nested routes in the application.
///
/// # Type Parameters
///
/// * `R` - The routable type that implements the routing logic
#[derive(Clone, Default)]
pub struct OutletContext<R> {
    current_level: usize,
    _marker: std::marker::PhantomData<R>,
}

impl<R> OutletContext<R> {
    /// Creates a new outlet context starting at level 0
    pub fn new() -> Self {
        Self {
            current_level: 0,
            _marker: std::marker::PhantomData,
        }
    }

    /// Creates a new outlet context for the next nesting level
    pub fn next(&self) -> Self {
        Self {
            current_level: self.current_level + 1,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the current nesting level of this outlet
    pub fn level(&self) -> usize {
        self.current_level
    }

    pub(crate) fn render() -> Element
    where
        R: Routable + Clone,
    {
        let router = use_router_internal().expect("Outlet must be inside of a router");
        let outlet: OutletContext<R> = use_outlet_context();
        let current_level = outlet.level();
        provide_context(outlet.next());

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

/// Returns the current outlet context from the component hierarchy.
///
/// This hook retrieves the outlet context from the current component scope. If no context is found,
/// it creates a new context with a default level of 0.
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
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use dioxus_router::use_outlet_context;
///
/// # #[derive(Routable,Clone,PartialEq)]
/// # enum MyRouter {
/// #   #[route("/")]
/// #   MyView
/// # }
///
/// # #[component]
/// # fn MyView() -> Element {
/// #   rsx!{ div { "My Text" } }
/// # }
///
/// let outlet_ctx = use_outlet_context::<MyRouter>();
/// println!("Current nesting level: {}", outlet_ctx.level());
/// ```
pub fn use_outlet_context<R: Clone + 'static>() -> OutletContext<R> {
    use_hook(|| try_consume_context().unwrap_or_else(OutletContext::new))
}
