//! Launch helper macros for fullstack apps

use std::path::PathBuf;

/// Settings for a statically generated site that may be hydrated in the browser
pub struct Config {
    #[cfg(feature = "server")]
    pub(crate) output_dir: PathBuf,

    #[cfg(feature = "server")]
    pub(crate) index_html: Option<String>,

    #[cfg(feature = "server")]
    pub(crate) index_path: Option<PathBuf>,

    #[cfg(feature = "server")]
    #[allow(clippy::type_complexity)]
    pub(crate) map_path: Option<Box<dyn Fn(&str) -> PathBuf + Send + Sync + 'static>>,

    #[cfg(feature = "server")]
    pub(crate) root_id: Option<&'static str>,

    #[cfg(feature = "server")]
    pub(crate) additional_routes: Vec<String>,

    #[cfg(feature = "server")]
    pub(crate) github_pages: bool,

    #[cfg(feature = "web")]
    #[allow(unused)]
    pub(crate) web_cfg: dioxus_web::Config,
}

#[allow(clippy::derivable_impls)]
impl Default for Config {
    fn default() -> Self {
        Self {
            #[cfg(feature = "server")]
            output_dir: PathBuf::from("./static"),
            #[cfg(feature = "server")]
            index_html: None,
            #[cfg(feature = "server")]
            index_path: None,
            #[cfg(feature = "server")]
            map_path: None,
            #[cfg(feature = "server")]
            root_id: None,
            #[cfg(feature = "server")]
            additional_routes: vec!["/".to_string()],
            #[cfg(feature = "server")]
            github_pages: false,
            #[cfg(feature = "web")]
            web_cfg: dioxus_web::Config::default(),
        }
    }
}

// Note: This config intentionally leaves server options in even if the server feature is not enabled to make it easier to use the config without moving different parts of the builder under configs.
impl Config {
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
        {
            self.map_path = Some(Box::new(map_path));
        }
        self
    }

    /// Set the output directory for the static site generation. (defaults to ./static)
    /// This is the directory that will be used to store the generated static html files.
    ///
    /// This method will only effect static site generation.
    #[allow(unused)]
    pub fn output_dir(mut self, output_dir: PathBuf) -> Self {
        #[cfg(feature = "server")]
        {
            self.output_dir = output_dir;
        }
        self
    }

    /// Set the id of the root element in the index.html file to place the prerendered content into. (defaults to main)
    #[allow(unused)]
    pub fn root_id(mut self, root_id: &'static str) -> Self {
        #[cfg(feature = "server")]
        {
            self.root_id = Some(root_id);
        }
        self
    }

    /// Set the contents of the index.html file to be served. (precedence over index_path)
    ///
    /// This method will only effect static site generation.
    #[allow(unused)]
    pub fn index_html(mut self, index_html: String) -> Self {
        #[cfg(feature = "server")]
        {
            self.index_html = Some(index_html);
        }
        self
    }

    /// Set the path of the index.html file to be served. (defaults to {assets_path}/index.html)
    ///
    /// This method will only effect static site generation.
    #[allow(unused)]
    pub fn index_path(mut self, index_path: PathBuf) -> Self {
        #[cfg(feature = "server")]
        {
            self.index_path = Some(index_path);
        }
        self
    }

    /// Sets a list of static routes that will be pre-rendered and served in addition to the static routes in the router.
    #[allow(unused)]
    pub fn additional_routes(mut self, mut routes: Vec<String>) -> Self {
        #[cfg(feature = "server")]
        {
            self.additional_routes.append(&mut routes);
        }
        self
    }

    /// A preset for github pages. This will output your files in the `/docs` directory and set up a `404.html` file.
    pub fn github_pages(self) -> Self {
        #[allow(unused_mut)]
        let mut myself = self
            .additional_routes(vec!["/404".into()])
            .output_dir(PathBuf::from("./docs"));
        #[cfg(feature = "server")]
        {
            myself.github_pages = true;
        }
        myself
    }

    /// Set the web config.
    ///
    /// This method will only effect the hydrated web renderer.
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    pub fn web_cfg(self, web_cfg: dioxus_web::Config) -> Self {
        #[allow(clippy::needless_update)]
        Self { web_cfg, ..self }
    }
}

#[cfg(feature = "server")]
impl Config {
    pub(crate) fn fullstack_template(&self) -> dioxus_fullstack::prelude::FullstackHTMLTemplate {
        use dioxus_fullstack::prelude::{FullstackHTMLTemplate, ServeConfig};
        let mut cfg_builder = ServeConfig::builder();
        if let Some(index_html) = &self.index_html {
            cfg_builder = cfg_builder.index_html(index_html.clone());
        }
        if let Some(index_path) = &self.index_path {
            cfg_builder = cfg_builder.index_path(index_path.clone());
        }
        if let Some(root_id) = self.root_id {
            cfg_builder = cfg_builder.root_id(root_id);
        }
        let cfg = cfg_builder.build();

        FullstackHTMLTemplate::new(&cfg)
    }

    pub(crate) fn create_cache(&mut self) -> dioxus_ssr::incremental::IncrementalRenderer {
        let mut builder = dioxus_ssr::incremental::IncrementalRenderer::builder()
            .static_dir(self.output_dir.clone());
        if let Some(map_path) = self.map_path.take() {
            builder = builder.map_path(map_path);
        }
        builder.build()
    }

    pub(crate) fn create_renderer(&mut self) -> dioxus_ssr::Renderer {
        let mut renderer = dioxus_ssr::Renderer::new();
        renderer.pre_render = true;
        renderer
    }
}
