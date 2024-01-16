//! Launch helper macros for fullstack apps
#![allow(unused)]
use std::any::Any;

use crate::prelude::*;
use dioxus_core::prelude::*;
use dioxus_core::{BoxedContext, CrossPlatformConfig, PlatformBuilder};

/// A builder for a fullstack app.
pub struct LaunchBuilder<Props: Clone + 'static, Platform: PlatformBuilder<Props> = CurrentPlatform>
{
    cross_platform_config: CrossPlatformConfig<Props>,
    platform_config: Option<<Platform as PlatformBuilder<Props>>::Config>,
}

// Default platform builder
impl<Props: Clone + 'static> LaunchBuilder<Props> {
    /// Create a new builder for your application. This will create a launch configuration for the current platform based on the features enabled on the `dioxus` crate.
    pub fn new<M>(component: impl ComponentFn<Props, M>) -> Self
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

impl<Props: Clone + 'static, Platform: PlatformBuilder<Props>> LaunchBuilder<Props, Platform> {
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
impl<Props: Clone + 'static> LaunchBuilder<Props, dioxus_web::WebPlatform> {
    /// Launch your web application.
    pub fn launch_web(self) {
        dioxus_web::WebPlatform::launch(
            self.cross_platform_config,
            self.platform_config.unwrap_or_default(),
        );
    }
}

#[cfg(feature = "desktop")]
impl<Props: Clone + 'static> LaunchBuilder<Props, dioxus_desktop::DesktopPlatform> {
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
pub fn launch<Props, Marker>(component: impl ComponentFn<Props, Marker>)
where
    Props: Default + Clone + 'static,
{
    LaunchBuilder::new(component).launch()
}

#[cfg(feature = "web")]
/// Launch your web application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_web<Props, Marker>(component: impl ComponentFn<Props, Marker>)
where
    Props: Default + Clone + 'static,
{
    LaunchBuilder::new(component).launch_web()
}

#[cfg(feature = "desktop")]
/// Launch your desktop application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_desktop<Props, Marker>(component: impl ComponentFn<Props, Marker>)
where
    Props: Default + Clone + 'static,
{
    LaunchBuilder::new(component).launch_desktop()
}
