//! Dioxus utilities for the [Axum](https://docs.rs/axum/latest/axum/index.html) server framework.
//!
//! # Example
//! ```rust
//! #![allow(non_snake_case)]
//! use dioxus::prelude::*;
//! use dioxus_server::prelude::*;
//!
//! fn main() {
//!     #[cfg(feature = "web")]
//!     // Hydrate the application on the client
//!     dioxus_web::launch_cfg(app, dioxus_web::Config::new().hydrate(true));
//!     #[cfg(feature = "ssr")]
//!     {
//!         GetServerData::register().unwrap();
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
    body::{self, Body, BoxBody, Full},
    extract::{State, WebSocketUpgrade},
    handler::Handler,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use dioxus_core::VirtualDom;
use server_fn::{Encoding, Payload, ServerFunctionRegistry};
use std::error::Error;
use std::sync::Arc;
use tokio::task::spawn_blocking;

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
    /// use dioxus_server::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    ///    axum::Server::bind(&addr)
    ///        .serve(
    ///            axum::Router::new()
    ///                .register_server_fns_with_handler("", |func| {
    ///                    move |headers: HeaderMap, body: Request<Body>| async move {
    ///                        // Add the headers to the context
    ///                        server_fn_handler((headers.clone(),), func.clone(), headers, body).await
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
        handler: impl Fn(ServerFunction) -> H,
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
    /// use dioxus_server::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    ///     axum::Server::bind(&addr)
    ///         .serve(
    ///             axum::Router::new()
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
    /// use dioxus_server::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     hot_reload_init!();
    ///     let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    ///     axum::Server::bind(&addr)
    ///         .serve(
    ///             axum::Router::new()
    ///                 // Server side render the application, serve static assets, and register server functions
    ///                 .connect_hot_reload()
    ///                 .into_make_service(),
    ///         )
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    fn connect_hot_reload(self) -> Self;

    /// Serves the Dioxus application. This will serve a complete server side rendered application.
    /// This will serve static assets, server render the application, register server functions, and intigrate with hot reloading.
    ///
    /// # Example
    /// ```rust
    /// #![allow(non_snake_case)]
    /// use dioxus::prelude::*;
    /// use dioxus_server::prelude::*;
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
        mut handler: impl FnMut(ServerFunction) -> H,
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
            match func.encoding {
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
            move |req: Request<Body>| async move {
                let (parts, body) = req.into_parts();
                let parts: Arc<RequestParts> = Arc::new(parts.into());
                let server_context = DioxusServerContext::new(parts.clone());
                server_fn_handler(server_context, func.clone(), parts, body).await
            }
        })
    }

    fn serve_dioxus_application<P: Clone + serde::Serialize + Send + Sync + 'static>(
        mut self,
        server_fn_route: &'static str,
        cfg: impl Into<ServeConfig<P>>,
    ) -> Self {
        use tower_http::services::{ServeDir, ServeFile};

        let cfg = cfg.into();

        // Serve all files in dist folder except index.html
        let dir = std::fs::read_dir(cfg.assets_path).unwrap_or_else(|e| {
            panic!(
                "Couldn't read assets directory at {:?}: {}",
                &cfg.assets_path, e
            )
        });

        for entry in dir.flatten() {
            let path = entry.path();
            if path.ends_with("index.html") {
                continue;
            }
            let route = path
                .strip_prefix(cfg.assets_path)
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

        // Add server functions and render index.html
        self.connect_hot_reload()
            .register_server_fns(server_fn_route)
            .route(
                "/",
                get(render_handler).with_state((cfg, SSRState::default())),
            )
    }

    fn connect_hot_reload(self) -> Self {
        #[cfg(all(debug_assertions, feature = "hot-reload", feature = "ssr"))]
        {
            self.route(
                "/_dioxus/hot_reload",
                get(hot_reload_handler).with_state(crate::hot_reload::HotReloadState::default()),
            )
        }
        #[cfg(not(all(debug_assertions, feature = "hot-reload", feature = "ssr")))]
        {
            self
        }
    }
}

async fn render_handler<P: Clone + serde::Serialize + Send + Sync + 'static>(
    State((cfg, ssr_state)): State<(ServeConfig<P>, SSRState)>,
    request: Request<Body>,
) -> impl IntoResponse {
    let (parts, _) = request.into_parts();
    let parts: Arc<RequestParts> = Arc::new(parts.into());
    let server_context = DioxusServerContext::new(parts);
    let mut vdom =
        VirtualDom::new_with_props(cfg.app, cfg.props.clone()).with_root_context(server_context);
    let _ = vdom.rebuild();

    let rendered = ssr_state.render_vdom(&vdom, &cfg);
    Full::from(rendered)
}

/// A default handler for server functions. It will deserialize the request body, call the server function, and serialize the response.
pub async fn server_fn_handler(
    server_context: DioxusServerContext,
    function: ServerFunction,
    parts: Arc<RequestParts>,
    body: Body,
) -> impl IntoResponse {
    let body = hyper::body::to_bytes(body).await;
    let Ok(body) = body else {
        return report_err(body.err().unwrap());
    };

    // Because the future returned by `server_fn_handler` is `Send`, and the future returned by this function must be send, we need to spawn a new runtime
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
    let query_string = parts.uri.query().unwrap_or_default().to_string();
    spawn_blocking({
        move || {
            tokio::runtime::Runtime::new()
                .expect("couldn't spawn runtime")
                .block_on(async {
                    let query = &query_string.into();
                    let data = match &function.encoding {
                        Encoding::Url | Encoding::Cbor => &body,
                        Encoding::GetJSON | Encoding::GetCBOR => query,
                    };
                    let resp = match (function.trait_obj)(server_context.clone(), &data).await {
                        Ok(serialized) => {
                            // if this is Accept: application/json then send a serialized JSON response
                            let accept_header = parts
                                .headers
                                .get("Accept")
                                .and_then(|value| value.to_str().ok());
                            let mut res = Response::builder();
                            *res.headers_mut().expect("empty responce should be valid") =
                                server_context.take_responce_headers();
                            if accept_header == Some("application/json")
                                || accept_header
                                    == Some(
                                        "application/\
                                                 x-www-form-urlencoded",
                                    )
                                || accept_header == Some("application/cbor")
                            {
                                res = res.status(StatusCode::OK);
                            }

                            let resp = match serialized {
                                Payload::Binary(data) => res
                                    .header("Content-Type", "application/cbor")
                                    .body(body::boxed(Full::from(data))),
                                Payload::Url(data) => res
                                    .header(
                                        "Content-Type",
                                        "application/\
                                        x-www-form-urlencoded",
                                    )
                                    .body(body::boxed(data)),
                                Payload::Json(data) => res
                                    .header("Content-Type", "application/json")
                                    .body(body::boxed(data)),
                            };

                            resp.unwrap()
                        }
                        Err(e) => report_err(e),
                    };

                    resp_tx.send(resp).unwrap();
                })
        }
    });
    resp_rx.await.unwrap()
}

fn report_err<E: Error>(e: E) -> Response<BoxBody> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(body::boxed(format!("Error: {}", e)))
        .unwrap()
}

/// A handler for Dioxus web hot reload websocket. This will send the updated static parts of the RSX to the client when they change.
#[cfg(all(debug_assertions, feature = "hot-reload", feature = "ssr"))]
pub async fn hot_reload_handler(
    ws: WebSocketUpgrade,
    State(state): State<crate::hot_reload::HotReloadState>,
) -> impl IntoResponse {
    use axum::extract::ws::Message;
    use futures_util::StreamExt;

    ws.on_upgrade(|mut socket| async move {
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

        let mut rx = tokio_stream::wrappers::WatchStream::from_changes(state.message_receiver);
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
