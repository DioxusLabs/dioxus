//! Configuration for how to serve a Dioxus application
#![allow(non_snake_case)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

/// A ServeConfig is used to configure how to serve a Dioxus application. It contains information about how to serve static assets, and what content to render with [`dioxus-ssr`].
#[derive(Clone, Default)]
pub struct ServeConfigBuilder {
    pub(crate) root_id: Option<&'static str>,
    pub(crate) index_html: Option<String>,
    pub(crate) index_path: Option<PathBuf>,
    pub(crate) incremental: Option<dioxus_isrg::IncrementalRendererConfig>,
}

impl ServeConfigBuilder {
    /// Create a new ServeConfigBuilder with the root component and props to render on the server.
    pub fn new() -> Self {
        Self {
            root_id: None,
            index_html: None,
            index_path: None,
            incremental: None,
        }
    }

    /// Enable incremental static generation
    pub fn incremental(mut self, cfg: dioxus_isrg::IncrementalRendererConfig) -> Self {
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
    pub fn root_id(mut self, root_id: &'static str) -> Self {
        self.root_id = Some(root_id);
        self
    }

    /// Build the ServeConfig
    pub fn build(self) -> Result<ServeConfig, UnableToLoadIndex> {
        // The CLI always bundles static assets into the exe/public directory
        let public_path = public_path();

        let index_path = self
            .index_path
            .map(PathBuf::from)
            .unwrap_or_else(|| public_path.join("index.html"));

        let root_id = self.root_id.unwrap_or("main");

        let index_html = match self.index_html {
            Some(index) => index,
            None => load_index_path(index_path)?,
        };

        let index = load_index_html(index_html, root_id);

        Ok(ServeConfig {
            index,
            incremental: self.incremental,
        })
    }
}

/// Get the path to the public assets directory to serve static files from
pub(crate) fn public_path() -> PathBuf {
    // The CLI always bundles static assets into the exe/public directory
    std::env::current_exe()
        .expect("Failed to get current executable path")
        .parent()
        .unwrap()
        .join("public")
}

/// An error that can occur when loading the index.html file
#[derive(Debug)]
pub struct UnableToLoadIndex(PathBuf);

impl std::fmt::Display for UnableToLoadIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to find index.html. Make sure the index_path is set correctly and the WASM application has been built. Tried to open file at path: {:?}", self.0)
    }
}

impl std::error::Error for UnableToLoadIndex {}

fn load_index_path(path: PathBuf) -> Result<String, UnableToLoadIndex> {
    let mut file = File::open(&path).map_err(|_| UnableToLoadIndex(path))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read index.html");
    Ok(contents)
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

    let (head, close_head) = pre_main.split_once("</head>").unwrap_or_else(|| {
        panic!("Failed to find closing </head> tag after id=\"{root_id}\" in index.html.")
    });
    let (head, close_head) = (head.to_string(), "</head>".to_string() + close_head);

    let (post_main, after_closing_body_tag) =
        post_main.split_once("</body>").unwrap_or_else(|| {
            panic!("Failed to find closing </body> tag after id=\"{root_id}\" in index.html.")
        });

    // Strip out the head if it exists
    let mut head_before_title = String::new();
    let mut head_after_title = head;
    let mut title = String::new();
    if let Some((new_head_before_title, new_title)) = head_after_title.split_once("<title>") {
        let (new_title, new_head_after_title) = new_title
            .split_once("</title>")
            .expect("Failed to find closing </title> tag after <title> in index.html.");
        title = format!("<title>{new_title}</title>");
        head_before_title = new_head_before_title.to_string();
        head_after_title = new_head_after_title.to_string();
    }

    IndexHtml {
        head_before_title,
        head_after_title,
        title,
        close_head,
        post_main: post_main.to_string(),
        after_closing_body_tag: "</body>".to_string() + after_closing_body_tag,
    }
}

#[derive(Clone)]
pub(crate) struct IndexHtml {
    pub(crate) head_before_title: String,
    pub(crate) head_after_title: String,
    pub(crate) title: String,
    pub(crate) close_head: String,
    pub(crate) post_main: String,
    pub(crate) after_closing_body_tag: String,
}

/// Used to configure how to serve a Dioxus application. It contains information about how to serve static assets, and what content to render with [`dioxus-ssr`].
/// See [`ServeConfigBuilder`] to create a ServeConfig
#[derive(Clone)]
pub struct ServeConfig {
    pub(crate) index: IndexHtml,
    pub(crate) incremental: Option<dioxus_isrg::IncrementalRendererConfig>,
}

impl ServeConfig {
    /// Create a new ServeConfig
    pub fn new() -> Result<Self, UnableToLoadIndex> {
        ServeConfigBuilder::new().build()
    }

    /// Create a new builder for a ServeConfig
    pub fn builder() -> ServeConfigBuilder {
        ServeConfigBuilder::new()
    }
}
