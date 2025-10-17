//! A launch function that creates an axum router for the LaunchBuilder

use crate::{server::DioxusRouterExt, RenderHandleState, ServeConfig};
use anyhow::Context;
use axum::{
    body::Body,
    extract::{Request, State},
    routing::IntoMakeService,
    Router,
};
use dioxus_cli_config::base_path;
use dioxus_core::Element;
#[cfg(not(target_arch = "wasm32"))]
use dioxus_core::{RenderError, VNode};
use dioxus_devtools::DevserverMsg;
use futures_util::{stream::FusedStream, StreamExt};
use hyper::body::Incoming;
use hyper_util::server::conn::auto::Builder as HyperBuilder;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    service::TowerToHyperService,
};
use std::sync::Arc;
use std::{any::Any, collections::HashMap, net::SocketAddr, prelude::rust_2024::Future};
use tokio::net::TcpStream;
use tokio_util::task::LocalPoolHandle;
use tower::{Service, ServiceExt as _};

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
    let (devtools_tx, mut devtools_rx) = futures_channel::mpsc::unbounded();

    dioxus_devtools::connect(move |msg| _ = devtools_tx.unbounded_send(msg));

    let mut cfg = platform_config
        .into_iter()
        .find_map(|cfg| cfg.downcast::<ServeConfig>().ok().map(|b| *b))
        .unwrap_or_else(ServeConfig::new);

    // Extend the config's context providers with the context providers from the launch builder
    for ctx in contexts {
        let arced = Arc::new(ctx) as Arc<dyn Fn() -> Box<dyn Any> + Send + Sync>;
        cfg.context_providers.push(arced);
    }

    // Get the address the server should run on. If the CLI is running, the CLI proxies fullstack into the main address
    // and we use the generated address the CLI gives us
    let address = dioxus_cli_config::fullstack_address_or_localhost();

    // Create the router and register the server functions under the basepath.
    let router = apply_base_path(
        Router::new().serve_dioxus_application(cfg.clone(), original_root),
        original_root,
        cfg.clone(),
        base_path().map(|s| s.to_string()),
    );

    let task_pool = LocalPoolHandle::new(
        std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1),
    );

    let mut make_service = router.into_make_service();

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

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
            Msg::TcpStream(Ok((tcp_stream, _remote_addr))) => {
                let this_hr_index = hr_idx;
                let mut make_service = make_service.clone();
                let mut shutdown_rx = shutdown_rx.clone();

                task_pool.spawn_pinned(move || async move {
                    let tcp_stream = TokioIo::new(tcp_stream);

                    std::future::poll_fn(|cx| {
                        <IntoMakeService<Router> as tower::Service<Request>>::poll_ready(
                            &mut make_service,
                            cx,
                        )
                    })
                    .await
                    .unwrap();

                    let tower_service = make_service
                        .call(())
                        .await
                        .unwrap()
                        .map_request(|req: Request<Incoming>| req.map(Body::new));

                    // upgrades needed for websockets
                    let builder = HyperBuilder::new(TokioExecutor::new());
                    let connection = builder.serve_connection_with_upgrades(
                        tcp_stream,
                        TowerToHyperService::new(tower_service),
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
                        _res = shutdown_rx.wait_for(|i| *i == this_hr_index + 1) => {}
                    }
                });
            }
            Msg::TcpStream(Err(_)) => {}
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
                        if hot_reload_msg.for_build_id == Some(dioxus_cli_config::build_id()) {
                            if let Some(table) = hot_reload_msg.jump_table {
                                use crate::ServerFunction;

                                unsafe { dioxus_devtools::subsecond::apply_patch(table).unwrap() };

                                let mut new_router = Router::new().serve_static_assets();
                                let new_cfg = ServeConfig::new();

                                let server_fn_iter = ServerFunction::collect();

                                // de-duplicate iteratively by preferring the most recent (first, since it's linked)
                                let mut server_fn_map: HashMap<_, _> = HashMap::new();
                                for f in server_fn_iter.into_iter().rev() {
                                    server_fn_map.insert(f.path(), f);
                                }

                                for (_, fn_) in server_fn_map {
                                    tracing::trace!(
                                        "Registering server function: {:?} {:?}",
                                        fn_.path(),
                                        fn_.method()
                                    );
                                    new_router = fn_.register_server_fn_on_router(new_router);
                                }

                                let hot_root = subsecond::HotFn::current(original_root);
                                let new_root_addr = hot_root.ptr_address().0 as usize as *const ();
                                let new_root = unsafe {
                                    std::mem::transmute::<*const (), fn() -> Element>(new_root_addr)
                                };

                                crate::document::reset_renderer();

                                let state = RenderHandleState::new(new_cfg.clone(), new_root);

                                let fallback_handler =
                                    axum::routing::get(RenderHandleState::render_handler)
                                        .with_state(state);

                                make_service = apply_base_path(
                                    new_router.fallback(fallback_handler),
                                    new_root,
                                    new_cfg.clone(),
                                    base_path().map(|s| s.to_string()),
                                )
                                .into_make_service();

                                shutdown_tx.send_modify(|i| {
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
                    _ => {}
                }
            }
        }
    }
}

fn apply_base_path(
    mut router: Router,
    root: fn() -> Element,
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
                |state: State<RenderHandleState>, mut request: Request<Body>| async move {
                    // The root of the base path always looks like the root from dioxus fullstack
                    *request.uri_mut() = "/".parse().unwrap();
                    RenderHandleState::render_handler(state, request).await
                },
            )
            .with_state(RenderHandleState::new(cfg, root)),
        )
    }

    router
}

/// Serve a fullstack dioxus application with a custom axum router.
///
/// This function sets up an async runtime, enables the default dioxus logger, runs the provided initializer,
/// and then starts an axum server with the returned router.
///
/// The axum router will be bound to the address specified by the `IP` and `PORT` environment variables,
/// defaulting to `127.0.0.1:8080` if not set.
pub fn serve<F>(mut serve_it: impl FnMut() -> F) -> !
where
    F: Future<Output = Result<Router, anyhow::Error>>,
{
    dioxus_logger::initialize_default();

    let app_future = async move {
        let router = serve_it().await.expect("Failed to create axum router");
        let address = dioxus_cli_config::fullstack_address_or_localhost();
        let listener = tokio::net::TcpListener::bind(address)
            .await
            .with_context(|| format!("Failed to bind app to given address: {address}"))
            .unwrap();
        tracing::trace!("Listening on {address}");
        axum::serve::serve(listener, router)
            .await
            .expect("Failed to serve axum app");
    };

    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.block_on(app_future);
    } else {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(app_future);
    }

    unreachable!("Serving a fullstack app should never return")
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
