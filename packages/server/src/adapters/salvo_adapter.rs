use std::{error::Error, sync::Arc};

use hyper::{http::HeaderValue, StatusCode};
use salvo::{
    async_trait, handler, serve_static::StaticDir, Depot, FlowCtrl, Handler, Request, Response,
    Router,
};
use server_fn::{Payload, ServerFunctionRegistry};
use tokio::task::spawn_blocking;

use crate::{
    prelude::DioxusServerContext,
    prelude::SSRState,
    serve::ServeConfig,
    server_fn::{DioxusServerFnRegistry, ServerFnTraitObj},
};

pub trait DioxusRouterExt {
    fn register_server_fns(self, server_fn_route: &'static str) -> Self;
    fn register_server_fns_with_handler<H>(
        self,
        server_fn_route: &'static str,
        handler: impl Fn(Arc<ServerFnTraitObj>) -> H,
    ) -> Self
    where
        H: Handler + 'static;
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
            let serve_dir = StaticDir::new([path.clone()]);
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
            let route = format!("/{}/<**path>", route);
            self = self.push(Router::with_path(route).get(serve_dir))
        }

        self.register_server_fns(server_fn_route)
            .push(Router::with_path("/").get(SSRHandler { cfg }))
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

pub struct ServerFnHandler {
    server_context: DioxusServerContext,
    function: Arc<ServerFnTraitObj>,
}

impl ServerFnHandler {
    pub fn new(server_context: DioxusServerContext, function: Arc<ServerFnTraitObj>) -> Self {
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
