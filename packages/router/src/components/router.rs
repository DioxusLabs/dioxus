use crate::{provide_router_context, routable::Routable, router_cfg::RouterConfig, Outlet};
use dioxus_core::{provide_context, use_hook, Callback, Element};
use dioxus_core_macro::{rsx, Props};

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
    use crate::{outlet::OutletContext, RouterContext};

    use_hook(|| {
        provide_router_context(RouterContext::new(props.config.call(())));
    });

    #[cfg(feature = "streaming")]
    dioxus_hooks::use_after_suspense_resolved(|| {
        dioxus_fullstack_core::commit_initial_chunk();
    });

    use_hook(|| {
        provide_context(OutletContext::<R>::new());
    });

    rsx! { Outlet::<R> {} }
}
