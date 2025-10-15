//! Configuration for how to serve a Dioxus application
#![allow(non_snake_case)]

use dioxus_core::LaunchConfig;
use std::any::Any;
use std::sync::Arc;

use crate::{IncrementalRendererConfig, IndexHtml};

#[allow(unused)]
pub(crate) type ContextProviders = Arc<Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync + 'static>>>;

/// A ServeConfig is used to configure how to serve a Dioxus application. It contains information about how to serve static assets, and what content to render with [`dioxus_ssr`].
#[derive(Clone, Default)]
pub struct ServeConfig {
    pub(crate) root_id: Option<&'static str>,
    pub(crate) index: IndexHtml,
    pub(crate) index_html_override: Option<String>,
    pub(crate) incremental: Option<IncrementalRendererConfig>,
    pub(crate) context_providers: Vec<Arc<dyn Fn() -> Box<dyn Any> + Send + Sync + 'static>>,
    pub(crate) streaming_mode: StreamingMode,
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

impl LaunchConfig for ServeConfig {}

impl ServeConfig {
    /// Create a new ServeConfig with incremental static generation disabled and the default index.html settings.
    pub fn builder() -> Self {
        Self::new()
    }

    /// Create a new ServeConfig with incremental static generation disabled and the default index.html settings
    pub fn new() -> Self {
        Self {
            root_id: None,
            incremental: None,
            index: IndexHtml::default(),
            index_html_override: None,
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
    ///     .with_cfg(server_only!(dioxus_server::ServeConfig::default().incremental(dioxus_server::IncrementalRendererConfig::default())))
    ///     .launch(app);
    /// ```
    pub fn incremental(mut self, cfg: IncrementalRendererConfig) -> Self {
        self.incremental = Some(cfg);
        self
    }

    /// Set the contents of the index.html file to be served.
    pub fn index_html(mut self, index_html: String) -> Self {
        self.index_html_override = Some(index_html);
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
    ///         dioxus_server::ServeConfig::default().root_id("app")
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
    ///     // Hydrate the application on the client
    ///     #[cfg(not(feature = "server"))]
    ///     dioxus::launch(app);
    ///
    ///    #[cfg(feature = "server")]
    ///    dioxus_server::serve(|| async move {
    ///        use dioxus_server::{axum, ServeConfig, DioxusRouterExt};
    ///
    ///        let config = ServeConfig::default()
    ///            // You can provide context to your whole app on the server (including server functions) with the `context_provider` method on the launch builder
    ///            .context_providers(Arc::new(vec![Box::new(|| Box::new(1234u32) as Box<dyn Any>) as Box<dyn Fn() -> Box<dyn Any> + Send + Sync>]));
    ///
    ///        Ok(
    ///            axum::Router::new()
    ///                .serve_dioxus_application(config, app)
    ///        )
    ///    })
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
    ///     #[cfg(not(feature = "server"))]
    ///     // Hydrate the application on the client
    ///     dioxus::launch(app);
    ///
    ///     #[cfg(feature = "server")]
    ///     dioxus_server::serve(|| async move {
    ///        use dioxus_server::{axum, ServeConfig, DioxusRouterExt};
    ///
    ///         let config = ServeConfig::default()
    ///             // You can provide context to your whole app on the server (including server functions) with the `context_provider` method on the launch builder
    ///             .context_provider(|| 1234u32);
    ///
    ///         Ok(
    ///             axum::Router::new()
    ///                 .serve_dioxus_application(config, app)
    ///         )
    ///     });
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
    ///     // Hydrate the application on the client
    ///     #[cfg(not(feature = "server"))]
    ///     dioxus::launch(app);
    ///
    ///     // Run a custom server with axum on the server
    ///     #[cfg(feature = "server")]
    ///     dioxus_server::serve(|| async move {
    ///         use dioxus_server::{axum, ServeConfig, DioxusRouterExt};
    ///
    ///         let config = ServeConfig::default()
    ///             // You can provide context to your whole app on the server (including server functions) with the `context_provider` method on the launch builder
    ///             .context(1234u32);
    ///
    ///         Ok(
    ///             axum::Router::new()
    ///                 .serve_dioxus_application(config, app)
    ///         )
    ///     });
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
}
