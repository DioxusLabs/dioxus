//! Dioxus utilities for the [Warp](https://docs.rs/warp/latest/warp/index.html) server framework.
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
//!         tokio::runtime::Runtime::new()
//!             .unwrap()
//!             .block_on(async move {
//!                 let routes = serve_dioxus_application("", ServeConfigBuilder::new(app, ()));
//!                 warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
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
//!
//! ```

use crate::layer::Service;
use crate::{
    prelude::*, render::SSRState, serve_config::ServeConfig, server_fn::DioxusServerFnRegistry,
};

use crate::server_fn_service;
use server_fn::{Encoding, Payload, ServerFunctionRegistry};
use std::error::Error;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::task::spawn_blocking;
use warp::path::FullPath;
use warp::Rejection;
use warp::{
    filters::BoxedFilter,
    http::{Response, StatusCode},
    hyper::body::Bytes,
    path, Filter, Reply,
};

/// Registers server functions with a custom handler function. This allows you to pass custom context to your server functions by generating a [`DioxusServerContext`] from the request.
///
/// # Example
/// ```rust
/// use warp::{body, header, hyper::HeaderMap, path, post, Filter};
///
/// #[tokio::main]
/// async fn main() {
///     let routes = register_server_fns_with_handler(server_fn_route, |full_route, func| {
///         path(full_route)
///         .and(warp::post().or(warp::get()).unify())
///         .and(request_parts())
///         .and(warp::body::bytes())
///         .and_then(move |parts, bytes: bytes::Bytes| {
///             let mut service = server_fn_service(DioxusServerContext::default(), func.clone());
///             async move {
///                 let req = warp::hyper::Request::from_parts(parts, bytes.into());
///                 service.run(req).await.map_err(|err| {
///                     tracing::error!("Server function error: {}", err);
///                     warp::reject::reject()
///                 })
///             }
///         })
/// })
///     warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
/// }
/// ```
pub fn register_server_fns_with_handler<H, F, R>(
    server_fn_route: &'static str,
    mut handler: H,
) -> BoxedFilter<(R,)>
where
    H: FnMut(String, server_fn::ServerFnTraitObj<()>) -> F,
    F: Filter<Extract = (R,), Error = warp::Rejection> + Send + Sync + 'static,
    F::Extract: Send,
    R: Reply + 'static,
{
    let mut filter: Option<BoxedFilter<F::Extract>> = None;
    for server_fn_path in DioxusServerFnRegistry::paths_registered() {
        let func = DioxusServerFnRegistry::get(server_fn_path).unwrap();
        let full_route = format!("{server_fn_route}/{server_fn_path}")
            .trim_start_matches('/')
            .to_string();
        let route = handler(full_route, func).boxed();
        if let Some(boxed_filter) = filter.take() {
            filter = Some(boxed_filter.or(route).unify().boxed());
        } else {
            filter = Some(route);
        }
    }
    filter.expect("No server functions found")
}

/// Registers server functions with the default handler. This handler function will pass an empty [`DioxusServerContext`] to your server functions.
///
/// # Example
/// ```rust
/// use dioxus_fullstack::prelude::*;
///
/// #[tokio::main]
/// async fn main() {
///     let routes = register_server_fns("");
///     warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
/// }
/// ```
pub fn register_server_fns(server_fn_route: &'static str) -> BoxedFilter<(impl Reply,)> {
    register_server_fns_with_handler(server_fn_route, |full_route, func| {
        path(full_route)
            .and(warp::post().or(warp::get()).unify())
            .and(request_parts())
            .and(warp::body::bytes())
            .and_then(move |parts, bytes: bytes::Bytes| {
                let mut service = server_fn_service(DioxusServerContext::default(), func.clone());
                async move {
                    let req = warp::hyper::Request::from_parts(parts, bytes.into());
                    service.run(req).await.map_err(|err| {
                        tracing::error!("Server function error: {}", err);
                      
                        struct WarpServerFnError(String);
                        impl std::fmt::Debug for WarpServerFnError {
                            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                                write!(f, "{}", self.0)
                            }
                        }

                        impl warp::reject::Reject for WarpServerFnError {}
                      
                        warp::reject::custom(WarpServerFnError(err.to_string()))
                    })
                }
            })
    })
}

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
///     let routes = serve_dioxus_application("", ServeConfigBuilder::new(app, ()));
///     warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
/// }
///
/// fn app(cx: Scope) -> Element {
///     todo!()
/// }
/// ```
pub fn serve_dioxus_application<P: Clone + serde::Serialize + Send + Sync + 'static>(
    server_fn_route: &'static str,
    cfg: impl Into<ServeConfig<P>>,
) -> BoxedFilter<(impl Reply,)> {
    let cfg = cfg.into();
    // Serve the dist folder and the index.html file
    let serve_dir = warp::fs::dir(cfg.assets_path);

    connect_hot_reload()
        // First register the server functions
        .or(register_server_fns(server_fn_route))
        // Then the index route
        .or(path::end().and(render_ssr(cfg.clone())))
        // Then the static assets
        .or(serve_dir)
        // Then all other routes
        .or(render_ssr(cfg))
        .boxed()
}

/// Server render the application.
pub fn render_ssr<P: Clone + serde::Serialize + Send + Sync + 'static>(
    cfg: ServeConfig<P>,
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::get()
        .and(request_parts())
        .and(with_ssr_state(&cfg))
        .then(move |parts: http::request::Parts, renderer: SSRState| {
            let route = parts.uri.path().to_string();
            let parts = Arc::new(RwLock::new(parts));
            let cfg = cfg.clone();
            async move {
                let server_context = DioxusServerContext::new(parts);

                match renderer.render(route, &cfg, &server_context).await {
                    Ok(rendered) => {
                        let crate::render::RenderResponse { html, freshness } = rendered;

                        let mut res = Response::builder()
                            .header("Content-Type", "text/html")
                            .body(html)
                            .unwrap();

                        let headers_mut = res.headers_mut();
                        let headers = server_context.response_parts().unwrap().headers.clone();
                        for (key, value) in headers.iter() {
                            headers_mut.insert(key, value.clone());
                        }
                        freshness.write(headers_mut);

                        res
                    }
                    Err(err) => {
                        tracing::error!("Failed to render ssr: {}", err);
                        Response::builder()
                            .status(500)
                            .body("Failed to render ssr".into())
                            .unwrap()
                    }
                }
            }
        })
}

/// An extractor for the request parts (used in [DioxusServerContext]). This will extract the method, uri, query, and headers from the request.
pub fn request_parts(
) -> impl Filter<Extract = (http::request::Parts,), Error = warp::reject::Rejection> + Clone {
    warp::method()
        .and(warp::filters::path::full())
        .and(
            warp::filters::query::raw()
                .or(warp::any().map(String::new))
                .unify(),
        )
        .and(warp::header::headers_cloned())
        .and_then(move |method, path: FullPath, query, headers| async move {
            http::uri::Builder::new()
                .path_and_query(format!("{}?{}", path.as_str(), query))
                .build()
                .map_err(|err| {
                    warp::reject::custom(FailedToReadBody(format!("Failed to build uri: {}", err)))
                })
                .map(|uri| {
                    let mut req = http::Request::builder()
                        .method(method)
                        .uri(uri)
                        .body(())
                        .unwrap();
                    req.headers_mut().extend(headers);
                    req.into_parts().0
                })
        })
}

fn with_ssr_state<P: Clone + serde::Serialize + Send + Sync + 'static>(
    cfg: &ServeConfig<P>,
) -> impl Filter<Extract = (SSRState,), Error = std::convert::Infallible> + Clone {
    let renderer = SSRState::new(cfg);
    warp::any().map(move || renderer.clone())
}

#[derive(Debug)]
struct FailedToReadBody(String);

impl warp::reject::Reject for FailedToReadBody {}

#[derive(Debug)]
struct RecieveFailed(String);

impl warp::reject::Reject for RecieveFailed {}

/// Register the web RSX hot reloading endpoint. This will enable hot reloading for your application in debug mode when you call [`dioxus_hot_reload::hot_reload_init`].
///
/// # Example
/// ```rust
/// #![allow(non_snake_case)]
/// use dioxus_fullstack::prelude::*;
///
/// #[tokio::main]
/// async fn main() {
///     let routes = connect_hot_reload();
///     warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
/// }
/// ```
pub fn connect_hot_reload() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone
{
    #[cfg(not(all(debug_assertions, feature = "hot-reload", feature = "ssr")))]
    {
        warp::path!("_dioxus" / "hot_reload")
            .map(warp::reply)
            .map(|reply| warp::reply::with_status(reply, warp::http::StatusCode::NOT_FOUND))
    }
    #[cfg(all(debug_assertions, feature = "hot-reload", feature = "ssr"))]
    {
        use crate::hot_reload::HotReloadState;
        use futures_util::sink::SinkExt;
        use futures_util::StreamExt;
        use warp::ws::Message;

        let hot_reload = warp::path!("_dioxus" / "hot_reload")
            .and(warp::any().then(crate::hot_reload::spawn_hot_reload))
            .and(warp::ws())
            .map(move |state: &'static HotReloadState, ws: warp::ws::Ws| {
                #[cfg(all(debug_assertions, feature = "hot-reload", feature = "ssr"))]
                ws.on_upgrade(move |mut websocket| {
                    async move {
                        println!("🔥 Hot Reload WebSocket connected");
                        {
                            // update any rsx calls that changed before the websocket connected.
                            {
                                println!("🔮 Finding updates since last compile...");
                                let templates_read = state.templates.read().await;

                                for template in &*templates_read {
                                    if websocket
                                        .send(Message::text(
                                            serde_json::to_string(&template).unwrap(),
                                        ))
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
                    }
                })
            });
        let disconnect =
            warp::path!("_dioxus" / "disconnect")
                .and(warp::ws())
                .map(move |ws: warp::ws::Ws| {
                    println!("disconnect");
                    #[cfg(all(debug_assertions, feature = "hot-reload", feature = "ssr"))]
                    ws.on_upgrade(move |mut websocket| async move {
                        struct DisconnectOnDrop(Option<warp::ws::WebSocket>);
                        impl Drop for DisconnectOnDrop {
                            fn drop(&mut self) {
                                std::mem::drop(self.0.take().unwrap().close());
                            }
                        }

                        let _ = websocket.send(Message::text("connected")).await;
                        let mut ws = DisconnectOnDrop(Some(websocket));

                        loop {
                            if ws.0.as_mut().unwrap().next().await.is_none() {
                                break;
                            }
                        }
                    })
                });
        disconnect.or(hot_reload)
    }
}
