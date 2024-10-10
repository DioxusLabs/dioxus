//! Launch helper macros for fullstack apps
#![allow(unused)]
use crate::prelude::*;
use dioxus_lib::prelude::*;
use std::sync::Arc;

/// Settings for a fullstack app.
///
/// Depending on what features are enabled, you can pass in configurations for each client platform as well as the server:
/// ```rust, no_run
/// # fn app() -> Element { todo!() }
/// use dioxus::prelude::*;
///
/// let mut cfg = dioxus::fullstack::Config::new();
///
/// // Only set the server config if the server feature is enabled
/// server_only! {
///     cfg = cfg.with_server_config(ServeConfigBuilder::default());
/// }
///
/// // Only set the web config if the web feature is enabled
/// web! {
///     cfg = cfg.with_web_config(dioxus::web::Config::default());
/// }
///
/// // Only set the desktop config if the desktop feature is enabled
/// desktop! {
///     cfg = cfg.with_desktop_config(dioxus::desktop::Config::default());
/// }
///
/// // Finally, launch the app with the config
/// LaunchBuilder::new()
///     .with_cfg(cfg)
///     .launch(app);
/// ```
pub struct Config {
    #[cfg(feature = "server")]
    pub(crate) server_cfg: ServeConfigBuilder,

    #[cfg(feature = "web")]
    pub(crate) web_cfg: dioxus_web::Config,

    #[cfg(feature = "desktop")]
    pub(crate) desktop_cfg: dioxus_desktop::Config,

    #[cfg(feature = "mobile")]
    pub(crate) mobile_cfg: dioxus_mobile::Config,
}

#[allow(clippy::derivable_impls)]
impl Default for Config {
    fn default() -> Self {
        Self {
            #[cfg(feature = "server")]
            server_cfg: ServeConfigBuilder::new(),
            #[cfg(feature = "web")]
            web_cfg: dioxus_web::Config::default(),
            #[cfg(feature = "desktop")]
            desktop_cfg: dioxus_desktop::Config::default(),
            #[cfg(feature = "mobile")]
            mobile_cfg: dioxus_mobile::Config::default(),
        }
    }
}

impl Config {
    /// Create a new config for a fullstack app.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the incremental renderer config. The incremental config can be used to improve
    /// performance of heavy routes by caching the rendered html in memory and/or the file system.
    ///
    /// ```rust, no_run
    /// # fn app() -> Element { todo!() }
    /// use dioxus::prelude::*;
    ///
    /// let mut cfg = dioxus::fullstack::Config::new();
    ///
    /// // Only set the server config if the server feature is enabled
    /// server_only! {
    ///     cfg = cfg.incremental(IncrementalRendererConfig::default().with_memory_cache_limit(10000));
    /// }
    ///
    /// // Finally, launch the app with the config
    /// LaunchBuilder::new()
    ///     .with_cfg(cfg)
    ///     .launch(app);
    /// ```
    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub fn incremental(self, cfg: IncrementalRendererConfig) -> Self {
        Self {
            server_cfg: self.server_cfg.incremental(cfg),
            ..self
        }
    }

    /// Set the server config
    /// ```rust, no_run
    /// # fn app() -> Element { todo!() }
    /// use dioxus::prelude::*;
    ///
    /// let mut cfg = dioxus::fullstack::Config::new();
    ///
    /// // Only set the server config if the server feature is enabled
    /// server_only! {
    ///     cfg = cfg.with_server_config(ServeConfigBuilder::default());
    /// }
    ///
    /// // Finally, launch the app with the config
    /// LaunchBuilder::new()
    ///     .with_cfg(cfg)
    ///     .launch(app);
    /// ```
    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub fn with_server_config(self, server_cfg: ServeConfigBuilder) -> Self {
        Self { server_cfg, ..self }
    }

    /// Set the server config by modifying the config in place
    ///
    /// ```rust, no_run
    /// # fn app() -> Element { todo!() }
    /// use dioxus::prelude::*;
    ///
    /// let mut cfg = dioxus::fullstack::Config::new();
    ///
    /// // Only set the server config if the server feature is enabled
    /// server_only! {
    ///     cfg.set_server_config(ServeConfigBuilder::default());
    /// }
    ///
    /// // Finally, launch the app with the config
    /// LaunchBuilder::new()
    ///     .with_cfg(cfg)
    ///     .launch(app);
    /// ```
    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub fn set_server_config(&mut self, server_cfg: ServeConfigBuilder) {
        self.server_cfg = server_cfg;
    }

    /// Set the web config
    /// ```rust, no_run
    /// # fn app() -> Element { todo!() }
    /// use dioxus::prelude::*;
    ///
    /// let mut cfg = dioxus::fullstack::Config::new();
    ///
    /// // Only set the web config if the server feature is enabled
    /// web! {
    ///     cfg = cfg.with_web_config(dioxus::web::Config::default());
    /// }
    ///
    /// // Finally, launch the app with the config
    /// LaunchBuilder::new()
    ///     .with_cfg(cfg)
    ///     .launch(app);
    /// ```
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    pub fn with_web_config(self, web_cfg: dioxus_web::Config) -> Self {
        Self { web_cfg, ..self }
    }

    /// Set the server config by modifying the config in place
    ///
    /// ```rust, no_run
    /// # fn app() -> Element { todo!() }
    /// use dioxus::prelude::*;
    ///
    /// let mut cfg = dioxus::fullstack::Config::new();
    ///
    /// // Only set the web config if the server feature is enabled
    /// web! {
    ///     cfg.set_web_config(dioxus::web::Config::default());
    /// }
    ///
    /// // Finally, launch the app with the config
    /// LaunchBuilder::new()
    ///     .with_cfg(cfg)
    ///     .launch(app);
    /// ```
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    pub fn set_web_config(&mut self, web_cfg: dioxus_web::Config) {
        self.web_cfg = web_cfg;
    }

    /// Set the desktop config
    /// ```rust, no_run
    /// # fn app() -> Element { todo!() }
    /// use dioxus::prelude::*;
    ///
    /// let mut cfg = dioxus::fullstack::Config::new();
    ///
    /// // Only set the desktop config if the server feature is enabled
    /// desktop! {
    ///     cfg = cfg.with_desktop_config(dioxus::desktop::Config::default());
    /// }
    ///
    /// // Finally, launch the app with the config
    /// LaunchBuilder::new()
    ///     .with_cfg(cfg)
    ///     .launch(app);
    /// ```
    #[cfg(feature = "desktop")]
    #[cfg_attr(docsrs, doc(cfg(feature = "desktop")))]
    pub fn with_desktop_config(self, desktop_cfg: dioxus_desktop::Config) -> Self {
        Self {
            desktop_cfg,
            ..self
        }
    }

    /// Set the desktop config by modifying the config in place
    ///
    /// ```rust, no_run
    /// # fn app() -> Element { todo!() }
    /// use dioxus::prelude::*;
    ///
    /// let mut cfg = dioxus::fullstack::Config::new();
    ///
    /// // Only set the desktop config if the server feature is enabled
    /// desktop! {
    ///     cfg.set_desktop_config(dioxus::desktop::Config::default());
    /// }
    ///
    /// // Finally, launch the app with the config
    /// LaunchBuilder::new()
    ///     .with_cfg(cfg)
    ///     .launch(app);
    /// ```
    #[cfg(feature = "desktop")]
    #[cfg_attr(docsrs, doc(cfg(feature = "desktop")))]
    pub fn set_desktop_config(&mut self, desktop_cfg: dioxus_desktop::Config) {
        self.desktop_cfg = desktop_cfg;
    }

    /// Set the mobile config
    /// ```rust, no_run
    /// # fn app() -> Element { todo!() }
    /// use dioxus::prelude::*;
    ///
    /// let mut cfg = dioxus::fullstack::Config::new();
    ///
    /// // Only set the mobile config if the server feature is enabled
    /// mobile! {
    ///     cfg = cfg.with_mobile_cfg(dioxus::mobile::Config::default());
    /// }
    ///
    /// // Finally, launch the app with the config
    /// LaunchBuilder::new()
    ///     .with_cfg(cfg)
    ///     .launch(app);
    /// Set the mobile config.
    #[cfg(feature = "mobile")]
    #[cfg_attr(docsrs, doc(cfg(feature = "mobile")))]
    pub fn with_mobile_cfg(self, mobile_cfg: dioxus_mobile::Config) -> Self {
        Self { mobile_cfg, ..self }
    }

    /// Set the mobile config by modifying the config in place
    /// ```rust, no_run
    /// # fn app() -> Element { todo!() }
    /// use dioxus::prelude::*;
    ///
    /// let mut cfg = dioxus::fullstack::Config::new();
    ///
    /// // Only set the mobile config if the server feature is enabled
    /// mobile! {
    ///     cfg.set_mobile_cfg(dioxus::mobile::Config::default());
    /// }
    ///
    /// // Finally, launch the app with the config
    /// LaunchBuilder::new()
    ///     .with_cfg(cfg)
    ///     .launch(app);
    /// Set the mobile config.
    #[cfg(feature = "mobile")]
    #[cfg_attr(docsrs, doc(cfg(feature = "mobile")))]
    pub fn set_mobile_cfg(&mut self, mobile_cfg: dioxus_mobile::Config) {
        self.mobile_cfg = mobile_cfg;
    }
}
