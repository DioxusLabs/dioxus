//! Launch helper macros for fullstack apps
#![allow(unused)]
use std::any::Any;

use crate::prelude::*;
use dioxus_core::prelude::*;
use dioxus_core::ComponentFunction;

pub trait ClonableAny: Any {
    fn clone_box(&self) -> Box<dyn ClonableAny>;
}

impl<T: Any + Clone> ClonableAny for T {
    fn clone_box(&self) -> Box<dyn ClonableAny> {
        Box::new(self.clone())
    }
}

/// The platform-independent part of the config needed to launch an application.
pub struct CrossPlatformConfig<F: ComponentFunction<P>, P> {
    /// The root component function.
    pub component: F,
    /// The props for the root component.
    pub props: P,
    /// The contexts to provide to the root component.
    pub root_contexts: Vec<Box<dyn ClonableAny>>,
}

pub trait PlatformBuilder<P> {
    type Config;

    /// Launch the app.
    fn launch<F: ComponentFunction<P>>(config: CrossPlatformConfig<F, P>, config: Self::Config);
}

impl<P> PlatformBuilder<P> for () {
    type Config = ();

    fn launch<F: ComponentFunction<P>>(config: CrossPlatformConfig<F, P>, _: ()) {}
}

/// A builder for a fullstack app.
pub struct LaunchBuilder<F: ComponentFunction<P>, P> {
    cross_platform_config: CrossPlatformConfig<F, P>,
}

impl<F: ComponentFunction<P>, P> LaunchBuilder<F, P> {
    /// Create a new builder for your application.
    pub fn new(component: F) -> Self
    where
        P: Default,
    {
        Self {
            cross_platform_config: CrossPlatformConfig {
                component,
                props: Default::default(),
                root_contexts: vec![],
            },
        }
    }

    /// Pass some props to your application.
    pub fn props(mut self, props: P) -> Self {
        self.cross_platform_config.props = props;
        self
    }

    /// Inject state into the root component's context.
    pub fn context(mut self, state: impl ClonableAny + 'static) -> Self {
        self.cross_platform_config
            .root_contexts
            .push(Box::new(state));
        self
    }

    /// Provide a platform-specific config to the builder.
    pub fn platform_config(
        self,
        config: Option<<CurrentPlatform as PlatformBuilder<P>>::Config>,
    ) -> Self {
        self
    }

    /// Launch the app.
    pub fn launch(self) {}
}

#[cfg(feature = "router")]
impl<R: Routable> LaunchBuilder<crate::router::FullstackRouterConfig<R>>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
    R: Clone + serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
{
    /// Create a new launch builder for the given router.
    pub fn router() -> Self {
        let component = crate::router::RouteWithCfg::<R>;
        let props = crate::router::FullstackRouterConfig::default();
        Self::new_with_props(component, props)
    }
}

type CurrentPlatform = ();
