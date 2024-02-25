use dioxus_lib::prelude::*;

use std::{cell::RefCell, rc::Rc, str::FromStr};

use crate::{prelude::Outlet, routable::Routable, router_cfg::RouterConfig};

/// The config for [`Router`].
#[derive(Clone)]
pub struct RouterConfigFactory<R: Routable> {
    #[allow(clippy::type_complexity)]
    config: Rc<RefCell<Option<Box<dyn FnOnce() -> RouterConfig<R>>>>>,
}

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
            config: Rc::new(RefCell::new(Some(Box::new(value)))),
        }
    }
}

/// The props for [`Router`].
#[derive(Props)]
pub struct RouterProps<R: Routable>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    #[props(default, into)]
    config: RouterConfigFactory<R>,
}

impl<T: Routable> Clone for RouterProps<T>
where
    <T as FromStr>::Err: std::fmt::Display,
{
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
        }
    }
}

impl<R: Routable> Default for RouterProps<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    fn default() -> Self {
        Self {
            config: RouterConfigFactory::default(),
        }
    }
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
pub fn Router<R: Routable + Clone>(props: RouterProps<R>) -> Element
where
    <R as FromStr>::Err: std::fmt::Display,
{
    use crate::prelude::{outlet::OutletContext, RouterContext};

    use_hook(|| {
        provide_context(RouterContext::new(
            (props
                .config
                .config
                .take()
                .expect("use_context_provider ran twice"))(),
            schedule_update_any(),
        ));

        provide_context(OutletContext::<R> {
            current_level: 0,
            _marker: std::marker::PhantomData,
        });
    });

    rsx! { Outlet::<R> {} }
}
