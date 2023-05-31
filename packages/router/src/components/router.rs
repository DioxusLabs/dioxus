use dioxus::prelude::*;
use log::error;
use serde::{de::DeserializeOwned, Serialize};
use std::{cell::RefCell, str::FromStr};

use crate::{
    prelude::{GenericOutlet, GenericRouterContext},
    routable::Routable,
    router_cfg::RouterConfiguration,
};

/// The config for [`GenericRouter`].
pub struct RouterCfg<R: Routable> {
    config: RefCell<Option<RouterConfiguration<R>>>,
}

impl<R: Routable + Serialize + DeserializeOwned> Default for RouterCfg<R>
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

/// The props for [`GenericRouter`].
#[derive(Props)]
pub struct GenericRouterProps<R: Routable + Serialize + DeserializeOwned>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    #[props(default, into)]
    config: RouterCfg<R>,
}

impl<R: Routable + Serialize + DeserializeOwned> PartialEq for GenericRouterProps<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    fn eq(&self, _: &Self) -> bool {
        // prevent the router from re-rendering when the initial url or config changes
        true
    }
}

/// A component that renders the current route.
pub fn GenericRouter<R: Routable + Clone + Serialize + DeserializeOwned>(
    cx: Scope<GenericRouterProps<R>>,
) -> Element
where
    <R as FromStr>::Err: std::fmt::Display,
{
    use_context_provider(cx, || {
        #[allow(unreachable_code, unused_variables)]
        if let Some(outer) = cx.consume_context::<GenericRouterContext<R>>() {
            let msg = "Router components should not be nested within each other";
            error!("{msg}, inner will be inactive and transparent");
            #[cfg(debug_assertions)]
            panic!("{}", msg);
        }
        let router = GenericRouterContext::new(
            cx.props.config.config.take().unwrap_or_default(),
            cx.schedule_update_any(),
        );
        router
    });

    render! {
        GenericOutlet::<R> {}
    }
}
