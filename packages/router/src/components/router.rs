use dioxus_lib::prelude::*;

use std::str::FromStr;

use crate::{
    prelude::{provide_router_context, Outlet},
    routable::Routable,
    router_cfg::RouterConfig,
};

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
pub fn Router<R: Routable + Clone>(props: RouterProps<R>) -> Element
where
    <R as FromStr>::Err: std::fmt::Display,
{
    use crate::prelude::{outlet::OutletContext, RouterContext};

    use_hook(|| {
        provide_router_context(RouterContext::new(
            props.config.call(()),
            schedule_update_any(),
        ));

        provide_context(OutletContext::<R> {
            current_level: 0,
            _marker: std::marker::PhantomData,
        });
    });

    rsx! { Outlet::<R> {} }
}
