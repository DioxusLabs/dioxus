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
    Platform: PlatformBuilder<Props> = CurrentPlatform,
> {
    cross_platform_config: CrossPlatformConfig<Component, Props, Phantom>,
    platform_config: Option<<Platform as PlatformBuilder<Props>>::Config>,
}

// Default platform builder
impl<
        Component: ComponentFunction<Phantom, Props = Props>,
        Props: Clone + 'static,
        Phantom: 'static,
    > LaunchBuilder<Component, Props, Phantom>
{
    /// Create a new builder for your application. This will create a launch configuration for the current platform based on the features enabled on the `dioxus` crate.
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
}

impl<
        Component: ComponentFunction<Phantom, Props = Props>,
        Props: Clone + 'static,
        Phantom: 'static,
        Platform: PlatformBuilder<Props>,
    > LaunchBuilder<Component, Props, Phantom, Platform>
{
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
        config: impl Into<Option<<Platform as PlatformBuilder<Props>>::Config>>,
    ) -> Self {
        if let Some(config) = config.into() {
            self.platform_config = Some(config);
        }
        self
    }

    #[allow(clippy::unit_arg)]
    /// Launch the app.
    pub fn launch(self) {
        Platform::launch(
            self.cross_platform_config,
            self.platform_config.unwrap_or_default(),
        );
    }
}

#[cfg(feature = "web")]
impl<
        Component: ComponentFunction<Phantom, Props = Props>,
        Props: Clone + 'static,
        Phantom: 'static,
    > LaunchBuilder<Component, Props, Phantom, dioxus_web::WebPlatform>
{
    /// Launch your web application.
    pub fn launch_web(self) {
        dioxus_web::WebPlatform::launch(
            self.cross_platform_config,
            self.platform_config.unwrap_or_default(),
        );
    }
}

#[cfg(feature = "desktop")]
impl<
        Component: ComponentFunction<Phantom, Props = Props>,
        Props: Clone + 'static,
        Phantom: 'static,
    > LaunchBuilder<Component, Props, Phantom, dioxus_desktop::DesktopPlatform>
{
    /// Launch your desktop application.
    pub fn launch_desktop(self) {
        dioxus_desktop::DesktopPlatform::launch(
            self.cross_platform_config,
            self.platform_config.unwrap_or_default(),
        );
    }
}

#[cfg(feature = "desktop")]
type CurrentPlatform = dioxus_desktop::DesktopPlatform;
#[cfg(all(feature = "web", not(feature = "desktop")))]
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

#[cfg(feature = "web")]
/// Launch your web application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_web<
    Component: ComponentFunction<Phantom, Props = Props>,
    Props: Clone + 'static,
    Phantom: 'static,
>(
    component: Component,
) where
    Props: Default,
{
    LaunchBuilder::new(component).launch_web()
}

#[cfg(feature = "desktop")]
/// Launch your desktop application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_desktop<
    Component: ComponentFunction<Phantom, Props = Props>,
    Props: Clone + 'static,
    Phantom: 'static,
>(
    component: Component,
) where
    Props: Default,
{
    LaunchBuilder::new(component).launch_desktop()
}
