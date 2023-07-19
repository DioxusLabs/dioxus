use dioxus::prelude::*;
use std::{cell::RefCell, str::FromStr};

use crate::{
    prelude::{GenericOutlet, GenericRouterContext},
    routable::Routable,
    router_cfg::RouterConfig,
};

/// The config for [`GenericRouter`].
pub struct RouterConfigFactory<R: Routable> {
    #[allow(clippy::type_complexity)]
    config: RefCell<Option<Box<dyn FnOnce() -> RouterConfig<R>>>>,
}

#[cfg(feature = "serde")]
impl<R: Routable> Default for RouterConfigFactory<R>
where
    <R as FromStr>::Err: std::fmt::Display,
    R: serde::Serialize + serde::de::DeserializeOwned,
{
    fn default() -> Self {
        Self::from(RouterConfig::default)
    }
}

#[cfg(not(feature = "serde"))]
impl<R: Routable> Default for RouterConfigFactory<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    fn default() -> Self {
        Self::from(RouterConfig::default)
    }
}

impl<R: Routable, F: FnOnce() -> RouterConfig<R> + 'static> From<F> for RouterConfigFactory<R> {
    fn from(value: F) -> Self {
        Self {
            config: RefCell::new(Some(Box::new(value))),
        }
    }
}

#[cfg(feature = "serde")]
/// The props for [`GenericRouter`].
#[derive(Props)]
pub struct GenericRouterProps<R: Routable>
where
    <R as FromStr>::Err: std::fmt::Display,
    R: serde::Serialize + serde::de::DeserializeOwned,
{
    #[props(default, into)]
    config: RouterConfigFactory<R>,
}

#[cfg(not(feature = "serde"))]
/// The props for [`GenericRouter`].
#[derive(Props)]
pub struct GenericRouterProps<R: Routable>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    #[props(default, into)]
    config: RouterConfigFactory<R>,
}

#[cfg(not(feature = "serde"))]
impl<R: Routable> Default for GenericRouterProps<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    fn default() -> Self {
        Self {
            config: RouterConfigFactory::default(),
        }
    }
}

#[cfg(feature = "serde")]
impl<R: Routable> Default for GenericRouterProps<R>
where
    <R as FromStr>::Err: std::fmt::Display,
    R: serde::Serialize + serde::de::DeserializeOwned,
{
    fn default() -> Self {
        Self {
            config: RouterConfigFactory::default(),
        }
    }
}

#[cfg(not(feature = "serde"))]
impl<R: Routable> PartialEq for GenericRouterProps<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    fn eq(&self, _: &Self) -> bool {
        // prevent the router from re-rendering when the initial url or config changes
        true
    }
}

#[cfg(feature = "serde")]
impl<R: Routable> PartialEq for GenericRouterProps<R>
where
    <R as FromStr>::Err: std::fmt::Display,
    R: serde::Serialize + serde::de::DeserializeOwned,
{
    fn eq(&self, _: &Self) -> bool {
        // prevent the router from re-rendering when the initial url or config changes
        true
    }
}

#[cfg(not(feature = "serde"))]
/// A component that renders the current route.
pub fn GenericRouter<R: Routable + Clone>(cx: Scope<GenericRouterProps<R>>) -> Element
where
    <R as FromStr>::Err: std::fmt::Display,
{
    use crate::prelude::outlet::OutletContext;

    use_context_provider(cx, || {
        GenericRouterContext::new(
            (cx.props
                .config
                .config
                .take()
                .expect("use_context_provider ran twice"))(),
            cx.schedule_update_any(),
        )
    });
    use_context_provider(cx, || OutletContext::<R> {
        current_level: 0,
        _marker: std::marker::PhantomData,
    });

    render! {
        GenericOutlet::<R> {}
    }
}

#[cfg(feature = "serde")]
/// A component that renders the current route.
pub fn GenericRouter<R: Routable + Clone>(cx: Scope<GenericRouterProps<R>>) -> Element
where
    <R as FromStr>::Err: std::fmt::Display,
    R: serde::Serialize + serde::de::DeserializeOwned,
{
    use_context_provider(cx, || {
        GenericRouterContext::new(
            (cx.props
                .config
                .config
                .take()
                .expect("use_context_provider ran twice"))(),
            cx.schedule_update_any(),
        )
    });
    use_context_provider(cx, || OutletContext::<R> {
        current_level: 0,
        _marker: std::marker::PhantomData,
    });

    render! {
        GenericOutlet::<R> {}
    }
}
