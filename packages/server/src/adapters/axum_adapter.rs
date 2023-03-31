use std::{error::Error, sync::Arc};

use axum::{
    body::{self, Body, BoxBody, Full},
    http::{HeaderMap, Request, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use server_fn::{Payload, ServerFunctionRegistry};
use tokio::task::spawn_blocking;

use crate::{
    dioxus_ssr_html,
    serve::ServeConfig,
    server_fn::{DioxusServerContext, DioxusServerFnRegistry, ServerFnTraitObj},
};

pub trait DioxusRouterExt {
    fn register_server_fns(self, server_fn_route: &'static str) -> Self;
    fn serve_dioxus_application<P: Clone + Send + Sync + 'static>(
        self,
        cfg: ServeConfig<P>,
    ) -> Self;
}

impl DioxusRouterExt for Router {
    fn register_server_fns(self, server_fn_route: &'static str) -> Self {
        let mut router = self;
        for server_fn_path in DioxusServerFnRegistry::paths_registered() {
            let func = DioxusServerFnRegistry::get(server_fn_path).unwrap();
            let full_route = format!("{server_fn_route}/{server_fn_path}");
            router = router.route(
                &full_route,
                post(move |headers: HeaderMap, body: Request<Body>| async move {
                    server_fn_handler(DioxusServerContext {}, func.clone(), headers, body).await
                }),
            );
        }
        router
    }

    fn serve_dioxus_application<P: Clone + Send + Sync + 'static>(
        self,
        cfg: ServeConfig<P>,
    ) -> Self {
        use tower_http::services::ServeDir;

        // Serve the dist folder and the index.html file
        let serve_dir = ServeDir::new("dist");

        self.register_server_fns(cfg.server_fn_route.unwrap_or_default())
            .route(
                "/",
                get(move || {
                    let rendered = dioxus_ssr_html(&cfg);
                    async move { Full::from(rendered) }
                }),
            )
            .fallback_service(serve_dir)
    }
}

async fn server_fn_handler(
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
