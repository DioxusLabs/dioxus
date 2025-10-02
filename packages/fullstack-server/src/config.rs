//! Configuration for how to serve a Dioxus application
#![allow(non_snake_case)]

use dioxus_core::LaunchConfig;
use std::any::Any;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use crate::IncrementalRendererConfig;

#[allow(unused)]
pub(crate) type ContextProviders =
    Arc<Vec<Box<dyn Fn() -> Box<dyn std::any::Any> + Send + Sync + 'static>>>;

/// A ServeConfig is used to configure how to serve a Dioxus application. It contains information about how to serve static assets, and what content to render with [`dioxus-ssr`].
#[derive(Clone, Default)]
pub struct ServeConfigBuilder {
    pub(crate) root_id: Option<&'static str>,
    pub(crate) index_html: Option<String>,
    pub(crate) index_path: Option<PathBuf>,
    pub(crate) incremental: Option<IncrementalRendererConfig>,
    pub(crate) context_providers:
        Vec<Arc<dyn Fn() -> Box<dyn std::any::Any> + Send + Sync + 'static>>,
    pub(crate) streaming_mode: StreamingMode,
}

impl LaunchConfig for ServeConfigBuilder {}

impl ServeConfigBuilder {
    /// Create a new ServeConfigBuilder with incremental static generation disabled and the default index.html settings
    pub fn new() -> Self {
        Self {
            root_id: None,
            index_html: None,
            index_path: None,
            incremental: None,
            context_providers: Default::default(),
            streaming_mode: StreamingMode::default(),
        }
    }

    /// Enable incremental static generation. Incremental static generation caches the
    /// rendered html in memory and/or the file system. It can be used to improve performance of heavy routes.
    ///
    /// ```rust, no_run
    /// # fn app() -> Element { unimplemented!() }
    /// use dioxus::prelude::*;
    ///
    /// // Finally, launch the app with the config
    /// LaunchBuilder::new()
    ///     // Only set the server config if the server feature is enabled
    ///     .with_cfg(server_only!(dioxus_server::ServeConfigBuilder::default().incremental(dioxus_server::IncrementalRendererConfig::default())))
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
    /// # fn app() -> Element { unimplemented!() }
    /// use dioxus::prelude::*;
    ///
    /// // Finally, launch the app with the config
    /// LaunchBuilder::new()
    ///     // Only set the server config if the server feature is enabled
    ///     .with_cfg(server_only! {
    ///         dioxus_server::ServeConfigBuilder::default().root_id("app")
    ///     })
    ///     // You also need to set the root id in your web config
    ///     .with_cfg(web! {
    ///         dioxus::web::Config::default().rootname("app")
    ///     })
    ///     // And desktop config
    ///     .with_cfg(desktop! {
    ///         dioxus::desktop::Config::default().with_root_name("app")
    ///     })
    ///     .launch(app);
    /// ```
    pub fn root_id(mut self, root_id: &'static str) -> Self {
        self.root_id = Some(root_id);
        self
    }

    /// Provide context to the root and server functions. You can use this context while rendering with [`consume_context`](dioxus_core::consume_context).
    ///
    ///
    /// The context providers passed into this method will be called when the context type is requested which may happen many times in the lifecycle of the application.
    ///
    ///
    /// Context will be forwarded from the LaunchBuilder if it is provided.
    ///
    /// ```rust, no_run
    /// #![allow(non_snake_case)]
    /// use dioxus::prelude::*;
    /// use std::sync::Arc;
    /// use std::any::Any;
    ///
    /// fn main() {
    ///     #[cfg(feature = "web")]
    ///     // Hydrate the application on the client
    ///     dioxus::launch(app);
    ///     #[cfg(feature = "server")]
    ///     {
    ///         tokio::runtime::Runtime::new()
    ///             .unwrap()
    ///             .block_on(async move {
    ///                 let address = dioxus_cli_config::fullstack_address_or_localhost();
    ///                 let listener = tokio::net::TcpListener::bind(address)
    ///                     .await
    ///                     .unwrap();
    ///                 let config = ServeConfigBuilder::default()
    ///                     // You can provide context to your whole app on the server (including server functions) with the `context_provider` method on the launch builder
    ///                     .context_providers(Arc::new(vec![Box::new(|| Box::new(1234u32) as Box<dyn Any>) as Box<dyn Fn() -> Box<dyn std::any::Any> + Send + Sync>]));
    ///                 axum::serve(
    ///                         listener,
    ///                         axum::Router::new()
    ///                             .serve_dioxus_application(config, app)
    ///                             .into_make_service(),
    ///                     )
    ///                     .await
    ///                     .unwrap();
    ///             });
    ///      }
    /// }
    ///
    /// #[server]
    /// async fn read_context() -> ServerFnResult<u32> {
    ///     Ok(123)
    /// }
    ///
    /// fn app() -> Element {
    ///     let future = use_resource(read_context);
    ///     rsx! {
    ///         h1 { "{future:?}" }
    ///     }
    /// }
    /// ```
    pub fn context_providers(mut self, state: ContextProviders) -> Self {
        // This API should probably accept Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync + 'static>> instead of Arc so we can
        // continue adding to the context list after calling this method. Changing the type is a breaking change so it cannot
        // be done until 0.7 is released.
        let context_providers = (0..state.len()).map(|i| {
            let state = state.clone();
            Arc::new(move || state[i]()) as Arc<dyn Fn() -> Box<dyn std::any::Any> + Send + Sync>
        });
        self.context_providers.extend(context_providers);
        self
    }

    /// Provide context to the root and server functions. You can use this context
    /// while rendering with [`consume_context`](dioxus_core::consume_context).
    ///
    ///
    /// The context providers passed into this method will be called when the context type is requested which may happen many times in the lifecycle of the application.
    ///
    ///
    /// Context will be forwarded from the LaunchBuilder if it is provided.
    ///
    /// ```rust, no_run
    /// #![allow(non_snake_case)]
    /// use dioxus::prelude::*;
    ///
    /// fn main() {
    ///     #[cfg(feature = "web")]
    ///     // Hydrate the application on the client
    ///     dioxus::launch(app);
    ///     #[cfg(feature = "server")]
    ///     {
    ///         tokio::runtime::Runtime::new()
    ///             .unwrap()
    ///             .block_on(async move {
    ///                 let address = dioxus_cli_config::fullstack_address_or_localhost();
    ///                 let listener = tokio::net::TcpListener::bind(address)
    ///                     .await
    ///                     .unwrap();
    ///                 let config = ServeConfigBuilder::default()
    ///                     // You can provide context to your whole app on the server (including server functions) with the `context_provider` method on the launch builder
    ///                     .context_provider(move || 1234u32);
    ///                 axum::serve(
    ///                         listener,
    ///                         axum::Router::new()
    ///                             .serve_dioxus_application(config, app)
    ///                             .into_make_service(),
    ///                     )
    ///                     .await
    ///                     .unwrap();
    ///             });
    ///      }
    /// }
    ///
    /// #[server]
    /// async fn read_context() -> ServerFnResult<u32> {
    ///     Ok(123)
    /// }
    ///
    /// fn app() -> Element {
    ///     let future = use_resource(read_context);
    ///     rsx! {
    ///         h1 { "{future:?}" }
    ///     }
    /// }
    /// ```
    pub fn context_provider<C: 'static>(
        mut self,
        state: impl Fn() -> C + Send + Sync + 'static,
    ) -> Self {
        self.context_providers
            .push(Arc::new(move || Box::new(state())));
        self
    }

    /// Provide context to the root and server functions. You can use this context while rendering with [`consume_context`](dioxus_core::consume_context).
    ///
    /// Context will be forwarded from the LaunchBuilder if it is provided.
    ///
    /// ```rust, no_run
    /// #![allow(non_snake_case)]
    /// use dioxus::prelude::*;
    ///
    /// fn main() {
    ///     #[cfg(feature = "web")]
    ///     // Hydrate the application on the client
    ///     dioxus::launch(app);
    ///     #[cfg(feature = "server")]
    ///     {
    ///         tokio::runtime::Runtime::new()
    ///             .unwrap()
    ///             .block_on(async move {
    ///                 let address = dioxus_cli_config::fullstack_address_or_localhost();
    ///                 let listener = tokio::net::TcpListener::bind(address)
    ///                     .await
    ///                     .unwrap();
    ///                 let config = ServeConfigBuilder::default()
    ///                     // You can provide context to your whole app on the server (including server functions) with the `context_provider` method on the launch builder
    ///                     .context(1234u32);
    ///                 axum::serve(
    ///                         listener,
    ///                         axum::Router::new()
    ///                             .serve_dioxus_application(config, app)
    ///                             .into_make_service(),
    ///                     )
    ///                     .await
    ///                     .unwrap();
    ///             });
    ///      }
    /// }
    ///
    /// #[server]
    /// async fn read_context() -> ServerFnResult<u32> {
    ///     Ok(123)
    /// }
    ///
    /// fn app() -> Element {
    ///     let future = use_resource(read_context);
    ///     rsx! {
    ///         h1 { "{future:?}" }
    ///     }
    /// }
    /// ```
    pub fn context(mut self, state: impl Any + Clone + Send + Sync + 'static) -> Self {
        self.context_providers
            .push(Arc::new(move || Box::new(state.clone())));
        self
    }

    /// Set the streaming mode for the server. By default, streaming is disabled.
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # fn app() -> Element { unimplemented!() }
    /// dioxus::LaunchBuilder::new()
    ///     .with_context(server_only! {
    ///         dioxus::server::ServeConfig::builder().streaming_mode(dioxus::server::StreamingMode::OutOfOrder)
    ///     })
    ///     .launch(app);
    /// ```
    pub fn streaming_mode(mut self, mode: StreamingMode) -> Self {
        self.streaming_mode = mode;
        self
    }

    /// Enable out of order streaming. This will cause server futures to be resolved out of order and streamed to the client as they resolve.
    ///
    /// It is equivalent to calling `streaming_mode(StreamingMode::OutOfOrder)`
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # fn app() -> Element { unimplemented!() }
    /// dioxus::LaunchBuilder::new()
    ///     .with_context(server_only! {
    ///         dioxus::server::ServeConfig::builder().enable_out_of_order_streaming()
    ///     })
    ///     .launch(app);
    /// ```
    pub fn enable_out_of_order_streaming(mut self) -> Self {
        self.streaming_mode = StreamingMode::OutOfOrder;
        self
    }

    /// Build the ServeConfig. This may fail if the index.html file is not found.
    ///
    /// ## WASM compatibility
    /// In the context of WASM the file system normally can't be read so this always requires
    /// the `index_html` field to be set.
    pub fn build(self) -> Result<ServeConfig, UnableToLoadIndex> {
        let root_id = self.root_id.unwrap_or("main");

        let index_html = match self.index_html {
            Some(index) => index,
            None => {
                // The CLI always bundles static assets into the exe/public directory
                let public_path = public_path();

                let index_path = self
                    .index_path
                    .unwrap_or_else(|| public_path.join("index.html"));
                load_index_path(index_path).unwrap_or_else(|_| {
                    r#"
                    <!DOCTYPE html>
                    <html>
                        <head> </head>
                        <body>
                            <div id="main"></div>
                        </body>
                    </html>
                    "#
                    .to_string()
                })
            }
        };

        let index = load_index_html(index_html, root_id);
        let context_providers = Arc::new(
            self.context_providers
                .into_iter()
                .map(|f| {
                    Box::new(move || f()) as Box<dyn Fn() -> Box<dyn std::any::Any> + Send + Sync>
                })
                .collect(),
        );

        Ok(ServeConfig {
            index,
            incremental: self.incremental,
            context_providers,
            streaming_mode: self.streaming_mode,
        })
    }
}

impl TryInto<ServeConfig> for ServeConfigBuilder {
    type Error = UnableToLoadIndex;

    fn try_into(self) -> Result<ServeConfig, Self::Error> {
        self.build()
    }
}

/// Get the path to the public assets directory to serve static files from
pub(crate) fn public_path() -> PathBuf {
    if let Ok(path) = std::env::var("DIOXUS_PUBLIC_PATH") {
        return PathBuf::from(path);
    }

    // The CLI always bundles static assets into the exe/public directory
    std::env::current_exe()
        .expect("Failed to get current executable path")
        .parent()
        .unwrap()
        .join("public")
}

/// An error that can occur when loading the index.html file
#[derive(Debug, Clone)]
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

#[derive(Clone, Debug)]
pub(crate) struct IndexHtml {
    pub(crate) head_before_title: String,
    pub(crate) head_after_title: String,
    pub(crate) title: String,
    pub(crate) close_head: String,
    pub(crate) post_main: String,
    pub(crate) after_closing_body_tag: String,
}

/// The streaming mode to use while rendering the page
#[derive(Clone, Copy, Default, PartialEq)]
pub enum StreamingMode {
    /// Streaming is disabled; all server futures should be resolved before hydrating the page on the client
    #[default]
    Disabled,
    /// Out of order streaming is enabled; server futures are resolved out of order and streamed to the client
    /// as they resolve
    OutOfOrder,
}

/// Used to configure how to serve a Dioxus application. It contains information about how to serve static assets, and what content to render with [`dioxus-ssr`].
/// See [`ServeConfigBuilder`] to create a ServeConfig
#[derive(Clone)]
pub struct ServeConfig {
    pub(crate) index: IndexHtml,
    pub(crate) incremental: Option<IncrementalRendererConfig>,
    pub(crate) context_providers: ContextProviders,
    pub(crate) streaming_mode: StreamingMode,
}

impl LaunchConfig for ServeConfig {}

impl ServeConfig {
    /// Create a new ServeConfig
    #[cfg(not(target_family = "wasm"))]
    pub fn new() -> Result<Self, UnableToLoadIndex> {
        ServeConfigBuilder::new().build()
    }

    /// Create a new builder for a ServeConfig
    pub fn builder() -> ServeConfigBuilder {
        ServeConfigBuilder::new()
    }
}
