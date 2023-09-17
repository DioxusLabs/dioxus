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

use http_body_util::{BodyExt, Limited};
use hyper::body::Body as HyperBody;
use hyper::StatusCode;
use salvo::{
    async_trait, handler,
    http::{
        cookie::{Cookie, CookieJar},
        ParseError,
    },
    serve_static::{StaticDir, StaticFile},
    Depot, Error as SalvoError, FlowCtrl, Handler, Request, Response, Router,
};
use server_fn::{Encoding, ServerFunctionRegistry};
use std::error::Error;
use std::sync::Arc;
use std::sync::RwLock;

use crate::{
    layer::Service, prelude::*, render::SSRState, serve_config::ServeConfig,
    server_fn::DioxusServerFnRegistry, server_fn_service,
};

type HyperRequest = hyper::Request<hyper::Body>;
type HyperResponse = hyper::Response<HyperBody>;

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
    ///     server_fn: server_fn::ServerFnTraitObj<()>,
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
        handler: impl Fn(server_fn::ServerFnTraitObj<()>) -> H,
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
        mut handler: impl FnMut(server_fn::ServerFnTraitObj<()>) -> H,
    ) -> Self
    where
        H: Handler + 'static,
    {
        let mut router = self;
        for server_fn_path in DioxusServerFnRegistry::paths_registered() {
            let func = DioxusServerFnRegistry::get(server_fn_path).unwrap();
            let full_route = format!("{server_fn_route}/{server_fn_path}");
            match func.encoding() {
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

        self.serve_static_assets(cfg.assets_path)
            .connect_hot_reload()
            .register_server_fns(server_fn_path)
            .push(Router::with_path("/<**any_path>").get(SSRHandler { cfg }))
    }

    fn connect_hot_reload(self) -> Self {
        let mut _dioxus_router = Router::with_path("_dioxus");
        _dioxus_router =
            _dioxus_router.push(Router::with_path("hot_reload").handle(HotReloadHandler));
        #[cfg(all(debug_assertions, feature = "hot-reload", feature = "ssr"))]
        {
            _dioxus_router = _dioxus_router.push(Router::with_path("disconnect").handle(ignore_ws));
        }
        self.push(_dioxus_router)
    }
}

/// Extracts the parts of a request that are needed for server functions. This will take parts of the request and replace them with empty values.
pub fn extract_parts(req: &mut Request) -> http::request::Parts {
    let mut parts = http::request::Request::new(()).into_parts().0;
    parts.method = std::mem::take(req.method_mut());
    parts.uri = std::mem::take(req.uri_mut());
    parts.version = req.version();
    parts.headers = std::mem::take(req.headers_mut());
    parts.extensions = std::mem::take(req.extensions_mut());

    parts
}

fn apply_request_parts_to_response(
    headers: hyper::header::HeaderMap,
    response: &mut salvo::prelude::Response,
) {
    let mut_headers = response.headers_mut();
    for (key, value) in headers.iter() {
        mut_headers.insert(key, value.clone());
    }
}

#[inline]
async fn convert_request(req: &mut Request) -> Result<HyperRequest, SalvoError> {
    let forward_url: hyper::Uri = TryFrom::try_from(req.uri()).map_err(SalvoError::other)?;
    let mut build = hyper::Request::builder()
        .method(req.method())
        .uri(&forward_url);
    for (key, value) in req.headers() {
        build = build.header(key, value);
    }
    static SECURE_MAX_SIZE: usize = 64 * 1024;

    let body = Limited::new(req.take_body(), SECURE_MAX_SIZE)
        .collect()
        .await
        .map_err(ParseError::other)?
        .to_bytes();
    build.body(body.into()).map_err(SalvoError::other)
}

#[inline]
async fn convert_response(response: HyperResponse, res: &mut Response) {
    let (parts, body) = response.into_parts();
    let http::response::Parts {
        version,
        headers,
        status,
        ..
    } = parts;
    res.status_code = Some(status);
    res.version = version;
    res.cookies = CookieJar::new();
    for cookie in headers.get_all(http::header::SET_COOKIE).iter() {
        if let Some(cookie) = cookie
            .to_str()
            .ok()
            .and_then(|s| Cookie::parse(s.to_string()).ok())
        {
            res.cookies.add_original(cookie);
        }
    }
    res.headers = headers;
    res.version = version;
    if let Ok(bytes) = hyper::body::to_bytes(body).await {
        res.body = bytes.into()
    }
}

/// A handler that renders a Dioxus application to HTML using server-side rendering.
pub struct SSRHandler<P: Clone> {
    cfg: ServeConfig<P>,
}

impl<P: Clone> SSRHandler<P> {
    /// Creates a new SSR handler with the given configuration.
    pub fn new(cfg: ServeConfig<P>) -> Self {
        Self { cfg }
    }
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
            let renderer = SSRState::new(&self.cfg);
            depot.inject(renderer.clone());
            renderer
        };

        let route = req.uri().path().to_string();
        let parts: Arc<RwLock<http::request::Parts>> = Arc::new(RwLock::new(extract_parts(req)));
        let server_context = DioxusServerContext::new(parts);

        match renderer_pool
            .render(route, &self.cfg, &server_context)
            .await
        {
            Ok(rendered) => {
                let crate::render::RenderResponse { html, freshness } = rendered;

                res.write_body(html).unwrap();

                let headers = server_context.response_parts().unwrap().headers.clone();
                apply_request_parts_to_response(headers, res);
                freshness.write(res.headers_mut());
            }
            Err(err) => {
                tracing::error!("Error rendering SSR: {}", err);
                res.write_body("Error rendering SSR").unwrap();
            }
        };
    }
}

/// A default handler for server functions. It will deserialize the request body, call the server function, and serialize the response.
pub struct ServerFnHandler {
    server_context: DioxusServerContext,
    function: server_fn::ServerFnTraitObj<()>,
}

impl ServerFnHandler {
    /// Create a new server function handler with the given server context and server function.
    pub fn new(
        server_context: impl Into<DioxusServerContext>,
        function: server_fn::ServerFnTraitObj<()>,
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
        match convert_request(req).await {
            Ok(hyper_req) => {
                let response =
                    server_fn_service(self.server_context.clone(), self.function.clone())
                        .run(hyper_req)
                        .await
                        .unwrap();
                convert_response(response, res).await;
            }
            Err(err) => handle_error(err, res),
        }
    }
}

fn handle_error(error: impl Error + Send + Sync, res: &mut Response) {
    let mut resp_err = Response::new();
    resp_err.status_code(StatusCode::INTERNAL_SERVER_ERROR);
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
        use salvo::websocket::Message;
        use salvo::websocket::WebSocketUpgrade;

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
    use salvo::websocket::WebSocketUpgrade;
    WebSocketUpgrade::new()
        .upgrade(req, res, |mut ws| async move {
            let _ = ws.send(salvo::websocket::Message::text("connected")).await;
            while let Some(msg) = ws.recv().await {
                if msg.is_err() {
                    return;
                };
            }
        })
        .await
}
