use dioxus::prelude::*;
use log::error;
use std::{cell::RefCell, str::FromStr};

use crate::{
    prelude::{Outlet, RouterContext},
    routable::Routable,
    router_cfg::RouterConfiguration,
};

/// The config for [`Router`].
pub struct RouterCfg<R: Routable> {
    config: RefCell<Option<RouterConfiguration<R>>>,
}

impl<R: Routable> Default for RouterCfg<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    fn default() -> Self {
        Self {
            config: RefCell::new(Some(RouterConfiguration::default())),
        }
    }
}

impl<R: Routable> From<RouterConfiguration<R>> for RouterCfg<R> {
    fn from(value: RouterConfiguration<R>) -> Self {
        Self {
            config: RefCell::new(Some(value)),
        }
    }
}

/// The props for [`Router`].
#[derive(Props)]
pub struct RouterProps<R: Routable>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    #[props(into)]
    initial_url: Option<String>,
    #[props(default, into)]
    config: RouterCfg<R>,
}

impl<R: Routable> PartialEq for RouterProps<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
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
    use_context_provider(cx, || {
        #[allow(unreachable_code, unused_variables)]
        if let Some(outer) = cx.consume_context::<RouterContext<R>>() {
            let msg = "Router components should not be nested within each other";
            error!("{msg}, inner will be inactive and transparent");
            #[cfg(debug_assertions)]
            panic!("{}", msg);
        }
        let router = RouterContext::new(
            cx.props.config.config.take().unwrap_or_default(),
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

    render! {
        Outlet::<R> {}
    }
}
