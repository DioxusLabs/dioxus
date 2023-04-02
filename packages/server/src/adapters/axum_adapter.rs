use std::{error::Error, sync::Arc};

use axum::{
    body::{self, Body, BoxBody, Full},
    extract::{State, WebSocketUpgrade},
    handler::Handler,
    http::{HeaderMap, Request, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use server_fn::{Payload, ServerFunctionRegistry};
use tokio::task::spawn_blocking;

use crate::{
    render::SSRState,
    serve::ServeConfig,
    server_context::DioxusServerContext,
    server_fn::{DioxusServerFnRegistry, ServerFnTraitObj},
};

pub trait DioxusRouterExt<S> {
    fn register_server_fns_with_handler<H, T>(
        self,
        server_fn_route: &'static str,
        handler: impl Fn(Arc<ServerFnTraitObj>) -> H,
    ) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
        S: Clone + Send + Sync + 'static;
    fn register_server_fns(self, server_fn_route: &'static str) -> Self;

    fn serve_dioxus_application<P: Clone + Send + Sync + 'static>(
        self,
        server_fn_route: &'static str,
        cfg: impl Into<ServeConfig<P>>,
    ) -> Self;

    fn connect_hot_reload(self) -> Self;
}

impl<S> DioxusRouterExt<S> for Router<S>
where
    S: Send + Sync + Clone + 'static,
{
    fn register_server_fns_with_handler<H, T>(
        self,
        server_fn_route: &'static str,
        mut handler: impl FnMut(Arc<ServerFnTraitObj>) -> H,
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
            router = router.route(&full_route, post(handler(func)));
        }
        router
    }

    fn register_server_fns(self, server_fn_route: &'static str) -> Self {
        self.register_server_fns_with_handler(server_fn_route, |func| {
            move |headers: HeaderMap, body: Request<Body>| async move {
                server_fn_handler(DioxusServerContext::default(), func.clone(), headers, body).await
            }
        })
    }

    fn serve_dioxus_application<P: Clone + Send + Sync + 'static>(
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

async fn render_handler<P: Clone + Send + Sync + 'static>(
    State((cfg, ssr_state)): State<(ServeConfig<P>, SSRState)>,
) -> impl IntoResponse {
    let rendered = ssr_state.render(&cfg);
    Full::from(rendered)
}

pub async fn server_fn_handler(
    server_context: DioxusServerContext,
    function: Arc<ServerFnTraitObj>,
    headers: HeaderMap,
    req: Request<Body>,
) -> impl IntoResponse {
    let (_, body) = req.into_parts();
    let body = hyper::body::to_bytes(body).await;
    let Ok(body)=body else {
        return report_err(body.err().unwrap());
    };

    // Because the future returned by `server_fn_handler` is `Send`, and the future returned by this function must be send, we need to spawn a new runtime
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
    spawn_blocking({
        move || {
            tokio::runtime::Runtime::new()
                .expect("couldn't spawn runtime")
                .block_on(async {
                    let resp = match function(server_context, &body).await {
                        Ok(serialized) => {
                            // if this is Accept: application/json then send a serialized JSON response
                            let accept_header =
                                headers.get("Accept").and_then(|value| value.to_str().ok());
                            let mut res = Response::builder();
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
