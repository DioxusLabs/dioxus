//! Dioxus utilities for the [Salvo](https://salvo.rs) server framework.
//!
//! # Example
//! ```rust
//! # #![allow(non_snake_case)]
//! # use dioxus::prelude::*;
//! # use dioxus_server::prelude::*;
//!
//! fn main() {
//!     #[cfg(feature = "web")]
//!     dioxus_web::launch_cfg(app, dioxus_web::Config::new().hydrate(true));
//!     #[cfg(feature = "ssr")]
//!     {
//!         use salvo::prelude::*;
//!         GetServerData::register().unwrap();
//!         tokio::runtime::Runtime::new()
//!             .unwrap()
//!             .block_on(async move {
//!                 let router =
//!                     Router::new().serve_dioxus_application("", ServeConfigBuilder::new(app, ()));
//!                 Server::new(TcpListener::bind("127.0.0.1:8080"))
//!                     .serve(router)
//!                     .await;
//!             });
//!     }
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

use std::{error::Error, sync::Arc};

use hyper::{http::HeaderValue, StatusCode};
use salvo::{
    async_trait, handler,
    serve_static::{StaticDir, StaticFile},
    Depot, FlowCtrl, Handler, Request, Response, Router,
};
use server_fn::{Payload, ServerFunctionRegistry};
use tokio::task::spawn_blocking;

use crate::{
    prelude::DioxusServerContext,
    render::SSRState,
    serve_config::ServeConfig,
    server_fn::{DioxusServerFnRegistry, ServerFnTraitObj},
};

/// A extension trait with utilities for integrating Dioxus with your Salvo router.
pub trait DioxusRouterExt {
    /// Registers server functions with a custom handler function. This allows you to pass custom context to your server functions by generating a [`DioxusServerContext`] from the request.
    ///
    /// ```rust
    /// use salvo::prelude::*;
    /// use std::{net::TcpListener, sync::Arc};
    /// use dioxus_server::prelude::*;
    ///
    /// struct ServerFunctionHandler {
    ///     server_fn: Arc<ServerFnTraitObj>,
    /// }
    ///
    /// #[handler]
    /// impl ServerFunctionHandler {
    ///     async fn handle(
    ///         &self,
    ///         req: &mut Request,
    ///         depot: &mut Depot,
    ///         res: &mut Response,
    ///         flow: &mut FlowCtrl,
    ///     ) {
    ///         // Add the headers to server context
    ///         ServerFnHandler::new((req.headers().clone(),), self.server_fn.clone())
    ///             .handle(req, depot, res, flow)
    ///             .await
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let router = Router::new()
    ///         .register_server_fns_with_handler("", |func| {
    ///             ServerFnHandler::new(DioxusServerContext::default(), func)
    ///         });
    ///     Server::new(TcpListener::bind("127.0.0.1:8080"))
    ///         .serve(router)
    ///         .await;
    /// }
    /// ```
    fn register_server_fns_with_handler<H>(
        self,
        server_fn_route: &'static str,
        handler: impl Fn(Arc<ServerFnTraitObj>) -> H,
    ) -> Self
    where
        H: Handler + 'static;

    /// Registers server functions with the default handler. This handler function will pass an empty [`DioxusServerContext`] to your server functions.
    ///
    /// # Example
    /// ```rust
    /// use salvo::prelude::*;
    /// use std::{net::TcpListener, sync::Arc};
    /// use dioxus_server::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let router = Router::new()
    ///         .register_server_fns("");
    ///     Server::new(TcpListener::bind("127.0.0.1:8080"))
    ///         .serve(router)
    ///         .await;
    /// }
    ///
    /// ```
    fn register_server_fns(self, server_fn_route: &'static str) -> Self;

    /// Register the web RSX hot reloading endpoint. This will enable hot reloading for your application in debug mode when you call [`dioxus_hot_reload::hot_reload_init`].
    ///
    /// # Example
    /// ```rust
    /// use salvo::prelude::*;
    /// use std::{net::TcpListener, sync::Arc};
    /// use dioxus_server::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let router = Router::new()
    ///         .connect_hot_reload();
    ///     Server::new(TcpListener::bind("127.0.0.1:8080"))
    ///         .serve(router)
    ///         .await;
    /// }
    fn connect_hot_reload(self) -> Self;

    /// Serves the Dioxus application. This will serve a complete server side rendered application.
    /// This will serve static assets, server render the application, register server functions, and intigrate with hot reloading.
    ///
    /// # Example
    /// ```rust
    /// #![allow(non_snake_case)]
    /// use dioxus::prelude::*;
    /// use dioxus_server::prelude::*;
    /// use salvo::prelude::*;
    /// use std::{net::TcpListener, sync::Arc};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let router = Router::new().serve_dioxus_application("", ServeConfigBuilder::new(app, ()));
    ///     Server::new(TcpListener::bind("127.0.0.1:8080"))
    ///         .serve(router)
    ///         .await;
    /// }
    ///
    /// # fn app(cx: Scope) -> Element {todo!()}
    /// ```    
    fn serve_dioxus_application<P: Clone + Send + Sync + 'static>(
        self,
        server_fn_path: &'static str,
        cfg: impl Into<ServeConfig<P>>,
    ) -> Self;
}

impl DioxusRouterExt for Router {
    fn register_server_fns_with_handler<H>(
        self,
        server_fn_route: &'static str,
        mut handler: impl FnMut(Arc<ServerFnTraitObj>) -> H,
    ) -> Self
    where
        H: Handler + 'static,
    {
        let mut router = self;
        for server_fn_path in DioxusServerFnRegistry::paths_registered() {
            let func = DioxusServerFnRegistry::get(server_fn_path).unwrap();
            let full_route = format!("{server_fn_route}/{server_fn_path}");
            router = router.push(Router::with_path(&full_route).post(handler(func)));
        }
        router
    }

    fn register_server_fns(self, server_fn_route: &'static str) -> Self {
        self.register_server_fns_with_handler(server_fn_route, |func| ServerFnHandler {
            server_context: DioxusServerContext::default(),
            function: func,
        })
    }

    fn serve_dioxus_application<P: Clone + Send + Sync + 'static>(
        mut self,
        server_fn_route: &'static str,
        cfg: impl Into<ServeConfig<P>>,
    ) -> Self {
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
                .strip_prefix(&cfg.assets_path)
                .unwrap()
                .iter()
                .map(|segment| {
                    segment.to_str().unwrap_or_else(|| {
                        panic!("Failed to convert path segment {:?} to string", segment)
                    })
                })
                .collect::<Vec<_>>()
                .join("/");
            if path.is_file() {
                let route = format!("/{}", route);
                let serve_dir = StaticFile::new(path.clone());
                self = self.push(Router::with_path(route).get(serve_dir))
            } else {
                let route = format!("/{}/<**path>", route);
                let serve_dir = StaticDir::new([path.clone()]);
                self = self.push(Router::with_path(route).get(serve_dir))
            }
        }

        self.connect_hot_reload()
            .register_server_fns(server_fn_route)
            .push(Router::with_path("/").get(SSRHandler { cfg }))
    }

    fn connect_hot_reload(self) -> Self {
        self.push(Router::with_path("/_dioxus/hot_reload").get(HotReloadHandler::default()))
    }
}

struct SSRHandler<P: Clone> {
    cfg: ServeConfig<P>,
}

#[async_trait]
impl<P: Clone + Send + Sync + 'static> Handler for SSRHandler<P> {
    async fn handle(
        &self,
        _req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        _flow: &mut FlowCtrl,
    ) {
        // Get the SSR renderer from the depot or create a new one if it doesn't exist
        let renderer_pool = if let Some(renderer) = depot.obtain::<SSRState>() {
            renderer.clone()
        } else {
            let renderer = SSRState::default();
            depot.inject(renderer.clone());
            renderer
        };
        res.write_body(renderer_pool.render(&self.cfg)).unwrap();
    }
}

/// A default handler for server functions. It will deserialize the request body, call the server function, and serialize the response.
pub struct ServerFnHandler {
    server_context: DioxusServerContext,
    function: Arc<ServerFnTraitObj>,
}

impl ServerFnHandler {
    /// Create a new server function handler with the given server context and server function.
    pub fn new(
        server_context: impl Into<DioxusServerContext>,
        function: Arc<ServerFnTraitObj>,
    ) -> Self {
        let server_context = server_context.into();
        Self {
            server_context,
            function,
        }
    }
}

#[handler]
impl ServerFnHandler {
    async fn handle(&self, req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        let Self {
            server_context,
            function,
        } = self;

        let body = hyper::body::to_bytes(req.body_mut().unwrap()).await;
        let Ok(body)=body else {
            handle_error(body.err().unwrap(), res);
            return;
        };
        let headers = req.headers();

        // Because the future returned by `server_fn_handler` is `Send`, and the future returned by this function must be send, we need to spawn a new runtime
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        let function = function.clone();
        let server_context = server_context.clone();
        spawn_blocking({
            move || {
                tokio::runtime::Runtime::new()
                    .expect("couldn't spawn runtime")
                    .block_on(async move {
                        let resp = function(server_context, &body).await;

                        resp_tx.send(resp).unwrap();
                    })
            }
        });
        let result = resp_rx.await.unwrap();

        match result {
            Ok(serialized) => {
                // if this is Accept: application/json then send a serialized JSON response
                let accept_header = headers.get("Accept").and_then(|value| value.to_str().ok());
                if accept_header == Some("application/json")
                    || accept_header
                        == Some(
                            "application/\
                                x-www-form-urlencoded",
                        )
                    || accept_header == Some("application/cbor")
                {
                    res.set_status_code(StatusCode::OK);
                }

                match serialized {
                    Payload::Binary(data) => {
                        res.headers_mut()
                            .insert("Content-Type", HeaderValue::from_static("application/cbor"));
                        res.write_body(data).unwrap();
                    }
                    Payload::Url(data) => {
                        res.headers_mut().insert(
                            "Content-Type",
                            HeaderValue::from_static(
                                "application/\
                                    x-www-form-urlencoded",
                            ),
                        );
                        res.write_body(data).unwrap();
                    }
                    Payload::Json(data) => {
                        res.headers_mut()
                            .insert("Content-Type", HeaderValue::from_static("application/json"));
                        res.write_body(data).unwrap();
                    }
                }
            }
            Err(err) => handle_error(err, res),
        }
    }
}

fn handle_error(error: impl Error + Send + Sync, res: &mut Response) {
    let mut resp_err = Response::new();
    resp_err.set_status_code(StatusCode::INTERNAL_SERVER_ERROR);
    resp_err.render(format!("Internal Server Error: {}", error));
    *res = resp_err;
}

/// A handler for Dioxus web hot reload websocket. This will send the updated static parts of the RSX to the client when they change.
#[cfg(not(all(debug_assertions, feature = "hot-reload", feature = "ssr")))]
#[derive(Default)]
pub struct HotReloadHandler;

#[cfg(not(all(debug_assertions, feature = "hot-reload", feature = "ssr")))]
#[handler]
impl HotReloadHandler {
    async fn handle(
        &self,
        _req: &mut Request,
        _depot: &mut Depot,
        _res: &mut Response,
    ) -> Result<(), salvo::http::StatusError> {
        Err(salvo::http::StatusError::not_found())
    }
}

/// A handler for Dioxus web hot reload websocket. This will send the updated static parts of the RSX to the client when they change.
#[cfg(all(debug_assertions, feature = "hot-reload", feature = "ssr"))]
#[derive(Default)]
pub struct HotReloadHandler {
    state: crate::hot_reload::HotReloadState,
}

#[cfg(all(debug_assertions, feature = "hot-reload", feature = "ssr"))]
#[handler]
impl HotReloadHandler {
    async fn handle(
        &self,
        req: &mut Request,
        _depot: &mut Depot,
        res: &mut Response,
    ) -> Result<(), salvo::http::StatusError> {
        use salvo::ws::Message;
        use salvo::ws::WebSocketUpgrade;

        let state = self.state.clone();

        WebSocketUpgrade::new()
            .upgrade(req, res, |mut websocket| async move {
                use futures_util::StreamExt;

                println!("🔥 Hot Reload WebSocket connected");
                {
                    // update any rsx calls that changed before the websocket connected.
                    {
                        println!("🔮 Finding updates since last compile...");
                        let templates_read = state.templates.read().await;

                        for template in &*templates_read {
                            if websocket
                                .send(Message::text(serde_json::to_string(&template).unwrap()))
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
                    tokio_stream::wrappers::WatchStream::from_changes(state.message_receiver);
                while let Some(change) = rx.next().await {
                    if let Some(template) = change {
                        let template = { serde_json::to_string(&template).unwrap() };
                        if websocket.send(Message::text(template)).await.is_err() {
                            break;
                        };
                    }
                }
            })
            .await
    }
}
