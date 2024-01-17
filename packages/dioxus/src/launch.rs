//! Launch helper macros for fullstack apps
#![allow(unused)]
use std::any::Any;

use crate::prelude::*;
use dioxus_core::VProps;
use dioxus_core::{prelude::*, AnyProps};
use dioxus_core::{BoxedContext, CrossPlatformConfig, PlatformBuilder};

/// A builder for a fullstack app.
pub struct LaunchBuilder<P: AnyProps, Platform: PlatformBuilder<P> = CurrentPlatform> {
    cross_platform_config: CrossPlatformConfig<P>,
    platform_config: Option<<Platform as PlatformBuilder<P>>::Config>,
}

#[cfg(feature = "fullstack")]
// Fullstack platform builder
impl<
        F: ComponentFunction<Props, M> + Send + Sync,
        Props: Clone + Send + Sync + 'static,
        M: Send + Sync + 'static,
    > LaunchBuilder<VProps<F, Props, M>>
{
    /// Create a new builder for your application. This will create a launch configuration for the current platform based on the features enabled on the `dioxus` crate.
    pub fn new(component: F) -> Self
    where
        Props: Default,
    {
        Self {
            cross_platform_config: CrossPlatformConfig::new(VProps::new(
                component,
                |_, _| true,
                Default::default(),
                "root",
            )),
            platform_config: None,
        }
    }

    /// Create a new builder for your application with some root props. This will create a launch configuration for the current platform based on the features enabled on the `dioxus` crate.
    pub fn new_with_props(component: F, props: Props) -> Self {
        Self {
            cross_platform_config: CrossPlatformConfig::new(VProps::new(
                component,
                |_, _| true,
                props,
                "root",
            )),
            platform_config: None,
        }
    }
}

#[cfg(not(feature = "fullstack"))]
// Default platform builder
impl<F: ComponentFunction<Props, M>, Props: Clone + 'static, M: 'static>
    LaunchBuilder<VProps<F, Props, M>>
{
    /// Create a new builder for your application. This will create a launch configuration for the current platform based on the features enabled on the `dioxus` crate.
    pub fn new(component: F) -> Self
    where
        Props: Default,
    {
        Self {
            cross_platform_config: CrossPlatformConfig::new(VProps::new(
                component,
                |_, _| true,
                Default::default(),
                "root",
            )),
            platform_config: None,
        }
    }

    /// Create a new builder for your application with some root props. This will create a launch configuration for the current platform based on the features enabled on the `dioxus` crate.
    pub fn new_with_props(component: F, props: Props) -> Self {
        Self {
            cross_platform_config: CrossPlatformConfig::new(VProps::new(
                component,
                |_, _| true,
                props,
                "root",
            )),
            platform_config: None,
        }
    }
}

impl<P: AnyProps, Platform: PlatformBuilder<P>> LaunchBuilder<P, Platform> {
    // /// Inject state into the root component's context.
    // pub fn context(mut self, state: impl Any + Clone + 'static) -> Self {
    //     self.cross_platform_config
    //         .push_context(BoxedContext::new(state));
    //     self
    // }

    /// Provide a platform-specific config to the builder.
    pub fn cfg(
        mut self,
        config: impl Into<Option<<Platform as PlatformBuilder<P>>::Config>>,
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
impl<P: AnyProps> LaunchBuilder<P, dioxus_web::WebPlatform> {
    /// Launch your web application.
    pub fn launch_web(self) {
        dioxus_web::WebPlatform::launch(
            self.cross_platform_config,
            self.platform_config.unwrap_or_default(),
        );
    }
}

#[cfg(feature = "desktop")]
impl<P: AnyProps> LaunchBuilder<P, dioxus_desktop::DesktopPlatform> {
    /// Launch your desktop application.
    pub fn launch_desktop(self) {
        dioxus_desktop::DesktopPlatform::launch(
            self.cross_platform_config,
            self.platform_config.unwrap_or_default(),
        );
    }
}

#[cfg(feature = "fullstack")]
impl<P: AnyProps + Clone + Send + Sync> LaunchBuilder<P, dioxus_fullstack::FullstackPlatform> {
    /// Launch your fullstack application.
    pub fn launch_fullstack(self) {
        dioxus_fullstack::FullstackPlatform::launch(
            self.cross_platform_config,
            self.platform_config.unwrap_or_default(),
        );
    }
}

#[cfg(feature = "fullstack")]
type CurrentPlatform = dioxus_fullstack::FullstackPlatform;
#[cfg(all(feature = "desktop", not(feature = "fullstack")))]
type CurrentPlatform = dioxus_desktop::DesktopPlatform;
#[cfg(all(feature = "web", not(any(feature = "desktop", feature = "fullstack"))))]
type CurrentPlatform = dioxus_web::WebPlatform;
#[cfg(not(any(feature = "desktop", feature = "web", feature = "fullstack")))]
type CurrentPlatform = ();

#[cfg(feature = "fullstack")]
/// Launch your application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch<Props, Marker>(component: impl ComponentFunction<Props, Marker> + Send + Sync)
where
    Props: Default + Send + Sync + Clone + 'static,
    Marker: Send + Sync + 'static,
{
    LaunchBuilder::new(component).launch()
}

#[cfg(not(feature = "fullstack"))]
/// Launch your application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch<Props, Marker: 'static>(component: impl ComponentFunction<Props, Marker>)
where
    Props: Default + Clone + 'static,
{
    LaunchBuilder::new(component).launch()
}

#[cfg(all(feature = "web", not(feature = "fullstack")))]
/// Launch your web application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_web<Props, Marker: 'static>(component: impl ComponentFunction<Props, Marker>)
where
    Props: Default + Clone + 'static,
{
    LaunchBuilder::new(component).launch_web()
}

#[cfg(all(feature = "desktop", not(feature = "fullstack")))]
/// Launch your desktop application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_desktop<Props, Marker: 'static>(component: impl ComponentFunction<Props, Marker>)
where
    Props: Default + Clone + 'static,
{
    LaunchBuilder::new(component).launch_desktop()
}

#[cfg(feature = "fullstack")]
/// Launch your fullstack application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_fullstack<Props, Marker>(
    component: impl ComponentFunction<Props, Marker> + Send + Sync,
) where
    Props: Default + Send + Sync + Clone + 'static,
    Marker: Send + Sync + 'static,
{
    LaunchBuilder::new(component).launch_fullstack()
}
