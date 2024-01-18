//! Launch helper macros for fullstack apps
#![allow(unused)]
use std::any::Any;

use crate::prelude::*;
use dioxus_core::VProps;
use dioxus_core::{prelude::*, AnyProps};

/// A builder for a fullstack app.
pub struct LaunchBuilder {
    cross_platform_config: fn() -> Element,
    #[cfg(feature = "fullstack")]
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send>>,
    #[cfg(not(feature = "fullstack"))]
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any>>>,
    platform_config: Option<current_platform::Config>,
}

// Fullstack platform builder
impl LaunchBuilder {
    /// Create a new builder for your application. This will create a launch configuration for the current platform based on the features enabled on the `dioxus` crate.
    pub fn new(component: fn() -> Element) -> Self {
        Self {
            cross_platform_config: component,
            contexts: Vec::new(),
            platform_config: None,
        }
    }

    #[cfg(feature = "fullstack")]
    /// Inject state into the root component's context that is created on the thread that the app is launched on.
    pub fn context_provider(mut self, state: impl Fn() -> Box<dyn Any> + Send + 'static) -> Self {
        self.contexts
            .push(Box::new(state) as Box<dyn Fn() -> Box<dyn Any> + Send>);
        self
    }

    #[cfg(not(feature = "fullstack"))]
    /// Inject state into the root component's context that is created on the thread that the app is launched on.
    pub fn context_provider(mut self, state: impl Fn() -> Box<dyn Any> + 'static) -> Self {
        self.contexts
            .push(Box::new(state) as Box<dyn Fn() -> Box<dyn Any>>);
        self
    }

    #[cfg(feature = "fullstack")]
    /// Inject state into the root component's context.
    pub fn context(mut self, state: impl Any + Clone + Send + 'static) -> Self {
        self.contexts
            .push(Box::new(move || Box::new(state.clone())));
        self
    }

    #[cfg(not(feature = "fullstack"))]
    /// Inject state into the root component's context.
    pub fn context(mut self, state: impl Any + Clone + 'static) -> Self {
        self.contexts
            .push(Box::new(move || Box::new(state.clone())));
        self
    }

    /// Provide a platform-specific config to the builder.
    pub fn cfg(mut self, config: impl Into<Option<current_platform::Config>>) -> Self {
        if let Some(config) = config.into() {
            self.platform_config = Some(config);
        }
        self
    }

    #[cfg(feature = "web")]
    /// Launch your web application.
    pub fn launch_web(self) {
        dioxus_web::launch::launch(
            self.cross_platform_config,
            Default::default(),
            Default::default(),
        );
    }

    /// Launch your desktop application.
    #[cfg(feature = "desktop")]
    pub fn launch_desktop(self) {
        dioxus_desktop::launch::launch(
            self.cross_platform_config,
            Default::default(),
            Default::default(),
        );
    }

    /// Launch your fullstack application.
    #[cfg(feature = "fullstack")]
    pub fn launch_fullstack(self) {
        dioxus_fullstack::launch::launch(
            self.cross_platform_config,
            Default::default(),
            Default::default(),
        );
    }

    #[allow(clippy::unit_arg)]
    /// Launch the app.
    pub fn launch(self) {
        current_platform::launch(
            self.cross_platform_config,
            Default::default(),
            self.platform_config.unwrap_or_default(),
        );
    }
}

mod current_platform {
    #[cfg(all(feature = "desktop", not(feature = "fullstack")))]
    pub use dioxus_desktop::launch::*;
    #[cfg(feature = "fullstack")]
    pub use dioxus_fullstack::launch::*;
    #[cfg(all(feature = "web", not(any(feature = "desktop", feature = "fullstack"))))]
    pub use dioxus_web::launch::*;
    #[cfg(not(any(feature = "desktop", feature = "web", feature = "fullstack")))]
    type Config = ();
    #[cfg(not(any(feature = "desktop", feature = "web", feature = "fullstack")))]
    pub fn launch(
        root: fn() -> Element,
        contexts: Vec<Box<dyn CloneAny + Send + Sync>>,
        platform_config: Config,
    ) {
    }
}

/// Launch your application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch(component: fn() -> Element) {
    LaunchBuilder::new(component).launch()
}

#[cfg(all(feature = "web", not(feature = "fullstack")))]
/// Launch your web application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_web(component: fn() -> Element) {
    LaunchBuilder::new(component).launch_web()
}

#[cfg(all(feature = "desktop", not(feature = "fullstack")))]
/// Launch your desktop application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_desktop(component: fn() -> Element) {
    LaunchBuilder::new(component).launch_desktop()
}

#[cfg(feature = "fullstack")]
/// Launch your fullstack application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_fullstack(component: fn() -> Element) {
    LaunchBuilder::new(component).launch_fullstack()
}
