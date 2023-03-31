use std::{error::Error, sync::Arc};

use hyper::{http::HeaderValue, StatusCode};
use salvo::{
    async_trait, handler, serve_static::StaticDir, Depot, FlowCtrl, Handler, Request, Response,
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
            router = router.push(Router::with_path(&full_route).post(ServerFnHandler {
                server_context: DioxusServerContext {},
                function: func,
            }));
        }
        router
    }

    fn serve_dioxus_application<P: Clone + Send + Sync + 'static>(
        self,
        cfg: ServeConfig<P>,
    ) -> Self {
        // Serve the dist folder and the index.html file
        let serve_dir = StaticDir::new(["dist"]);

        self.register_server_fns(cfg.server_fn_route.unwrap_or_default())
            .push(Router::with_path("/").get(SSRHandler { cfg }))
            .push(Router::with_path("<**path>").get(serve_dir))
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
        _depot: &mut Depot,
        res: &mut Response,
        _flow: &mut FlowCtrl,
    ) {
        res.write_body(dioxus_ssr_html(&self.cfg)).unwrap();
    }
}

struct ServerFnHandler {
    server_context: DioxusServerContext,
    function: Arc<ServerFnTraitObj>,
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
                        res.render(data);
                    }
                    Payload::Json(data) => {
                        res.headers_mut()
                            .insert("Content-Type", HeaderValue::from_static("application/json"));
                        res.render(data);
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
