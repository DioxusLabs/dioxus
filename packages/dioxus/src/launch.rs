//! Launch helper macros for fullstack apps
#![allow(clippy::new_without_default)]
#![allow(unused)]
use std::any::Any;

use crate::prelude::*;

/// A builder for a fullstack app.
#[must_use]
pub struct LaunchBuilder<Cfg: 'static = (), ContextFn: ?Sized = ValidContext> {
    launch_fn: LaunchFn<Cfg, ContextFn>,
    contexts: Vec<Box<ContextFn>>,

    platform_config: Option<Box<dyn Any>>,
}

pub type LaunchFn<Cfg, Context> = fn(fn() -> Element, Vec<Box<Context>>, Cfg);

#[cfg(any(feature = "fullstack", feature = "liveview"))]
type ValidContext = SendContext;

#[cfg(not(any(feature = "fullstack", feature = "liveview")))]
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
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    pub fn web() -> LaunchBuilder<dioxus_web::Config, UnsendContext> {
        LaunchBuilder {
            launch_fn: dioxus_web::launch::launch,
            contexts: Vec::new(),
            platform_config: None,
        }
    }

    /// Launch your desktop application.
    #[cfg(feature = "desktop")]
    #[cfg_attr(docsrs, doc(cfg(feature = "desktop")))]
    pub fn desktop() -> LaunchBuilder<dioxus_desktop::Config, UnsendContext> {
        LaunchBuilder {
            launch_fn: dioxus_desktop::launch::launch,
            contexts: Vec::new(),
            platform_config: None,
        }
    }

    /// Launch your fullstack application.
    #[cfg(feature = "fullstack")]
    #[cfg_attr(docsrs, doc(cfg(feature = "fullstack")))]
    pub fn fullstack() -> LaunchBuilder<dioxus_fullstack::Config, SendContext> {
        LaunchBuilder {
            launch_fn: dioxus_fullstack::launch::launch,
            contexts: Vec::new(),
            platform_config: None,
        }
    }

    /// Launch your fullstack application.
    #[cfg(feature = "mobile")]
    #[cfg_attr(docsrs, doc(cfg(feature = "mobile")))]
    pub fn mobile() -> LaunchBuilder<dioxus_mobile::Config, UnsendContext> {
        LaunchBuilder {
            launch_fn: dioxus_mobile::launch::launch,
            contexts: Vec::new(),
            platform_config: None,
        }
    }

    #[cfg(feature = "tui")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tui")))]
    /// Launch your tui application
    pub fn tui() -> LaunchBuilder<dioxus_tui::Config, UnsendContext> {
        LaunchBuilder {
            launch_fn: dioxus_tui::launch::launch,
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
impl<Cfg> LaunchBuilder<Cfg, UnsendContext> {
    /// Inject state into the root component's context that is created on the thread that the app is launched on.
    pub fn with_context_provider(mut self, state: impl Fn() -> Box<dyn Any> + 'static) -> Self {
        self.contexts.push(Box::new(state) as Box<UnsendContext>);
        self
    }

    /// Inject state into the root component's context.
    pub fn with_context(mut self, state: impl Any + Clone + 'static) -> Self {
        self.contexts
            .push(Box::new(move || Box::new(state.clone())));
        self
    }
}

impl<Cfg> LaunchBuilder<Cfg, SendContext> {
    /// Inject state into the root component's context that is created on the thread that the app is launched on.
    pub fn with_context_provider(
        mut self,
        state: impl Fn() -> Box<dyn Any> + Send + Sync + 'static,
    ) -> Self {
        self.contexts.push(Box::new(state) as Box<SendContext>);
        self
    }

    /// Inject state into the root component's context.
    pub fn with_context(mut self, state: impl Any + Clone + Send + Sync + 'static) -> Self {
        self.contexts
            .push(Box::new(move || Box::new(state.clone())));
        self
    }
}

impl<Cfg: Default + 'static, ContextFn: ?Sized> LaunchBuilder<Cfg, ContextFn> {
    /// Provide a platform-specific config to the builder.
    pub fn with_cfg<CG: 'static>(mut self, config: impl Into<Option<CG>>) -> Self {
        if let Some(config) = config.into() {
            self.platform_config = Some(Box::new(config));
        }
        self
    }

    /// Launch your application.
    pub fn launch(self, app: fn() -> Element) {
        let cfg: Box<Cfg> = self
            .platform_config
            .and_then(|c| c.downcast().ok())
            .unwrap_or_default();

        (self.launch_fn)(app, self.contexts, *cfg)
    }
}

mod current_platform {
    #[cfg(all(feature = "desktop", not(feature = "fullstack")))]
    pub use dioxus_desktop::launch::*;

    #[cfg(all(feature = "mobile", not(feature = "fullstack")))]
    pub use dioxus_desktop::launch::*;

    #[cfg(feature = "fullstack")]
    pub use dioxus_fullstack::launch::*;

    #[cfg(all(
        feature = "web",
        not(any(feature = "desktop", feature = "mobile", feature = "fullstack"))
    ))]
    pub use dioxus_web::launch::*;

    #[cfg(all(
        feature = "liveview",
        not(any(
            feature = "web",
            feature = "desktop",
            feature = "mobile",
            feature = "fullstack"
        ))
    ))]
    pub use dioxus_liveview::launch::*;

    #[cfg(all(
        feature = "tui",
        not(any(
            feature = "liveview",
            feature = "web",
            feature = "desktop",
            feature = "mobile",
            feature = "fullstack"
        ))
    ))]
    pub use dioxus_tui::launch::*;

    #[cfg(not(any(
        feature = "liveview",
        feature = "desktop",
        feature = "mobile",
        feature = "web",
        feature = "tui",
        feature = "fullstack"
    )))]
    pub type Config = ();

    #[cfg(not(any(
        feature = "liveview",
        feature = "desktop",
        feature = "mobile",
        feature = "web",
        feature = "tui",
        feature = "fullstack"
    )))]
    pub fn launch(
        root: fn() -> dioxus_core::Element,
        contexts: Vec<Box<super::ValidContext>>,
        platform_config: (),
    ) {
        #[cfg(feature = "third-party-renderer")]
        panic!("No first party renderer feature enabled. It looks like you are trying to use a third party renderer. You will need to use the launch function from the third party renderer crate.");

        panic!("No platform feature enabled. Please enable one of the following features: liveview, desktop, mobile, web, tui, fullstack to use the launch API.");
    }
}

/// Launch your application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch(app: fn() -> Element) {
    LaunchBuilder::new().launch(app)
}

#[cfg(feature = "web")]
#[cfg_attr(docsrs, doc(cfg(feature = "web")))]
/// Launch your web application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_web(app: fn() -> Element) {
    LaunchBuilder::web().launch(app)
}

#[cfg(feature = "desktop")]
#[cfg_attr(docsrs, doc(cfg(feature = "desktop")))]
/// Launch your desktop application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_desktop(app: fn() -> Element) {
    LaunchBuilder::desktop().launch(app)
}

#[cfg(feature = "fullstack")]
#[cfg_attr(docsrs, doc(cfg(feature = "fullstack")))]
/// Launch your fullstack application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_fullstack(app: fn() -> Element) {
    LaunchBuilder::fullstack().launch(app)
}

#[cfg(feature = "tui")]
#[cfg_attr(docsrs, doc(cfg(feature = "tui")))]
/// Launch your tui application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_tui(app: fn() -> Element) {
    LaunchBuilder::tui().launch(app)
}
