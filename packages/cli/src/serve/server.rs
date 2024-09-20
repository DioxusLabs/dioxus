use crate::{
    config::WebHttpsConfig,
    serve::{ServeArgs, ServeUpdate},
    BuildStage, BuildUpdate, DioxusCrate, Error, Platform, Result, TraceSrc,
};
use anyhow::Context;
use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    extract::{Request, State},
    http::{
        header::{HeaderName, HeaderValue, CACHE_CONTROL, EXPIRES, PRAGMA},
        Method, Response, StatusCode,
    },
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, get_service},
    Extension, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use dioxus_devtools_types::{DevserverMsg, HotReloadMsg};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{
    future,
    stream::{self, FuturesUnordered},
    StreamExt,
};
use hyper::{header::ACCEPT, HeaderMap};
use serde::{Deserialize, Serialize};
use std::{
    convert::Infallible,
    fs, io,
    net::{IpAddr, SocketAddr, TcpListener},
    sync::RwLock,
};
use std::{path::Path, sync::Arc};
use tower_http::{
    cors::Any,
    services::fs::{ServeDir, ServeFileSystemResponseBody},
    ServiceBuilderExt,
};

pub(crate) struct DevServer {
    devserver_ip: IpAddr,
    devserver_port: u16,
    proxied_port: Option<u16>,
    hot_reload_sockets: Vec<WebSocket>,
    build_status_sockets: Vec<WebSocket>,
    new_hot_reload_sockets: UnboundedReceiver<WebSocket>,
    new_build_status_sockets: UnboundedReceiver<WebSocket>,
    build_status: SharedStatus,
    application_name: String,
    platform: String,
}

impl DevServer {
    /// Start the development server.
    /// This will set up the default http server if there's no server specified (usually via fullstack).
    ///
    /// This will also start the websocket server that powers the devtools. If you want to communicate
    /// with connected devtools clients, this is the place to do it.
    pub(crate) fn start(args: &ServeArgs, cfg: &DioxusCrate) -> Result<Self> {
        let (hot_reload_sockets_tx, hot_reload_sockets_rx) = futures_channel::mpsc::unbounded();
        let (build_status_sockets_tx, build_status_sockets_rx) = futures_channel::mpsc::unbounded();

        let devserver_ip = args.address.addr;
        let devserver_port = args.address.port;
        let devserver_address = SocketAddr::new(devserver_ip, devserver_port);

        // All servers will end up behind us (the devserver) but on a different port
        // This is so we can serve a loading screen as well as devtools without anything particularly fancy
        let proxied_port = args
            .should_proxy_build()
            .then(|| get_available_port(devserver_ip))
            .flatten();

        let proxied_address = proxied_port.map(|port| SocketAddr::new(devserver_ip, port));

        // Set up the router with some shared state
        let build_status = SharedStatus::new_with_starting_build();
        let router = build_devserver_router(
            args,
            cfg,
            hot_reload_sockets_tx,
            build_status_sockets_tx,
            proxied_address,
            build_status.clone(),
        )?;

        // Create the listener that we'll pass into the devserver, but save its IP here so
        // we can display it to the user in the tui
        let listener = std::net::TcpListener::bind(devserver_address).with_context(|| {
            anyhow::anyhow!(
                "Failed to bind server to: {devserver_address}, is there another devserver running?\nTo run multiple devservers, use the --address flag to specify a different port"
            )
        })?;

        // And finally, start the server mainloop
        tokio::spawn(devserver_mainloop(
            cfg.dioxus_config.web.https.clone(),
            listener,
            router,
        ));

        Ok(Self {
            proxied_port,
            devserver_ip,
            devserver_port,
            hot_reload_sockets: Default::default(),
            build_status_sockets: Default::default(),
            new_hot_reload_sockets: hot_reload_sockets_rx,
            new_build_status_sockets: build_status_sockets_rx,
            build_status,
            application_name: cfg.dioxus_config.application.name.clone(),
            platform: args.build_arguments.platform().to_string(),
        })
    }

    /// Wait for new clients to be connected and then save them
    pub(crate) async fn wait(&mut self) -> ServeUpdate {
        let mut new_hot_reload_socket = self.new_hot_reload_sockets.next();
        let mut new_build_status_socket = self.new_build_status_sockets.next();
        let mut new_message = self
            .hot_reload_sockets
            .iter_mut()
            .enumerate()
            .map(|(idx, socket)| async move { (idx, socket.next().await) })
            .collect::<FuturesUnordered<_>>();

        tokio::select! {
            new_hot_reload_socket = &mut new_hot_reload_socket => {
                if let Some(new_socket) = new_hot_reload_socket {
                    drop(new_message);
                    self.hot_reload_sockets.push(new_socket);
                    return ServeUpdate::NewConnection;
                } else {
                    panic!("Could not receive a socket - the devtools could not boot - the port is likely already in use");
                }
            }
            new_build_status_socket = &mut new_build_status_socket => {
                if let Some(mut new_socket) = new_build_status_socket {
                    drop(new_message);

                    // Update the socket with project info and current build status
                    let project_info = SharedStatus::new(Status::ClientInit { application_name: self.application_name.clone(), platform: self.platform.clone() });
                    if send_build_status_to(&project_info, &mut new_socket).await.is_ok() {
                        _ = send_build_status_to(&self.build_status, &mut new_socket).await;
                        self.build_status_sockets.push(new_socket);
                    }
                    return future::pending::<ServeUpdate>().await;
                } else {
                    panic!("Could not receive a socket - the devtools could not boot - the port is likely already in use");
                }
            }
            Some((idx, message)) = new_message.next() => {
                match message {
                    Some(Ok(message)) => return ServeUpdate::WsMessage(message),
                    _ => {
                        drop(new_message);
                        _ = self.hot_reload_sockets.remove(idx);
                    }
                }
            }
        }

        future::pending().await
    }

    pub(crate) async fn shutdown(&mut self) {
        self.send_shutdown().await;
        for socket in self.hot_reload_sockets.drain(..) {
            _ = socket.close().await;
        }
    }

    /// Sends the current build status to all clients.
    async fn send_build_status(&mut self) {
        let mut i = 0;
        while i < self.build_status_sockets.len() {
            let socket = &mut self.build_status_sockets[i];
            if send_build_status_to(&self.build_status, socket)
                .await
                .is_err()
            {
                self.build_status_sockets.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Sends a start build message to all clients.
    pub(crate) async fn start_build(&mut self) {
        self.build_status.set(Status::Building {
            progress: 0.0,
            build_message: "Starting the build...".to_string(),
        });
        self.send_build_status().await;
    }

    /// Sends an updated build status to all clients.
    pub(crate) async fn new_build_update(&mut self, update: &BuildUpdate) {
        match update {
            BuildUpdate::Progress { stage } => {
                //
                match stage {
                    BuildStage::Initializing => {}
                    BuildStage::InstallingTooling {} => {}
                    BuildStage::Compiling { current, total } => {}
                    BuildStage::OptimizingWasm {} => {}
                    BuildStage::OptimizingAssets {} => {}
                    BuildStage::Success => {}
                    BuildStage::Failed => self.send_reload_failed().await,
                    BuildStage::Aborted => {}
                    BuildStage::Restarting => self.send_reload_start().await,
                    BuildStage::CopyingAssets { current, total } => {}
                }
            }
            BuildUpdate::Message {} => {}
            BuildUpdate::BuildReady { bundle } => {}
            BuildUpdate::BuildFailed { err } => {}
        }

        // if !matches!(self.build_status.get(), Status::Building { .. }) {
        //     return;
        // }
        // self.build_status.set(Status::Building {
        //     progress,
        //     build_message,
        // });
        // self.send_build_status().await;
    }

    /// Sends hot reloadable changes to all clients.
    pub(crate) async fn send_hotreload(&mut self, reload: HotReloadMsg) {
        if !reload.assets.is_empty() {
            tracing::info!("Hot reloading assets {:?}", reload.assets);
        }

        let msg = DevserverMsg::HotReload(reload);
        let msg = serde_json::to_string(&msg).unwrap();

        // Send the changes to any connected clients
        let mut i = 0;
        while i < self.hot_reload_sockets.len() {
            let socket = &mut self.hot_reload_sockets[i];
            if socket.send(Message::Text(msg.clone())).await.is_err() {
                self.hot_reload_sockets.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Converts a `cargo` error to HTML and sends it to clients.
    pub(crate) async fn send_build_error(&mut self, error: Error) {
        let error = error.to_string();
        self.build_status.set(Status::BuildError {
            error: ansi_to_html::convert(&error).unwrap_or(error),
        });
        self.send_build_status().await;
    }

    /// Tells all clients that a full rebuild has started.
    pub(crate) async fn send_reload_start(&mut self) {
        self.send_devserver_message(DevserverMsg::FullReloadStart)
            .await;
    }

    /// Tells all clients that a full rebuild has failed.
    pub(crate) async fn send_reload_failed(&mut self) {
        self.send_devserver_message(DevserverMsg::FullReloadFailed)
            .await;
    }

    /// Tells all clients to reload if possible for new changes.
    pub(crate) async fn send_reload_command(&mut self) {
        self.build_status.set(Status::Ready);
        self.send_build_status().await;
        self.send_devserver_message(DevserverMsg::FullReloadCommand)
            .await;
    }

    /// Send a shutdown message to all connected clients.
    pub(crate) async fn send_shutdown(&mut self) {
        self.send_devserver_message(DevserverMsg::Shutdown).await;
    }

    /// Sends a devserver message to all connected clients.
    async fn send_devserver_message(&mut self, msg: DevserverMsg) {
        for socket in self.hot_reload_sockets.iter_mut() {
            _ = socket
                .send(Message::Text(serde_json::to_string(&msg).unwrap()))
                .await;
        }
    }

    /// Get the address the devserver should run on
    pub fn devserver_address(&self) -> SocketAddr {
        SocketAddr::new(self.devserver_ip, self.devserver_port)
    }

    // Get the address the server should run on if we're serving the user's server
    pub fn server_address(&self) -> Option<SocketAddr> {
        self.proxied_port
            .map(|port| SocketAddr::new(self.devserver_ip, port))
    }
}

/// Sets up and returns a router
///
/// Steps include:
/// - Setting up cors
/// - Setting up the proxy to the endpoint specified in the config
/// - Setting up the file serve service
/// - Setting up the websocket endpoint for devtools
fn build_devserver_router(
    args: &ServeArgs,
    krate: &DioxusCrate,
    hot_reload_sockets: UnboundedSender<WebSocket>,
    build_status_sockets: UnboundedSender<WebSocket>,
    fullstack_address: Option<SocketAddr>,
    build_status: SharedStatus,
) -> Result<Router> {
    let mut router = Router::new();

    // Setup proxy for the endpoint specified in the config
    for proxy_config in krate.dioxus_config.web.proxy.iter() {
        router = super::proxy::add_proxy(router, proxy_config)?;
    }

    if args.should_proxy_build() {
        // For fullstack, liveview, and server, forward all requests to the inner server
        let address = fullstack_address.unwrap();
        router = router.nest_service("/",super::proxy::proxy_to(
                format!("http://{address}").parse().unwrap(),
                true,
                |error| {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from(format!(
                            "Backend connection failed. The backend is likely still starting up. Please try again in a few seconds. Error: {:#?}",
                            error
                        )))
                        .unwrap()
                },
            ));
    } else {
        // Otherwise, just serve the dir ourselves
        // Route file service to output the .wasm and assets if this is a web build
        let base_path = format!(
            "/{}",
            krate
                .dioxus_config
                .web
                .app
                .base_path
                .as_deref()
                .unwrap_or_default()
                .trim_matches('/')
        );

        router = router.nest_service(&base_path, build_serve_dir(args, krate));
    }

    // Setup middleware to intercept html requests if the build status is "Building"
    router = router.layer(middleware::from_fn_with_state(
        build_status,
        build_status_middleware,
    ));

    // Setup websocket endpoint - and pass in the extension layer immediately after
    router = router.nest(
        "/_dioxus",
        Router::new()
            .route(
                "/",
                get(
                    |ws: WebSocketUpgrade, ext: Extension<UnboundedSender<WebSocket>>| async move {
                        tracing::info!("Incoming hotreload websocket request: {ws:?}");
                        ws.on_upgrade(move |socket| async move { _ = ext.0.unbounded_send(socket) })
                    },
                ),
            )
            .layer(Extension(hot_reload_sockets))
            .route(
                "/build_status",
                get(
                    |ws: WebSocketUpgrade, ext: Extension<UnboundedSender<WebSocket>>| async move {
                        ws.on_upgrade(move |socket| async move { _ = ext.0.unbounded_send(socket) })
                    },
                ),
            )
            .layer(Extension(build_status_sockets)),
    );

    // Setup cors
    router = router.layer(
        tower_http::cors::CorsLayer::new()
            // allow `GET` and `POST` when accessing the resource
            .allow_methods([Method::GET, Method::POST])
            // allow requests from any origin
            .allow_origin(Any)
            .allow_headers(Any),
    );

    Ok(router)
}

async fn devserver_mainloop(
    cfg: WebHttpsConfig,
    listener: TcpListener,
    router: Router,
) -> Result<std::result::Result<(), Error>, Error> {
    let rustls = get_rustls(&cfg).await.unwrap();
    let _ = listener.set_nonblocking(true);

    if let Some(rustls) = rustls {
        axum_server::from_tcp_rustls(listener, rustls)
            .serve(router.into_make_service())
            .await?
    } else {
        // Create a TCP listener bound to the address
        axum::serve(
            tokio::net::TcpListener::from_std(listener).unwrap(),
            router.into_make_service(),
        )
        .await?
    }
    Ok(Ok(()) as Result<()>)
}

fn build_serve_dir(args: &ServeArgs, cfg: &DioxusCrate) -> axum::routing::MethodRouter {
    use tower::ServiceBuilder;

    static CORS_UNSAFE: (HeaderValue, HeaderValue) = (
        HeaderValue::from_static("unsafe-none"),
        HeaderValue::from_static("unsafe-none"),
    );

    static CORS_REQUIRE: (HeaderValue, HeaderValue) = (
        HeaderValue::from_static("require-corp"),
        HeaderValue::from_static("same-origin"),
    );

    let (coep, coop) = match args.cross_origin_policy {
        true => CORS_REQUIRE.clone(),
        false => CORS_UNSAFE.clone(),
    };

    let out_dir = cfg.workdir(Platform::Web);
    let index_on_404 = cfg.dioxus_config.web.watcher.index_on_404;

    get_service(
        ServiceBuilder::new()
            .override_response_header(
                HeaderName::from_static("cross-origin-embedder-policy"),
                coep,
            )
            .override_response_header(HeaderName::from_static("cross-origin-opener-policy"), coop)
            .and_then({
                let out_dir = out_dir.clone();
                move |response| async move { Ok(no_cache(index_on_404, &out_dir, response)) }
            })
            .service(ServeDir::new(out_dir)),
    )
    .handle_error(|error: Infallible| async move {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled internal error: {}", error),
        )
    })
}

fn no_cache(
    index_on_404: bool,
    out_dir: &Path,
    response: Response<ServeFileSystemResponseBody>,
) -> Response<Body> {
    // By default we just decompose into the response
    let mut response = response.into_response();

    // If there's a 404 and we're supposed to index on 404, upgrade that failed request to the index.html
    // We might want to isnert a header here saying we *did* that but oh well
    if response.status() == StatusCode::NOT_FOUND && index_on_404 {
        let fallback = out_dir.join("index.html");
        let contents = std::fs::read_to_string(fallback).unwrap_or_else(|_| {
            String::from(
                r#"
            <!DOCTYPE html>
            <html>
                <head>
                    <title>Err 404 - dx is not serving a web app</title>
                </head>
                <body>
                <p>Err 404 - dioxus is not currently serving a web app</p>
                </body>
            </html>
            "#,
            )
        });
        let body = Body::from(contents);

        response = Response::builder()
            .status(StatusCode::OK)
            .body(body)
            .unwrap();
    };

    insert_no_cache_headers(response.headers_mut());

    response
}

pub(crate) fn insert_no_cache_headers(headers: &mut HeaderMap) {
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    headers.insert(PRAGMA, HeaderValue::from_static("no-cache"));
    headers.insert(EXPIRES, HeaderValue::from_static("0"));
}

/// Returns an enum of rustls config
async fn get_rustls(web_config: &WebHttpsConfig) -> Result<Option<RustlsConfig>> {
    if web_config.enabled != Some(true) {
        return Ok(None);
    }

    let (cert_path, key_path) = match web_config.mkcert {
        Some(true) => get_rustls_with_mkcert(web_config).await?,
        _ => get_rustls_without_mkcert(web_config)?,
    };

    Ok(Some(
        RustlsConfig::from_pem_file(cert_path, key_path).await?,
    ))
}

async fn get_rustls_with_mkcert(web_config: &WebHttpsConfig) -> Result<(String, String)> {
    const DEFAULT_KEY_PATH: &str = "ssl/key.pem";
    const DEFAULT_CERT_PATH: &str = "ssl/cert.pem";

    // Get paths to store certs, otherwise use ssl/item.pem
    let key_path = web_config
        .key_path
        .clone()
        .unwrap_or(DEFAULT_KEY_PATH.to_string());

    let cert_path = web_config
        .cert_path
        .clone()
        .unwrap_or(DEFAULT_CERT_PATH.to_string());

    // Create ssl directory if using defaults
    if key_path == DEFAULT_KEY_PATH && cert_path == DEFAULT_CERT_PATH {
        _ = fs::create_dir("ssl");
    }

    let cmd = tokio::process::Command::new("mkcert")
        .args([
            "-install",
            "-key-file",
            &key_path,
            "-cert-file",
            &cert_path,
            "localhost",
            "::1",
            "127.0.0.1",
        ])
        .spawn();

    match cmd {
        Err(e) => {
            match e.kind() {
                io::ErrorKind::NotFound => {
                    tracing::error!(dx_src = ?TraceSrc::Dev, "`mkcert` is not installed. See https://github.com/FiloSottile/mkcert#installation for installation instructions.")
                }
                e => {
                    tracing::error!(dx_src = ?TraceSrc::Dev, "An error occurred while generating mkcert certificates: {}", e.to_string())
                }
            };
            return Err("failed to generate mkcert certificates".into());
        }
        Ok(mut cmd) => {
            cmd.wait().await?;
        }
    }

    Ok((cert_path, key_path))
}

fn get_rustls_without_mkcert(web_config: &WebHttpsConfig) -> Result<(String, String)> {
    // get paths to cert & key
    if let (Some(key), Some(cert)) = (web_config.key_path.clone(), web_config.cert_path.clone()) {
        Ok((cert, key))
    } else {
        // missing cert or key
        Err("https is enabled but cert or key path is missing".into())
    }
}

fn get_available_port(address: IpAddr) -> Option<u16> {
    TcpListener::bind((address, 0))
        .map(|listener| listener.local_addr().unwrap().port())
        .ok()
}

/// Middleware that intercepts html requests if the status is "Building" and returns a loading page instead
async fn build_status_middleware(
    state: State<SharedStatus>,
    request: Request,
    next: Next,
) -> axum::response::Response {
    // If the request is for html, and the status is "Building", return the loading page instead of the contents of the response
    let accepts = request.headers().get(ACCEPT);
    let accepts_html = accepts
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/html"));

    if let Some(true) = accepts_html {
        let status = state.get();
        if status != Status::Ready {
            let html = include_str!("../../assets/loading.html");
            return axum::response::Response::builder()
                .status(StatusCode::OK)
                // Load the html loader then keep loading forever
                // We never close the stream so any headless testing framework (like playwright) will wait until the real build is done
                .body(Body::from_stream(
                    stream::once(async move { Ok::<_, std::convert::Infallible>(html) })
                        .chain(stream::pending()),
                ))
                .unwrap();
        }
    }

    next.run(request).await
}

async fn send_build_status_to(
    build_status: &SharedStatus,
    socket: &mut WebSocket,
) -> Result<(), axum::Error> {
    let msg = serde_json::to_string(&build_status.get()).unwrap();
    socket.send(Message::Text(msg)).await
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
enum Status {
    ClientInit {
        application_name: String,
        platform: String,
    },
    Building {
        progress: f64,
        build_message: String,
    },
    BuildError {
        error: String,
    },
    Ready,
}

#[derive(Debug, Clone)]
struct SharedStatus(Arc<RwLock<Status>>);

impl SharedStatus {
    fn new(status: Status) -> Self {
        Self(Arc::new(RwLock::new(status)))
    }

    fn new_with_starting_build() -> Self {
        Self::new(Status::Building {
            progress: 0.0,
            build_message: "Starting the build...".to_string(),
        })
    }

    fn set(&self, status: Status) {
        *self.0.write().unwrap() = status;
    }

    fn get(&self) -> Status {
        self.0.read().unwrap().clone()
    }
}
