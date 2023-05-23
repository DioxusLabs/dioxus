use dioxus::prelude::*;
use log::error;
use std::{cell::RefCell, str::FromStr};

use crate::{
    prelude::{outlet::OutletContext, RouterContext},
    routable::Routable,
    router_cfg::RouterConfiguration,
};

/// The props for [`Router`].
#[derive(Props)]
pub struct RouterProps<R: Routable> {
    #[props(into)]
    initial_url: Option<String>,
    #[props(default, into)]
    config: RefCell<Option<RouterConfiguration<R>>>,
}

impl<R: Routable> PartialEq for RouterProps<R> {
    fn eq(&self, _: &Self) -> bool {
        // prevent the router from re-rendering when the initial url or config changes
        true
    }
}

/// A component that renders the current route.
pub fn Router<R: Routable + Clone>(cx: Scope<RouterProps<R>>) -> Element
where
    <R as FromStr>::Err: std::fmt::Display,
{
    let router = use_context_provider(cx, || {
        #[allow(unreachable_code, unused_variables)]
        if let Some(outer) = cx.consume_context::<RouterContext<R>>() {
            let msg = "Router components should not be nested within each other";
            error!("{msg}, inner will be inactive and transparent");
            #[cfg(debug_assertions)]
            panic!("{}", msg);
        }
        let router = RouterContext::new(
            cx.props.config.take().unwrap_or_default(),
            cx.schedule_update_any(),
        );
        if let Some(initial) = cx.props.initial_url.as_ref() {
            router.replace(
                initial
                    .parse()
                    .unwrap_or_else(|_| panic!("failed to parse initial url")),
            );
        }
        router
    });

    use_context_provider(cx, || OutletContext { current_level: 1 });

    router.current().render(cx, 0)
}
