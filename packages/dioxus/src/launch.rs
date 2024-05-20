//! Launch helper macros for fullstack apps
#![allow(clippy::new_without_default)]
#![allow(unused)]
use dioxus_config_macro::*;
use std::any::Any;

use crate::prelude::*;

/// A builder for a fullstack app.
#[must_use]
pub struct LaunchBuilder<Cfg: 'static = (), ContextFn: ?Sized = ValidContext> {
    launch_fn: LaunchFn<Cfg, ContextFn>,
    contexts: Vec<Box<ContextFn>>,

    platform_config: Option<Cfg>,
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

/// A trait for converting a type into a platform-specific config:
/// - A unit value will be converted into `None`
/// - Any config will be converted into `Some(config)`
/// - If the config is for another platform, it will be converted into `None`
pub trait TryIntoConfig<Config = Self> {
    fn into_config(self) -> Option<Config>;
}

// A config can always be converted into itself
impl<Cfg> TryIntoConfig<Cfg> for Cfg {
    fn into_config(self) -> Option<Cfg> {
        Some(self)
    }
}

// The unit type can be converted into the current platform config.
// This makes it possible to use the `desktop!`, `web!`, etc macros with the launch API.
#[cfg(any(
    feature = "liveview",
    feature = "desktop",
    feature = "mobile",
    feature = "web",
    feature = "fullstack"
))]
impl TryIntoConfig<current_platform::Config> for () {
    fn into_config(self) -> Option<current_platform::Config> {
        None
    }
}

impl<Cfg: Default + 'static, ContextFn: ?Sized> LaunchBuilder<Cfg, ContextFn> {
    /// Provide a platform-specific config to the builder.
    pub fn with_cfg(mut self, config: impl TryIntoConfig<Cfg>) -> Self {
        self.platform_config = self.platform_config.or(config.into_config());
        self
    }

    /// Launch your application.
    pub fn launch(self, app: fn() -> Element) {
        let cfg = self.platform_config.unwrap_or_default();

        (self.launch_fn)(app, self.contexts, cfg)
    }
}

/// Re-export the platform we expect the user wants
///
/// If multiple platforms are enabled, we use this priority (from highest to lowest):
/// - `fullstack`
/// - `desktop`
/// - `mobile`
/// - `web`
/// - `liveview`
mod current_platform {
    macro_rules! if_else_cfg {
        (if $attr:meta { $($then:item)* } else { $($else:item)* }) => {
            $(
                #[cfg($attr)]
                $then
            )*
            $(
                #[cfg(not($attr))]
                $else
            )*
        };
    }
    use crate::prelude::TryIntoConfig;

    #[cfg(any(feature = "desktop", feature = "mobile"))]
    if_else_cfg! {
        if not(feature = "fullstack") {
            #[cfg(feature = "desktop")]
            pub use dioxus_desktop::launch::*;
            #[cfg(not(feature = "desktop"))]
            pub use dioxus_mobile::launch::*;
        } else {
            impl TryIntoConfig<crate::launch::current_platform::Config> for ::dioxus_desktop::Config {
                fn into_config(self) -> Option<crate::launch::current_platform::Config> {
                    None
                }
            }
        }
    }

    #[cfg(feature = "fullstack")]
    pub use dioxus_fullstack::launch::*;

    #[cfg(feature = "web")]
    if_else_cfg! {
        if not(any(feature = "desktop", feature = "mobile", feature = "fullstack")) {
            pub use dioxus_web::launch::*;
        } else {
            impl TryIntoConfig<crate::launch::current_platform::Config> for ::dioxus_web::Config {
                fn into_config(self) -> Option<crate::launch::current_platform::Config> {
                    None
                }
            }
        }
    }

    #[cfg(feature = "liveview")]
    if_else_cfg! {
        if
            not(any(
                feature = "web",
                feature = "desktop",
                feature = "mobile",
                feature = "fullstack"
            ))
        {
            pub use dioxus_liveview::launch::*;
        } else {
            impl<R: ::dioxus_liveview::LiveviewRouter> TryIntoConfig<crate::launch::current_platform::Config> for ::dioxus_liveview::Config<R> {
                fn into_config(self) -> Option<crate::launch::current_platform::Config> {
                    None
                }
            }
        }
    }

    #[cfg(not(any(
        feature = "liveview",
        feature = "desktop",
        feature = "mobile",
        feature = "web",
        feature = "fullstack"
    )))]
    pub type Config = ();

    #[cfg(not(any(
        feature = "liveview",
        feature = "desktop",
        feature = "mobile",
        feature = "web",
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

#[cfg(feature = "mobile")]
#[cfg_attr(docsrs, doc(cfg(feature = "mobile")))]
/// Launch your mobile application without any additional configuration. See [`LaunchBuilder`] for more options.
pub fn launch_mobile(app: fn() -> Element) {
    LaunchBuilder::mobile().launch(app)
}
