//! Dioxus utilities for the [Salvo](https://salvo.rs) server framework.
//!
//! # Example
//! ```rust
//! #![allow(non_snake_case)]
//! use dioxus::prelude::*;
//! use dioxus_fullstack::prelude::*;
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
//! ```

use dioxus_core::VirtualDom;
use hyper::{http::HeaderValue, StatusCode};
use salvo::{
    async_trait, handler,
    serve_static::{StaticDir, StaticFile},
    Depot, FlowCtrl, Handler, Request, Response, Router,
};
use server_fn::{Encoding, Payload, ServerFunctionRegistry};
use std::error::Error;
use std::sync::Arc;
use tokio::task::spawn_blocking;

use crate::{
    prelude::*, render::SSRState, serve_config::ServeConfig, server_fn::DioxusServerFnRegistry,
};

/// A extension trait with utilities for integrating Dioxus with your Salvo router.
pub trait DioxusRouterExt {
    /// Registers server functions with a custom handler function. This allows you to pass custom context to your server functions by generating a [`DioxusServerContext`] from the request.
    ///
    /// # Example
    /// ```rust
    /// use salvo::prelude::*;
    /// use std::{net::TcpListener, sync::Arc};
    /// use dioxus_fullstack::prelude::*;
    ///
    /// struct ServerFunctionHandler {
    ///     server_fn: ServerFunction,
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
        handler: impl Fn(ServerFunction) -> H,
    ) -> Self
    where
        H: Handler + 'static;

    /// Registers server functions with the default handler. This handler function will pass an empty [`DioxusServerContext`] to your server functions.
    ///
    /// # Example
    /// ```rust
    /// use salvo::prelude::*;
    /// use std::{net::TcpListener, sync::Arc};
    /// use dioxus_fullstack::prelude::*;
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
    /// use dioxus_fullstack::prelude::*;
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

    /// Serves the static WASM for your Dioxus application (except the generated index.html).
    ///
    /// # Example
    /// ```rust
    /// use salvo::prelude::*;
    /// use std::{net::TcpListener, sync::Arc};
    /// use dioxus_fullstack::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let router = Router::new()
    ///         .server_static_assets("/dist");
    ///     Server::new(TcpListener::bind("127.0.0.1:8080"))
    ///         .serve(router)
    ///         .await;
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
    /// fn app(cx: Scope) -> Element {todo!()}
    /// ```
    fn serve_dioxus_application<P: Clone + serde::Serialize + Send + Sync + 'static>(
        self,
        server_fn_path: &'static str,
        cfg: impl Into<ServeConfig<P>>,
    ) -> Self;
}

impl DioxusRouterExt for Router {
    fn register_server_fns_with_handler<H>(
        self,
        server_fn_route: &'static str,
        mut handler: impl FnMut(ServerFunction) -> H,
    ) -> Self
    where
        H: Handler + 'static,
    {
        let mut router = self;
        for server_fn_path in DioxusServerFnRegistry::paths_registered() {
            let func = DioxusServerFnRegistry::get(server_fn_path).unwrap();
            let full_route = format!("{server_fn_route}/{server_fn_path}");
            match func.encoding {
                Encoding::Url | Encoding::Cbor => {
                    router = router.push(Router::with_path(&full_route).post(handler(func)));
                }
                Encoding::GetJSON | Encoding::GetCBOR => {
                    router = router.push(Router::with_path(&full_route).get(handler(func)));
                }
            }
        }
        router
    }

    fn register_server_fns(self, server_fn_route: &'static str) -> Self {
        self.register_server_fns_with_handler(server_fn_route, |func| ServerFnHandler {
            server_context: DioxusServerContext::default(),
            function: func,
        })
    }

    fn serve_static_assets(mut self, assets_path: impl Into<std::path::PathBuf>) -> Self {
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

        self
    }

    fn serve_dioxus_application<P: Clone + serde::Serialize + Send + Sync + 'static>(
        self,
        server_fn_path: &'static str,
        cfg: impl Into<ServeConfig<P>>,
    ) -> Self {
        let cfg = cfg.into();

        self.serve_static_assets(&cfg.assets_path)
            .connect_hot_reload()
            .register_server_fns(server_fn_path)
            .push(Router::with_path("/").get(SSRHandler { cfg }))
    }

    fn connect_hot_reload(self) -> Self {
        let mut _dioxus_router = Router::with_path("_dioxus");
        _dioxus_router = _dioxus_router
            .push(Router::with_path("hot_reload").handle(HotReloadHandler::default()));
        #[cfg(all(debug_assertions, feature = "hot-reload", feature = "ssr"))]
        {
            _dioxus_router = _dioxus_router.push(Router::with_path("disconnect").handle(ignore_ws));
        }
        self.push(_dioxus_router)
    }
}

/// Extracts the parts of a request that are needed for server functions. This will take parts of the request and replace them with empty values.
pub fn extract_parts(req: &mut Request) -> RequestParts {
    RequestParts {
        method: std::mem::take(req.method_mut()),
        uri: std::mem::take(req.uri_mut()),
        version: req.version(),
        headers: std::mem::take(req.headers_mut()),
        extensions: std::mem::take(req.extensions_mut()),
    }
}

struct SSRHandler<P: Clone> {
    cfg: ServeConfig<P>,
}

#[async_trait]
impl<P: Clone + serde::Serialize + Send + Sync + 'static> Handler for SSRHandler<P> {
    async fn handle(
        &self,
        req: &mut Request,
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
        let parts: Arc<RequestParts> = Arc::new(extract_parts(req));
        let server_context = DioxusServerContext::new(parts);
        let mut vdom = VirtualDom::new_with_props(self.cfg.app, self.cfg.props.clone())
            .with_root_context(server_context.clone());
        let _ = vdom.rebuild();

        res.write_body(renderer_pool.render_vdom(&vdom, &self.cfg))
            .unwrap();

        *res.headers_mut() = server_context.take_response_headers();
    }
}

/// A default handler for server functions. It will deserialize the request body, call the server function, and serialize the response.
pub struct ServerFnHandler {
    server_context: DioxusServerContext,
    function: ServerFunction,
}

impl ServerFnHandler {
    /// Create a new server function handler with the given server context and server function.
    pub fn new(server_context: impl Into<DioxusServerContext>, function: ServerFunction) -> Self {
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

        let query = req
            .uri()
            .query()
            .unwrap_or_default()
            .as_bytes()
            .to_vec()
            .into();
        let body = hyper::body::to_bytes(req.body_mut().unwrap()).await;
        let Ok(body)=body else {
            handle_error(body.err().unwrap(), res);
            return;
        };
        let headers = req.headers();
        let accept_header = headers.get("Accept").cloned();

        let parts = Arc::new(extract_parts(req));

        // Because the future returned by `server_fn_handler` is `Send`, and the future returned by this function must be send, we need to spawn a new runtime
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        spawn_blocking({
            let function = function.clone();
            let mut server_context = server_context.clone();
            server_context.parts = parts;
            move || {
                tokio::runtime::Runtime::new()
                    .expect("couldn't spawn runtime")
                    .block_on(async move {
                        let data = match &function.encoding {
                            Encoding::Url | Encoding::Cbor => &body,
                            Encoding::GetJSON | Encoding::GetCBOR => &query,
                        };
                        let resp = (function.trait_obj)(server_context, data).await;

                        resp_tx.send(resp).unwrap();
                    })
            }
        });
        let result = resp_rx.await.unwrap();

        // Set the headers from the server context
        *res.headers_mut() = server_context.take_response_headers();

        match result {
            Ok(serialized) => {
                // if this is Accept: application/json then send a serialized JSON response
                let accept_header = accept_header.as_ref().and_then(|value| value.to_str().ok());
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
pub struct HotReloadHandler;

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

        let state = crate::hot_reload::spawn_hot_reload().await;

        WebSocketUpgrade::new()
            .upgrade(req, res, move |mut websocket| async move {
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

                let mut rx = tokio_stream::wrappers::WatchStream::from_changes(
                    state.message_receiver.clone(),
                );
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

#[cfg(all(debug_assertions, feature = "hot-reload", feature = "ssr"))]
#[handler]
async fn ignore_ws(req: &mut Request, res: &mut Response) -> Result<(), salvo::http::StatusError> {
    use salvo::ws::WebSocketUpgrade;
    WebSocketUpgrade::new()
        .upgrade(req, res, |mut ws| async move {
            let _ = ws.send(salvo::ws::Message::text("connected")).await;
            while let Some(msg) = ws.recv().await {
                if msg.is_err() {
                    return;
                };
            }
        })
        .await
}
