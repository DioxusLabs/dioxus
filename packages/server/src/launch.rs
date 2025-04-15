//! A launch function that creates an axum router for the LaunchBuilder

use std::{any::Any, collections::HashMap, net::SocketAddr};

use axum::{
    body::Body,
    extract::{Request, State},
    response::IntoResponse,
    routing::IntoMakeService,
    serve::IncomingStream,
};
use dashmap::DashMap;
use dioxus_cli_config::base_path;
use dioxus_devtools::DevserverMsg;
use dioxus_lib::prelude::*;
use futures_util::{pin_mut, stream::FusedStream, StreamExt};
use hyper::body::Incoming;
use hyper_util::server::conn::auto::Builder as HyperBuilder;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    service::TowerToHyperService,
};
use server_fn::ServerFnTraitObj;
use tokio::net::TcpStream;
use tokio_util::task::LocalPoolHandle;
use tower::Service;
use tower::ServiceExt as _;
// use tower::{Service, ServiceExt};

use crate::{
    register_server_fn_on_router, render_handler, rt::DioxusRouterExt, RenderHandleState, SSRState,
    ServeConfig, ServeConfigBuilder,
};

type ContextList = Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>;

type BaseComp = fn() -> Element;

/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
pub fn launch(root: BaseComp, contexts: ContextList, platform_config: Vec<Box<dyn Any>>) -> ! {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            serve_server(root, contexts, platform_config).await;
        });

    unreachable!("Launching a fullstack app should never return")
}

async fn serve_server(
    root: fn() -> Result<VNode, RenderError>,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Vec<Box<dyn Any>>,
) {
    let (devtools_tx, mut devtools_rx) = futures_channel::mpsc::unbounded();

    if let Some(endpoint) = dioxus_cli_config::devserver_ws_endpoint() {
        dioxus_devtools::connect(endpoint, move |msg| {
            _ = devtools_tx.unbounded_send(msg);
        })
    }

    let platform_config = platform_config
        .into_iter()
        .find_map(|cfg| {
            cfg.downcast::<ServeConfig>()
                .map(|cfg| Result::Ok(*cfg))
                .or_else(|cfg| {
                    cfg.downcast::<ServeConfigBuilder>()
                        .map(|builder| builder.build())
                })
                .ok()
        })
        .unwrap_or_else(ServeConfig::new);

    // Extend the config's context providers with the context providers from the launch builder
    let cfg = platform_config
        .map(|mut cfg| {
            let mut contexts = contexts;
            let cfg_context_providers = cfg.context_providers.clone();
            for i in 0..cfg_context_providers.len() {
                contexts.push(Box::new({
                    let cfg_context_providers = cfg_context_providers.clone();
                    move || (cfg_context_providers[i])()
                }));
            }
            cfg.context_providers = std::sync::Arc::new(contexts);
            cfg
        })
        .unwrap();

    // Get the address the server should run on. If the CLI is running, the CLI proxies fullstack into the main address
    // and we use the generated address the CLI gives us
    let address = dioxus_cli_config::fullstack_address_or_localhost();

    let router = axum::Router::new().serve_dioxus_application(cfg.clone(), root);

    let task_pool = LocalPoolHandle::new(5);
    let mut make_service = router.into_make_service();

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    tracing::info!("Listening on {address} with listener {listener:?}");

    enum Msg {
        TcpStream(std::io::Result<(TcpStream, SocketAddr)>),
        Devtools(DevserverMsg),
    }

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(0);
    let mut hr_idx = 0;

    // Manually loop on accepting connections so we can also respond to devtools messages
    loop {
        let res = tokio::select! {
            res = listener.accept() => Msg::TcpStream(res),
            msg = devtools_rx.next(), if !devtools_rx.is_terminated() => {
                if let Some(msg) = msg {
                    Msg::Devtools(msg)
                } else {
                    continue;
                }
            }
        };

        match res {
            // We need to delete our old router and build a new one
            //
            // one challenge is that the server functions are sitting in the dlopened lib and no longer
            // accessible by us (the original process)
            //
            // We need to somehow get them out... ?
            //
            // for now we just support editing existing server functions
            Msg::Devtools(devserver_msg) => {
                match devserver_msg {
                    DevserverMsg::HotReload(hot_reload_msg) => {
                        if let Some(table) = hot_reload_msg.jump_table {
                            use axum::body::Body;
                            use http::{Method, Request, Response, StatusCode};

                            if let Ok(_) = dioxus_devtools::apply_patch(table) {
                                let mut new_router = axum::Router::new().serve_static_assets();

                                let server_fn_iter = collect_raw_server_fns();

                                // de-duplicate iteratively by prefering the most recent (first, since it's linked)
                                let mut server_fn_map: HashMap<_, _> = HashMap::new();
                                for f in server_fn_iter.into_iter().rev() {
                                    server_fn_map.insert(f.path(), f);
                                }

                                for (_, f) in server_fn_map {
                                    tracing::info!(
                                        "Registering server function: {:?} {:?}",
                                        f.path(),
                                        f.method()
                                    );
                                    new_router = crate::register_server_fn_on_router(
                                        f,
                                        new_router,
                                        cfg.context_providers.clone(),
                                    );
                                }

                                let ssr_state = SSRState::new(&cfg);

                                make_service = new_router
                                    .fallback(
                                        axum::routing::get(render_handler).with_state(
                                            RenderHandleState::new(cfg.clone(), root)
                                                .with_ssr_state(ssr_state),
                                        ),
                                    )
                                    .into_make_service();

                                tracing::info!("Shutting down connections...");
                                _ = shutdown_tx.send_modify(|i| {
                                    *i += 1;
                                    hr_idx += 1;
                                });
                            }
                        }
                    }
                    DevserverMsg::FullReloadStart => {}
                    DevserverMsg::FullReloadFailed => {}
                    DevserverMsg::FullReloadCommand => {}
                    DevserverMsg::Shutdown => {}
                }
            }
            Msg::TcpStream(Err(_)) => {}
            Msg::TcpStream(Ok((tcp_stream, remote_addr))) => {
                tracing::debug!("Accepted connection from {remote_addr}");

                let mut make_service = make_service.clone();
                let mut shutdown_rx = shutdown_rx.clone();
                let mut hr_idx = hr_idx.clone();
                task_pool.spawn_pinned(move || async move {
                    let tcp_stream = TokioIo::new(tcp_stream);

                    std::future::poll_fn(|cx| {
                        <IntoMakeService<axum::Router> as tower::Service<Request>>::poll_ready(
                            &mut make_service,
                            cx,
                        )
                    })
                    .await
                    .unwrap_or_else(|err| match err {});

                    // todo - this was taken from axum::serve but it seems like IncomingStream serves no purpose?
                    #[derive(Debug)]
                    pub struct IncomingStream_<'a> {
                        tcp_stream: &'a TokioIo<TcpStream>,
                        remote_addr: SocketAddr,
                    }

                    let tower_service = make_service
                        .call(IncomingStream_ {
                            tcp_stream: &tcp_stream,
                            remote_addr,
                        })
                        .await
                        .unwrap_or_else(|err| match err {})
                        .map_request(|req: Request<Incoming>| {
                            let req = req.map(Body::new);

                            tracing::info!("Handling request: {:?}", req);

                            req
                        });

                    // upgrades needed for websockets
                    let builder = HyperBuilder::new(TokioExecutor::new());
                    let connection = builder.serve_connection_with_upgrades(
                        tcp_stream,
                        TowerToHyperService::new(tower_service),
                    );

                    tokio::select! {
                        res = connection => {
                            if let Err(err) = res {
                                // This error only appears when the client doesn't send a request and
                                // terminate the connection.
                                //
                                // If client sends one request then terminate connection whenever, it doesn't
                                // appear.
                            }
                        }
                        res = shutdown_rx.wait_for(|i| *i == hr_idx + 1) => {
                            tracing::info!("Shutting down connection server: {res:?}");
                            return;
                        }
                    }
                });
            }
        }
    }
}

pub type AxumServerFn = ServerFnTraitObj<http::Request<Body>, http::Response<Body>>;

pub fn collect_raw_server_fns() -> Vec<&'static AxumServerFn> {
    inventory::iter::<AxumServerFn>().into_iter().collect()
}

fn build_router(
    root: fn() -> Result<VNode, RenderError>,
    platform_config: Result<ServeConfig, crate::UnableToLoadIndex>,
) -> axum::Router {
    let mut base_path = base_path();

    let dioxus_router =
        axum::Router::new().serve_dioxus_application(platform_config.unwrap(), root);

    let router = dioxus_router;

    // let mut router;
    // match base_path.as_deref() {
    //     Some(base_path) => {
    //         let base_path = base_path.trim_matches('/');
    //         // If there is a base path, nest the router under it and serve the root route manually
    //         // Nesting a route in axum only serves /base_path or /base_path/ not both
    //         router = axum::Router::new().nest(&format!("/{base_path}/"), dioxus_router);
    //         async fn root_render_handler(
    //             state: State<RenderHandleState>,
    //             mut request: Request<Body>,
    //         ) -> impl IntoResponse {
    //             // The root of the base path always looks like the root from dioxus fullstack
    //             *request.uri_mut() = "/".parse().unwrap();
    //             render_handler(state, request).await
    //         }
    //         if let Some(cfg) = config {
    //             let ssr_state = SSRState::new(&cfg);
    //             router = router.route(
    //                 &format!("/{base_path}"),
    //                 axum::routing::method_routing::get(root_render_handler).with_state(
    //                     RenderHandleState::new(cfg, root).with_ssr_state(ssr_state),
    //                 ),
    //             )
    //         }
    //     }
    //     None => router = dioxus_router,
    // }
    router
}
