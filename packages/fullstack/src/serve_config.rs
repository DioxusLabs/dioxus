#![allow(non_snake_case)]
//! Configuration for how to serve a Dioxus application

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

/// A ServeConfig is used to configure how to serve a Dioxus application. It contains information about how to serve static assets, and what content to render with [`dioxus-ssr`].
#[derive(Clone, Default)]
pub struct ServeConfigBuilder {
    pub(crate) root_id: Option<&'static str>,
    pub(crate) index_html: Option<String>,
    pub(crate) index_path: Option<PathBuf>,
    pub(crate) assets_path: Option<PathBuf>,
    pub(crate) incremental:
        Option<std::sync::Arc<dioxus_ssr::incremental::IncrementalRendererConfig>>,
}

impl ServeConfigBuilder {
    /// Create a new ServeConfigBuilder with the root component and props to render on the server.
    pub fn new() -> Self {
        Self {
            root_id: None,
            index_html: None,
            index_path: None,
            assets_path: None,
            incremental: None,
        }
    }

    /// Enable incremental static generation
    pub fn incremental(mut self, cfg: dioxus_ssr::incremental::IncrementalRendererConfig) -> Self {
        self.incremental = Some(std::sync::Arc::new(cfg));
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
    pub fn root_id(mut self, root_id: &'static str) -> Self {
        self.root_id = Some(root_id);
        self
    }

    /// Set the path of the assets folder generated by the Dioxus CLI. (defaults to dist)
    pub fn assets_path(mut self, assets_path: PathBuf) -> Self {
        self.assets_path = Some(assets_path);
        self
    }

    /// Build the ServeConfig
    pub fn build(self) -> ServeConfig {
        let assets_path = self.assets_path.unwrap_or(
            dioxus_cli_config::CURRENT_CONFIG
                .as_ref()
                .map(|c| c.dioxus_config.application.out_dir.clone())
                .unwrap_or("dist".into()),
        );

        let index_path = self
            .index_path
            .map(PathBuf::from)
            .unwrap_or_else(|| assets_path.join("index.html"));

        let root_id = self.root_id.unwrap_or("main");

        let index_html = self
            .index_html
            .unwrap_or_else(|| load_index_path(index_path));

        let index = load_index_html(index_html, root_id);
        ServeConfig {
            index,
            assets_path,
            incremental: self.incremental,
        }
    }
}

fn load_index_path(path: PathBuf) -> String {
    let mut file = File::open(path).expect("Failed to find index.html. Make sure the index_path is set correctly and the WASM application has been built.");

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read index.html");
    contents
}

fn load_index_html(contents: String, root_id: &'static str) -> IndexHtml {
    let (pre_main, post_main) = contents.split_once(&format!("id=\"{root_id}\"")).unwrap_or_else(|| panic!("Failed to find id=\"{root_id}\" in index.html. The id is used to inject the application into the page."));

    let post_main = post_main.split_once('>').unwrap_or_else(|| {
        panic!("Failed to find closing > after id=\"{root_id}\" in index.html.")
    });

    let (pre_main, post_main) = (
        pre_main.to_string() + &format!("id=\"{root_id}\"") + post_main.0 + ">",
        post_main.1.to_string(),
    );

    IndexHtml {
        pre_main,
        post_main,
    }
}

#[derive(Clone)]
pub(crate) struct IndexHtml {
    pub(crate) pre_main: String,
    pub(crate) post_main: String,
}

/// Used to configure how to serve a Dioxus application. It contains information about how to serve static assets, and what content to render with [`dioxus-ssr`].
/// See [`ServeConfigBuilder`] to create a ServeConfig
#[derive(Clone)]
pub struct ServeConfig {
    pub(crate) index: IndexHtml,
    #[allow(dead_code)]
    pub(crate) assets_path: PathBuf,
    pub(crate) incremental:
        Option<std::sync::Arc<dioxus_ssr::incremental::IncrementalRendererConfig>>,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl ServeConfig {
    /// Create a new builder for a ServeConfig
    pub fn builder() -> ServeConfigBuilder {
        ServeConfigBuilder::new()
    }
}

impl From<ServeConfigBuilder> for ServeConfig {
    fn from(builder: ServeConfigBuilder) -> Self {
        builder.build()
    }
}
