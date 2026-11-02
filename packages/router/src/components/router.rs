use crate::{Outlet, provide_router_context, routable::Routable, router_cfg::RouterConfig};
use dioxus_vdom::{Callback, Element, provide_context, try_consume_context, use_hook};
use dioxus_vdom_macro::{Props, rsx};
use std::rc::Rc;

/// A context callback invoked when the suspense boundary above the initial [`Router`] resolves.
///
/// Provide this context above the router to be notified when its initial route is ready.
/// If there is no suspense boundary, the callback runs when the router first renders.
#[derive(Clone)]
pub struct OnInitialRouteResolved(Rc<dyn Fn()>);

impl OnInitialRouteResolved {
    /// Create a callback to run when the initial route resolves.
    pub fn new(callback: impl Fn() + 'static) -> Self {
        Self(Rc::new(callback))
    }
}

/// The props for [`Router`].
#[derive(Props)]
pub struct RouterProps<R: Clone + 'static> {
    #[props(default, into)]
    config: Callback<(), RouterConfig<R>>,
}

impl<T: Clone> Clone for RouterProps<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Clone> Copy for RouterProps<T> {}

impl<R: Clone + 'static> Default for RouterProps<R> {
    fn default() -> Self {
        Self {
            config: Callback::new(|_| RouterConfig::default()),
        }
    }
}

impl<R: Clone> PartialEq for RouterProps<R> {
    fn eq(&self, _: &Self) -> bool {
        // prevent the router from re-rendering when the initial url or config changes
        true
    }
}

/// A component that renders the current route.
pub fn Router<R: Routable + Clone>(props: RouterProps<R>) -> Element {
    use crate::{RouterContext, outlet::OutletContext};

    use_hook(|| {
        provide_router_context(RouterContext::new(props.config.call(())));
    });

    let on_initial_route_resolved = use_hook(try_consume_context::<OnInitialRouteResolved>);
    dioxus_hooks::use_after_suspense_resolved(move || {
        if let Some(callback) = on_initial_route_resolved {
            (callback.0)();
        }
    });

    use_hook(|| {
        provide_context(OutletContext::<R>::new());
    });

    rsx! { Outlet::<R> {} }
}
