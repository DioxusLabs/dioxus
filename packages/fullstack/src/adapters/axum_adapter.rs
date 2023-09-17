//! Dioxus utilities for the [Axum](https://docs.rs/axum/latest/axum/index.html) server framework.
//!
//! # Example
//! ```rust
//! #![allow(non_snake_case)]
//! use dioxus::prelude::*;
//! use dioxus_fullstack::prelude::*;
//!
//! fn main() {
//!     #[cfg(feature = "web")]
//!     // Hydrate the application on the client
//!     dioxus_web::launch_cfg(app, dioxus_web::Config::new().hydrate(true));
//!     #[cfg(feature = "ssr")]
//!     {
//!         tokio::runtime::Runtime::new()
//!             .unwrap()
//!             .block_on(async move {
//!                 let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
//!                 axum::Server::bind(&addr)
//!                     .serve(
//!                         axum::Router::new()
//!                             // Server side render the application, serve static assets, and register server functions
//!                             .serve_dioxus_application("", ServeConfigBuilder::new(app, ()))
//!                             .into_make_service(),
//!                     )
//!                     .await
//!                     .unwrap();
//!             });
//!      }
//! }
//!
//! fn app(cx: Scope) -> Element {
//!     let text = use_state(cx, || "...".to_string());
//!
//!     cx.render(rsx! {
//!         button {
//!             onclick: move |_| {
//!                 to_owned![text];
//!                 async move {
//!                     if let Ok(data) = get_server_data().await {
//!                         text.set(data);
//!                     }
//!                 }
//!             },
//!             "Run a server function"
//!         }
//!         "Server said: {text}"
//!     })
//! }
//!
//! #[server(GetServerData)]
//! async fn get_server_data() -> Result<String, ServerFnError> {
//!     Ok("Hello from the server!".to_string())
//! }
//! ```

use axum::{
    body::{self, Body, BoxBody},
    extract::State,
    handler::Handler,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use server_fn::{Encoding, ServerFunctionRegistry};
use std::sync::Arc;
use std::sync::RwLock;

use crate::{
    prelude::*, render::SSRState, serve_config::ServeConfig, server_context::DioxusServerContext,
    server_fn::DioxusServerFnRegistry,
};

/// A extension trait with utilities for integrating Dioxus with your Axum router.
pub trait DioxusRouterExt<S> {
    /// Registers server functions with a custom handler function. This allows you to pass custom context to your server functions by generating a [`DioxusServerContext`] from the request.
    ///
    /// # Example
    /// ```rust
    /// use dioxus::prelude::*;
    /// use dioxus_fullstack::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    ///    axum::Server::bind(&addr)
    ///        .serve(
    ///            axum::Router::new()
    ///                .register_server_fns_with_handler("", |func| {
    ///                    move |req: Request<Body>| async move {
    ///                        let (parts, body) = req.into_parts();
    ///                        let parts: Arc<http::request::Parts> = Arc::new(parts.into());
    ///                        let server_context = DioxusServerContext::new(parts.clone());
    ///                        server_fn_handler(server_context, func.clone(), parts, body).await
    ///                    }
    ///                })
    ///                .into_make_service(),
    ///        )
    ///        .await
    ///        .unwrap();
    /// }
    /// ```
    fn register_server_fns_with_handler<H, T>(
        self,
        server_fn_route: &'static str,
        handler: impl FnMut(server_fn::ServerFnTraitObj<()>) -> H,
    ) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
        S: Clone + Send + Sync + 'static;

    /// Registers server functions with the default handler. This handler function will pass an empty [`DioxusServerContext`] to your server functions.
    ///
    /// # Example
    /// ```rust
    /// use dioxus::prelude::*;
    /// use dioxus_fullstack::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    ///     axum::Server::bind(&addr)
    ///         .serve(
    ///             axum::Router::new()
    ///                 // Register server functions routes with the default handler
    ///                 .register_server_fns("")
    ///                 .into_make_service(),
    ///         )
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    fn register_server_fns(self, server_fn_route: &'static str) -> Self;

    /// Register the web RSX hot reloading endpoint. This will enable hot reloading for your application in debug mode when you call [`dioxus_hot_reload::hot_reload_init`].
    ///
    /// # Example
    /// ```rust
    /// #![allow(non_snake_case)]
    /// use dioxus_fullstack::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     hot_reload_init!();
    ///     let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    ///     axum::Server::bind(&addr)
    ///         .serve(
    ///             axum::Router::new()
    ///                 // Connect to hot reloading in debug mode
    ///                 .connect_hot_reload()
    ///                 .into_make_service(),
    ///         )
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    fn connect_hot_reload(self) -> Self;

    /// Serves the static WASM for your Dioxus application (except the generated index.html).
    ///
    /// # Example
    /// ```rust
    /// #![allow(non_snake_case)]
    /// use dioxus::prelude::*;
    /// use dioxus_fullstack::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    ///     axum::Server::bind(&addr)
    ///         .serve(
    ///             axum::Router::new()
    ///                 // Server side render the application, serve static assets, and register server functions
    ///                 .serve_static_assets(ServeConfigBuilder::new(app, ()))
    ///                 // Server render the application
    ///                 // ...
    ///                 .into_make_service(),
    ///         )
    ///         .await
    ///         .unwrap();
    /// }
    ///
    /// fn app(cx: Scope) -> Element {
    ///     todo!()
    /// }
    /// ```
    fn serve_static_assets(self, assets_path: impl Into<std::path::PathBuf>) -> Self;

    /// Serves the Dioxus application. This will serve a complete server side rendered application.
    /// This will serve static assets, server render the application, register server functions, and intigrate with hot reloading.
    ///
    /// # Example
    /// ```rust
    /// #![allow(non_snake_case)]
    /// use dioxus::prelude::*;
    /// use dioxus_fullstack::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    ///     axum::Server::bind(&addr)
    ///         .serve(
    ///             axum::Router::new()
    ///                 // Server side render the application, serve static assets, and register server functions
    ///                 .serve_dioxus_application("", ServeConfigBuilder::new(app, ()))
    ///                 .into_make_service(),
    ///         )
    ///         .await
    ///         .unwrap();
    /// }
    ///
    /// fn app(cx: Scope) -> Element {
    ///     todo!()
    /// }
    /// ```
    fn serve_dioxus_application<P: Clone + serde::Serialize + Send + Sync + 'static>(
        self,
        server_fn_route: &'static str,
        cfg: impl Into<ServeConfig<P>>,
    ) -> Self;
}

impl<S> DioxusRouterExt<S> for Router<S>
where
    S: Send + Sync + Clone + 'static,
{
    fn register_server_fns_with_handler<H, T>(
        self,
        server_fn_route: &'static str,
        mut handler: impl FnMut(server_fn::ServerFnTraitObj<()>) -> H,
    ) -> Self
    where
        H: Handler<T, S, Body>,
        T: 'static,
        S: Clone + Send + Sync + 'static,
    {
        let mut router = self;
        for server_fn_path in DioxusServerFnRegistry::paths_registered() {
            let func = DioxusServerFnRegistry::get(server_fn_path).unwrap();
            let full_route = format!("{server_fn_route}/{server_fn_path}");
            match func.encoding() {
                Encoding::Url | Encoding::Cbor => {
                    router = router.route(&full_route, post(handler(func)));
                }
                Encoding::GetJSON | Encoding::GetCBOR => {
                    router = router.route(&full_route, get(handler(func)));
                }
            }
        }
        router
    }

    fn register_server_fns(self, server_fn_route: &'static str) -> Self {
        self.register_server_fns_with_handler(server_fn_route, |func| {
            use crate::layer::Service;
            move |req: Request<Body>| {
                let mut service = crate::server_fn_service(Default::default(), func);
                async move {
                    let (req, body) = req.into_parts();
                    let req = Request::from_parts(req, body);
                    let res = service.run(req);
                    match res.await {
                        Ok(res) => Ok::<_, std::convert::Infallible>(res.map(|b| b.into())),
                        Err(e) => {
                            let mut res = Response::new(Body::from(e.to_string()));
                            *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                            Ok(res)
                        }
                    }
                }
            }
        })
    }

    fn serve_static_assets(mut self, assets_path: impl Into<std::path::PathBuf>) -> Self {
        use tower_http::services::{ServeDir, ServeFile};

        let assets_path = assets_path.into();

        // Serve all files in dist folder except index.html
        let dir = std::fs::read_dir(&assets_path).unwrap_or_else(|e| {
            panic!(
                "Couldn't read assets directory at {:?}: {}",
                &assets_path, e
            )
        });

        for entry in dir.flatten() {
            let path = entry.path();
            if path.ends_with("index.html") {
                continue;
            }
            let route = path
                .strip_prefix(&assets_path)
                .unwrap()
                .iter()
                .map(|segment| {
                    segment.to_str().unwrap_or_else(|| {
                        panic!("Failed to convert path segment {:?} to string", segment)
                    })
                })
                .collect::<Vec<_>>()
                .join("/");
            let route = format!("/{}", route);
            if path.is_dir() {
                self = self.nest_service(&route, ServeDir::new(path));
            } else {
                self = self.nest_service(&route, ServeFile::new(path));
            }
        }

        self
    }

    fn serve_dioxus_application<P: Clone + serde::Serialize + Send + Sync + 'static>(
        self,
        server_fn_route: &'static str,
        cfg: impl Into<ServeConfig<P>>,
    ) -> Self {
        let cfg = cfg.into();
        let ssr_state = SSRState::new(&cfg);

        // Add server functions and render index.html
        self.serve_static_assets(cfg.assets_path)
            .connect_hot_reload()
            .register_server_fns(server_fn_route)
            .fallback(get(render_handler).with_state((cfg, ssr_state)))
    }

    fn connect_hot_reload(self) -> Self {
        #[cfg(all(debug_assertions, feature = "hot-reload", feature = "ssr"))]
        {
            self.nest(
                "/_dioxus",
                Router::new()
                    .route(
                        "/disconnect",
                        get(|ws: axum::extract::WebSocketUpgrade| async {
                            ws.on_upgrade(|mut ws| async move {
                                use axum::extract::ws::Message;
                                let _ = ws.send(Message::Text("connected".into())).await;
                                loop {
                                    if ws.recv().await.is_none() {
                                        break;
                                    }
                                }
                            })
                        }),
                    )
                    .route("/hot_reload", get(hot_reload_handler)),
            )
        }
        #[cfg(not(all(debug_assertions, feature = "hot-reload", feature = "ssr")))]
        {
            self
        }
    }
}

fn apply_request_parts_to_response<B>(
    headers: hyper::header::HeaderMap,
    response: &mut axum::response::Response<B>,
) {
    let mut_headers = response.headers_mut();
    for (key, value) in headers.iter() {
        mut_headers.insert(key, value.clone());
    }
}

/// SSR renderer handler for Axum
pub async fn render_handler<P: Clone + serde::Serialize + Send + Sync + 'static>(
    State((cfg, ssr_state)): State<(ServeConfig<P>, SSRState)>,
    request: Request<Body>,
) -> impl IntoResponse {
    let (parts, _) = request.into_parts();
    let url = parts.uri.path_and_query().unwrap().to_string();
    let parts: Arc<RwLock<http::request::Parts>> = Arc::new(RwLock::new(parts.into()));
    let server_context = DioxusServerContext::new(parts.clone());

    match ssr_state.render(url, &cfg, &server_context).await {
        Ok(rendered) => {
            let crate::render::RenderResponse { html, freshness } = rendered;
            let mut response = axum::response::Html::from(html).into_response();
            freshness.write(response.headers_mut());
            let headers = server_context.response_parts().unwrap().headers.clone();
            apply_request_parts_to_response(headers, &mut response);
            response
        }
        Err(e) => {
            tracing::error!("Failed to render page: {}", e);
            report_err(e).into_response()
        }
    }
}

fn report_err<E: std::fmt::Display>(e: E) -> Response<BoxBody> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(body::boxed(format!("Error: {}", e)))
        .unwrap()
}

/// A handler for Dioxus web hot reload websocket. This will send the updated static parts of the RSX to the client when they change.
#[cfg(all(debug_assertions, feature = "hot-reload", feature = "ssr"))]
pub async fn hot_reload_handler(ws: axum::extract::WebSocketUpgrade) -> impl IntoResponse {
    use axum::extract::ws::Message;
    use futures_util::StreamExt;

    let state = crate::hot_reload::spawn_hot_reload().await;

    ws.on_upgrade(move |mut socket| async move {
        println!("🔥 Hot Reload WebSocket connected");
        {
            // update any rsx calls that changed before the websocket connected.
            {
                println!("🔮 Finding updates since last compile...");
                let templates_read = state.templates.read().await;

                for template in &*templates_read {
                    if socket
                        .send(Message::Text(serde_json::to_string(&template).unwrap()))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            }
            println!("finished");
        }

        let mut rx =
            tokio_stream::wrappers::WatchStream::from_changes(state.message_receiver.clone());
        while let Some(change) = rx.next().await {
            if let Some(template) = change {
                let template = { serde_json::to_string(&template).unwrap() };
                if socket.send(Message::Text(template)).await.is_err() {
                    break;
                };
            }
        }
    })
}
