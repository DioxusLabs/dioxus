//! Launch helper macros for fullstack apps
#![allow(unused)]
use crate::prelude::*;
use dioxus_lib::prelude::*;
use std::sync::Arc;

/// Settings for a staticly generated site that may be hydrated in the browser
pub struct StaticSiteGenerationConfig {
    #[allow(feature = "server")]
    pub(crate) output_dir: PathBuf,

    #[allow(feature = "server")]
    pub(crate) index_html: Option<String>,

    #[allow(feature = "server")]
    pub(crate) index_path: Option<PathBuf>,

    #[allow(feature = "server")]
    pub(crate) map_path: Option<Arc<dyn Fn(&str) -> PathBuf + Send + Sync + 'static>>,

    #[allow(feature = "web")]
    pub(crate) web_cfg: dioxus_web::Config,
}

#[allow(clippy::derivable_impls)]
impl Default for StaticSiteGenerationConfig {
    fn default() -> Self {
        Self {
            #[allow(feature = "server")]
            output_dir: PathBuf::from("./static"),
            #[allow(feature = "server")]
            index_html: None,
            #[allow(feature = "server")]
            index_path: None,
            #[allow(feature = "server")]
            map_path: None,
            #[allow(feature = "web")]
            web_cfg: dioxus_web::Config::default(),
        }
    }
}

// Note: This config intentionally leaves server options in even if the server feature is not enabled to make it easier to use the config without moving different parts of the builder under configs.
impl StaticSiteGenerationConfig {
    /// Create a new config for a static site generation app.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a mapping from the route to the file path. This will override the default mapping configured with `static_dir`.
    /// The function should return the path to the folder to store the index.html file in.
    ///
    /// This method will only effect static site generation.
    #[allow(unused)]
    pub fn map_path<F: Fn(&str) -> PathBuf + Send + Sync + 'static>(mut self, map_path: F) -> Self {
        #[cfg(feature = "server")]
        self.map_path = Some(Arc::new(map_path));
        self
    }

    /// Set the output directory for the static site generation. (defaults to ./static)
    /// This is the directory that will be used to store the generated static html files.
    ///
    /// This method will only effect static site generation.
    #[allow(unused)]
    pub fn output_dir(mut self, output_dir: PathBuf) -> Self {
        #[cfg(feature = "server")]
        self.output_dir = output_dir;
        self
    }

    /// Set the contents of the index.html file to be served. (precedence over index_path)
    ///
    /// This method will only effect static site generation.
    #[allow(unused)]
    pub fn index_html(mut self, index_html: String) -> Self {
        #[cfg(feature = "server")]
        self.index_html = Some(index_html);
        self
    }

    /// Set the path of the index.html file to be served. (defaults to {assets_path}/index.html)
    ///
    /// This method will only effect static site generation.
    #[allow(unused)]
    pub fn index_path(mut self, index_path: PathBuf) -> Self {
        #[cfg(feature = "server")]
        self.index_path = Some(index_path);
        self
    }

    /// Set the web config.
    ///
    /// This method will only effect the hydrated web renderer.
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    pub fn web_cfg(self, web_cfg: dioxus_web::Config) -> Self {
        Self { web_cfg, ..self }
    }
}
