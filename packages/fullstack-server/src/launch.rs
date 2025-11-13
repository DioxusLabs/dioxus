//! A launch function that creates an axum router for the LaunchBuilder

use crate::{server::DioxusRouterExt, FullstackState, ServeConfig};
use anyhow::Context;
use axum::{
    body::Body,
    extract::{Request, State},
    routing::IntoMakeService,
    Router,
};
use dioxus_cli_config::base_path;
use dioxus_core::{ComponentFunction, Element};

use dioxus_devtools::{DevserverMsg, HotReloadMsg};
use futures_util::{stream::FusedStream, StreamExt};
use hyper::body::Incoming;
use hyper_util::server::conn::auto::Builder as HyperBuilder;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    service::TowerToHyperService,
};
use std::{any::Any, net::SocketAddr, prelude::rust_2024::Future};
use std::{pin::Pin, sync::Arc};
use subsecond::HotFn;
use tokio_util::either::Either;
use tower::{Service, ServiceExt as _};

#[cfg(not(target_arch = "wasm32"))]
use {
    dioxus_core::{RenderError, VNode},
    tokio::net::TcpListener,
};

type ContextList = Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>;
type BaseComp = fn() -> Element;

/// Launch a fullstack app with the given root component.
pub fn launch(root: BaseComp) -> ! {
    launch_cfg(root, vec![], vec![])
}

/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
pub fn launch_cfg(root: BaseComp, contexts: ContextList, platform_config: Vec<Box<dyn Any>>) -> ! {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move { serve_server(root, contexts, platform_config).await });

    unreachable!("Launching a fullstack app should never return")
}

#[cfg(not(target_arch = "wasm32"))]
async fn serve_server(
    original_root: fn() -> Result<VNode, RenderError>,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Vec<Box<dyn Any>>,
) {
    let mut cfg = platform_config
        .into_iter()
        .find_map(|cfg| cfg.downcast::<ServeConfig>().ok().map(|b| *b))
        .unwrap_or_else(ServeConfig::new);

    // Extend the config's context providers with the context providers from the launch builder
    for ctx in contexts {
        let arced = Arc::new(ctx) as Arc<dyn Fn() -> Box<dyn Any> + Send + Sync>;
        cfg.context_providers.push(arced);
    }

    let cb = move || {
        let cfg = cfg.clone();
        Box::pin(async move {
            Ok(apply_base_path(
                Router::new().serve_dioxus_application(cfg.clone(), original_root),
                original_root,
                cfg.clone(),
                base_path().map(|s| s.to_string()),
            ))
        }) as _
    };

    serve_router(cb, dioxus_cli_config::fullstack_address_or_localhost()).await;
}

/// Create a router that serves the dioxus application at the appropriate base path.
///
/// This method automatically setups up:
/// - Static asset serving
/// - Mapping of base paths
/// - Automatic registration of server functions
/// - Handler to render the dioxus application
/// - WebSocket handling for live reload and devtools
/// - Hot-reloading
/// - Async Runtime
/// - Logging
pub fn router(app: fn() -> Element) -> Router {
    let cfg = ServeConfig::new();
    apply_base_path(
        Router::new().serve_dioxus_application(cfg.clone(), app),
        app,
        cfg,
        base_path().map(|s| s.to_string()),
    )
}

/// Serve a fullstack dioxus application with a custom axum router.
///
/// This function sets up an async runtime, enables the default dioxus logger, runs the provided initializer,
/// and then starts an axum server with the returned router.
///
/// The axum router will be bound to the address specified by the `IP` and `PORT` environment variables,
/// defaulting to `127.0.0.1:8080` if not set.
///
/// This function uses axum to block on serving the application, and will not return.
pub fn serve<F>(mut serve_it: impl FnMut() -> F) -> !
where
    F: Future<Output = Result<Router, anyhow::Error>> + 'static,
{
    let cb = move || Box::pin(serve_it()) as _;

    block_on(
        async move { serve_router(cb, dioxus_cli_config::fullstack_address_or_localhost()).await },
    );

    unreachable!("Serving a fullstack app should never return")
}

/// Serve a fullstack dioxus application with a custom axum router.
///
/// This function enables the dioxus logger and then serves the axum server with hot-reloading support.
///
/// To enable hot-reloading of the router, the provided `serve_callback` should return a new `Router`
/// each time it is called.
pub async fn serve_router(
    mut serve_callback: impl FnMut() -> Pin<Box<dyn Future<Output = Result<Router, anyhow::Error>>>>,
    addr: SocketAddr,
) {
    dioxus_logger::initialize_default();

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind to address {addr}"))
        .unwrap();

    // If we're not in debug mode, just serve the app normally
    if !cfg!(debug_assertions) {
        axum::serve(listener, serve_callback().await.unwrap())
            .await
            .unwrap();
        return;
    }

    // Wire up the devtools connection. The sender only sends messages in dev.
    let (devtools_tx, mut devtools_rx) = futures_channel::mpsc::unbounded();
    dioxus_devtools::connect(move |msg| _ = devtools_tx.unbounded_send(msg));

    let mut hot_serve_callback = HotFn::current(serve_callback);
    let mut make_service = hot_serve_callback
        .call(())
        .await
        .map(|router| router.into_make_service())
        .unwrap();

    let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
    let our_build_id = Some(dioxus_cli_config::build_id());

    // Manually loop on accepting connections so we can also respond to devtools messages
    loop {
        let res = tokio::select! {
            res = listener.accept() => Either::Left(res),
            Some(msg) = devtools_rx.next(), if !devtools_rx.is_terminated() => Either::Right(msg),
            else => continue
        };

        match res {
            Either::Left(Ok((tcp_stream, _remote_addr))) => {
                let mut make_service = make_service.clone();
                let mut shutdown_rx = shutdown_tx.subscribe();

                tokio::task::spawn(async move {
                    let tcp_stream = TokioIo::new(tcp_stream);

                    std::future::poll_fn(|cx| {
                        <IntoMakeService<Router> as tower::Service<Request>>::poll_ready(
                            &mut make_service,
                            cx,
                        )
                    })
                    .await
                    .expect("Infallible");

                    // upgrades needed for websockets
                    let builder = HyperBuilder::new(TokioExecutor::new());
                    let connection = builder.serve_connection_with_upgrades(
                        tcp_stream,
                        TowerToHyperService::new(
                            make_service
                                .call(())
                                .await
                                .unwrap()
                                .map_request(|req: Request<Incoming>| req.map(Body::new)),
                        ),
                    );

                    tokio::select! {
                        res = connection => {
                            if let Err(_err) = res {
                                // This error only appears when the client doesn't send a request and
                                // terminate the connection.
                                //
                                // If client sends one request then terminate connection whenever, it doesn't
                                // appear.
                            }
                        }
                        _res = shutdown_rx.recv() => {}
                    }
                });
            }

            // Handle just hot-patches for now.
            // We don't do RSX hot-reload since usually the client handles that once the page is loaded.
            //
            // todo(jon): I *believe* SSR is resilient to RSX changes, but we should verify that...
            Either::Right(DevserverMsg::HotReload(HotReloadMsg {
                jump_table: Some(table),
                for_build_id,
                ..
            })) if for_build_id == our_build_id => {
                // Apply the hot-reload patch to the dioxus devtools first
                unsafe { dioxus_devtools::subsecond::apply_patch(table).unwrap() };

                // Now recreate the router
                // We panic here because we don't want their app to continue in a maybe-corrupted state
                make_service = hot_serve_callback
                    .call(())
                    .await
                    .expect("Failed to create new router after hot-patch!")
                    .into_make_service();

                // Make sure to wipe out the renderer state so we don't have stale elements
                crate::document::reset_renderer();

                _ = shutdown_tx.send(());
            }

            // Explicitly don't handle RSX hot-reloads on the server
            // The client will handle that once the page is loaded. If we handled it here,
            _ => {}
        }
    }
}

fn block_on<T>(app_future: impl Future<Output = T>) {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.block_on(app_future);
    } else {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(app_future);
    }
}

fn apply_base_path<M: 'static>(
    mut router: Router,
    root: impl ComponentFunction<(), M> + Send + Sync,
    cfg: ServeConfig,
    base_path: Option<String>,
) -> Router {
    if let Some(base_path) = base_path {
        let base_path = base_path.trim_matches('/');

        // If there is a base path, nest the router under it and serve the root route manually
        // Nesting a route in axum only serves /base_path or /base_path/ not both
        router = Router::new().nest(&format!("/{base_path}/"), router).route(
            &format!("/{base_path}"),
            axum::routing::method_routing::get(
                |state: State<FullstackState>, mut request: Request<Body>| async move {
                    // The root of the base path always looks like the root from dioxus fullstack
                    *request.uri_mut() = "/".parse().unwrap();
                    FullstackState::render_handler(state, request).await
                },
            )
            .with_state(FullstackState::new(cfg, root)),
        )
    }

    router
}
