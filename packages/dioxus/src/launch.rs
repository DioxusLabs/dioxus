//! Launch helper macros for fullstack apps

#![allow(unused)]
use std::any::Any;

use crate::prelude::*;
use dioxus_core::AnyProps;
use dioxus_core::VProps;

/// A builder for a fullstack app.
pub struct LaunchBuilder<Cfg = (), ContextFn: ?Sized = ValidContext> {
    launch_fn: LaunchFn<Cfg, ContextFn>,
    contexts: Vec<Box<ContextFn>>,
    platform_config: Option<Cfg>,
}

pub type LaunchFn<Cfg, Context> = fn(fn() -> Element, Vec<Box<Context>>, Cfg);

#[cfg(feature = "fullstack")]
type ValidContext = SendContext;

#[cfg(not(feature = "fullstack"))]
type ValidContext = UnsendContext;

type SendContext = dyn Fn() -> Box<dyn Any> + Send + Sync + 'static;

type UnsendContext = dyn Fn() -> Box<dyn Any> + 'static;

impl LaunchBuilder {
    /// Create a new builder for your application. This will create a launch configuration for the current platform based on the features enabled on the `dioxus` crate.
    pub fn new() -> LaunchBuilder<current_platform::Config, ValidContext> {
        LaunchBuilder {
            launch_fn: current_platform::launch,
            contexts: Vec::new(),
            platform_config: None,
        }
    }

    /// Launch your web application.
    #[cfg(feature = "web")]
    pub fn web() -> LaunchBuilder<dioxus_web::Config, UnsendContext> {
        LaunchBuilder {
            launch_fn: dioxus_web::launch::launch,
            contexts: Vec::new(),
            platform_config: None,
        }
    }

    /// Launch your desktop application.
    #[cfg(feature = "desktop")]
    pub fn desktop() -> LaunchBuilder<dioxus_desktop::Config, UnsendContext> {
        LaunchBuilder {
            launch_fn: dioxus_desktop::launch::launch,
            contexts: Vec::new(),
            platform_config: None,
        }
    }

    /// Launch your fullstack application.
    #[cfg(feature = "fullstack")]
    pub fn fullstack() -> LaunchBuilder<dioxus_fullstack::Config, SendContext> {
        LaunchBuilder {
            launch_fn: dioxus_fullstack::launch::launch,
            contexts: Vec::new(),
            platform_config: None,
        }
    }

    /// Launch your fullstack application.
    #[cfg(feature = "mobile")]
    pub fn mobile() -> LaunchBuilder<dioxus_mobile::Config> {
        LaunchBuilder {
            launch_fn: dioxus_mobile::launch::launch,
            contexts: Vec::new(),
            platform_config: None,
        }
    }

    /// Provide a custom launch function for your application.
    ///
    /// Useful for third party renderers to tap into the launch builder API without having to reimplement it.
    pub fn custom<Cfg, List>(launch_fn: LaunchFn<Cfg, List>) -> LaunchBuilder<Cfg, List> {
        LaunchBuilder {
            launch_fn,
            contexts: vec![],
            platform_config: None,
        }
    }
}

// Fullstack platform builder
impl<Cfg: Default> LaunchBuilder<Cfg, ValidContext> {
    /// Inject state into the root component's context that is created on the thread that the app is launched on.
    pub fn with_context_provider(
        mut self,

        #[cfg(feature = "fullstack")] state: impl Fn() -> Box<dyn Any> + Send + Sync + 'static,
        #[cfg(not(feature = "fullstack"))] state: impl Fn() -> Box<dyn Any> + 'static,
    ) -> Self {
        self.contexts.push(Box::new(state) as Box<ValidContext>);
        self
    }

    #[cfg(feature = "fullstack")]
    /// Inject state into the root component's context.
    pub fn with_context(
        mut self,
        #[cfg(feature = "fullstack")] state: impl Any + Clone + Send + Sync + 'static,
        #[cfg(not(feature = "fullstack"))] state: impl Any + Clone + 'static,
    ) -> Self {
        self.contexts
            .push(Box::new(move || Box::new(state.clone())));
        self
    }

    /// Provide a platform-specific config to the builder.
    pub fn with_cfg(mut self, config: impl Into<Option<Cfg>>) -> Self {
        if let Some(config) = config.into() {
            self.platform_config = Some(config);
        }
        self
    }

    #[allow(clippy::unit_arg)]
    /// Launch the app.
    pub fn launch(self, app: fn() -> Element) {
        (self.launch_fn)(app, self.contexts, self.platform_config.unwrap_or_default());
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
    pub type Config = ();
    #[cfg(not(any(feature = "desktop", feature = "web", feature = "fullstack")))]
    pub fn launch(
        root: fn() -> dioxus_core::Element,
        contexts: Vec<Box<super::ValidContext>>,
        platform_config: Config,
    ) {
    }
}

/// Launch your application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch(app: fn() -> Element) {
    LaunchBuilder::new().launch(app)
}

#[cfg(all(feature = "web", not(feature = "fullstack")))]
/// Launch your web application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_web(app: fn() -> Element) {
    LaunchBuilder::web().launch(app)
}

#[cfg(all(feature = "desktop", not(feature = "fullstack")))]
/// Launch your desktop application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_desktop(app: fn() -> Element) {
    LaunchBuilder::desktop().launch(app)
}

#[cfg(feature = "fullstack")]
/// Launch your fullstack application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_fullstack(app: fn() -> Element) {
    LaunchBuilder::fullstack().launch(app)
}
