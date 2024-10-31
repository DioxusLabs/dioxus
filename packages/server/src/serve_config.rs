//! Configuration for how to serve a Dioxus application

use crate::{Error, Result};
use dioxus_lib::prelude::dioxus_core::LaunchConfig;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use crate::IncrementalRendererConfig;

/// A ServeConfig is used to configure how to serve a Dioxus application. It contains information about how to serve static assets, and what content to render with [`dioxus-ssr`].
#[derive(Clone, Default)]
pub struct ServeConfig {
    pub(crate) root_id: Option<&'static str>,
    pub(crate) index_html: Option<String>,
    pub(crate) index_path: Option<PathBuf>,
    pub(crate) incremental: Option<IncrementalRendererConfig>,
}

impl ServeConfig {
    /// Create a new ServeConfigBuilder with incremental static generation disabled and the default index.html settings
    pub fn new() -> Self {
        Self {
            root_id: None,
            index_html: None,
            index_path: None,
            incremental: None,
        }
    }

    /// Enable incremental static generation. Incremental static generation caches the
    /// rendered html in memory and/or the file system. It can be used to improve performance of heavy routes.
    ///
    /// ```rust, no_run
    /// # fn app() -> Element { todo!() }
    /// use dioxus::prelude::*;
    ///
    /// let mut cfg = dioxus::fullstack::Config::new();
    ///
    /// // Only set the server config if the server feature is enabled
    /// server_only! {
    ///     cfg = cfg.with_server_cfg(ServeConfigBuilder::default().incremental(IncrementalRendererConfig::default()));
    /// }
    ///
    /// // Finally, launch the app with the config
    /// LaunchBuilder::new()
    ///     .with_cfg(cfg)
    ///     .launch(app);
    /// ```
    pub fn incremental(mut self, cfg: IncrementalRendererConfig) -> Self {
        self.incremental = Some(cfg);
        self
    }

    /// Set the contents of the index.html file to be served. (precedence over index_path)
    pub fn index_html(mut self, index_html: String) -> Self {
        self.index_html = Some(index_html);
        self
    }

    /// Set the path of the index.html file to be served. (defaults to {assets_path}/index.html)
    pub fn index_path(mut self, index_path: PathBuf) -> Self {
        self.index_path = Some(index_path);
        self
    }

    /// Set the id of the root element in the index.html file to place the prerendered content into. (defaults to main)
    ///
    /// # Example
    ///
    /// If your index.html file looks like this:
    /// ```html
    /// <!DOCTYPE html>
    /// <html>
    ///     <head>
    ///         <title>My App</title>
    ///     </head>
    ///     <body>
    ///         <div id="my-custom-root"></div>
    ///     </body>
    /// </html>
    /// ```
    ///
    /// You can set the root id to `"my-custom-root"` to render the app into that element:
    ///
    /// ```rust, no_run
    /// # fn app() -> Element { todo!() }
    /// use dioxus::prelude::*;
    ///
    /// let mut cfg = dioxus::fullstack::Config::new();
    ///
    /// // Only set the server config if the server feature is enabled
    /// server_only! {
    ///     cfg = cfg.with_server_cfg(ServeConfigBuilder::default().root_id("app"));
    /// }
    ///
    /// // You also need to set the root id in your web config
    /// web! {
    ///     cfg = cfg.with_web_config(dioxus::web::Config::default().rootname("app"));
    /// }
    ///
    /// // And desktop config
    /// desktop! {
    ///     cfg = cfg.with_desktop_config(dioxus::desktop::Config::default().with_root_name("app"));
    /// }
    ///
    /// // Finally, launch the app with the config
    /// LaunchBuilder::new()
    ///     .with_cfg(cfg)
    ///     .launch(app);
    /// ```
    pub fn root_id(mut self, root_id: &'static str) -> Self {
        self.root_id = Some(root_id);
        self
    }
}
