//! Launch helper macros for fullstack apps
#![allow(unused)]
use crate::prelude::*;
use dioxus_lib::prelude::*;
use std::sync::Arc;

/// Settings for a fullstack app.
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

    /// Set the incremental renderer config.
    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub fn incremental(self, cfg: IncrementalRendererConfig) -> Self {
        Self {
            server_cfg: self.server_cfg.incremental(cfg),
            ..self
        }
    }

    /// Set the server config.
    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub fn with_server_cfg(self, server_cfg: ServeConfigBuilder) -> Self {
        Self { server_cfg, ..self }
    }

    /// Set the server config.
    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    pub fn set_server_cfg(&mut self, server_cfg: ServeConfigBuilder) {
        self.server_cfg = server_cfg;
    }

    /// Set the web config.
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    pub fn with_web_config(self, web_cfg: dioxus_web::Config) -> Self {
        Self { web_cfg, ..self }
    }

    /// Set the web config.
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    pub fn set_web_config(&mut self, web_cfg: dioxus_web::Config) {
        self.web_cfg = web_cfg;
    }

    /// Set the desktop config
    #[cfg(feature = "desktop")]
    #[cfg_attr(docsrs, doc(cfg(feature = "desktop")))]
    pub fn with_desktop_config(self, desktop_cfg: dioxus_desktop::Config) -> Self {
        Self {
            desktop_cfg,
            ..self
        }
    }

    /// Set the desktop config.
    #[cfg(feature = "desktop")]
    #[cfg_attr(docsrs, doc(cfg(feature = "desktop")))]
    pub fn set_desktop_config(&mut self, desktop_cfg: dioxus_desktop::Config) {
        self.desktop_cfg = desktop_cfg;
    }

    /// Set the mobile config.
    #[cfg(feature = "mobile")]
    #[cfg_attr(docsrs, doc(cfg(feature = "mobile")))]
    pub fn with_mobile_cfg(self, mobile_cfg: dioxus_mobile::Config) -> Self {
        Self { mobile_cfg, ..self }
    }
}
