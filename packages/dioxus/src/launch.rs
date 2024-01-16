//! Launch helper macros for fullstack apps
#![allow(unused)]
use std::any::Any;

use crate::prelude::*;
use dioxus_core::prelude::*;
use dioxus_core::ComponentFunction;
use dioxus_core::{BoxedContext, CrossPlatformConfig, PlatformBuilder};

/// A builder for a fullstack app.
pub struct LaunchBuilder<
    Component: ComponentFunction<Phantom, Props = Props>,
    Props: Clone + 'static,
    Phantom: 'static,
> {
    cross_platform_config: CrossPlatformConfig<Component, Props, Phantom>,
    platform_config: Option<<CurrentPlatform as PlatformBuilder<Props>>::Config>,
}

impl<
        Component: ComponentFunction<Phantom, Props = Props>,
        Props: Clone + 'static,
        Phantom: 'static,
    > LaunchBuilder<Component, Props, Phantom>
{
    /// Create a new builder for your application.
    pub fn new(component: Component) -> Self
    where
        Props: Default,
    {
        Self {
            cross_platform_config: CrossPlatformConfig::new(
                component,
                Default::default(),
                Default::default(),
            ),
            platform_config: None,
        }
    }

    /// Pass some props to your application.
    pub fn props(mut self, props: Props) -> Self {
        self.cross_platform_config.props = props;
        self
    }

    /// Inject state into the root component's context.
    pub fn context(mut self, state: impl Any + Clone + 'static) -> Self {
        self.cross_platform_config
            .root_contexts
            .push(BoxedContext::new(state));
        self
    }

    /// Provide a platform-specific config to the builder.
    pub fn cfg(
        mut self,
        config: impl Into<Option<<CurrentPlatform as PlatformBuilder<Props>>::Config>>,
    ) -> Self {
        if let Some(config) = config.into() {
            self.platform_config = Some(config);
        }
        self
    }

    #[allow(clippy::unit_arg)]
    /// Launch the app.
    pub fn launch(self) {
        CurrentPlatform::launch(
            self.cross_platform_config,
            self.platform_config.unwrap_or_default(),
        );
    }
}

// #[cfg(feature = "router")]
// impl<R: Routable> LaunchBuilder<crate::router::FullstackRouterConfig<R>>
// where
//     <R as std::str::FromStr>::Err: std::fmt::Display,
//     R: Clone + serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
// {
//     /// Create a new launch builder for the given router.
//     pub fn router() -> Self {
//         let component = crate::router::RouteWithCfg::<R>;
//         let props = crate::router::FullstackRouterConfig::default();
//         Self::new_with_props(component, props)
//     }
// }

#[cfg(feature = "desktop")]
type CurrentPlatform = dioxus_desktop::DesktopPlatform;
#[cfg(feature = "web")]
type CurrentPlatform = dioxus_web::WebPlatform;
#[cfg(not(any(feature = "desktop", feature = "web")))]
type CurrentPlatform = ();

/// Launch your application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch<
    Component: ComponentFunction<Phantom, Props = Props>,
    Props: Clone + 'static,
    Phantom: 'static,
>(
    component: Component,
) where
    Props: Default,
{
    LaunchBuilder::new(component).launch()
}
