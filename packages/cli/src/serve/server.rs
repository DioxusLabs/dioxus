use crate::{
    config::WebHttpsConfig, serve::ServeUpdate, BuildId, BuildStage, BuilderUpdate, Platform,
    Result, TraceSrc,
};
use anyhow::Context;
use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket},
        Query, Request, State, WebSocketUpgrade,
    },
    http::{
        header::{HeaderName, HeaderValue, CACHE_CONTROL, EXPIRES, PRAGMA},
        Method, Response, StatusCode,
    },
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, get_service},
    Extension, Router,
};
use dioxus_devtools_types::{DevserverMsg, HotReloadMsg};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{
    future,
    stream::{self, FuturesUnordered},
    StreamExt,
};
use hyper::HeaderMap;
use serde::{Deserialize, Serialize};
use std::{
    convert::Infallible,
    fs, io,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
    path::Path,
    sync::{Arc, RwLock},
    time::Duration,
};
use subsecond_types::JumpTable;
use tokio::process::Command;
use tower_http::{
    cors::Any,
    services::fs::{ServeDir, ServeFileSystemResponseBody},
    ServiceBuilderExt,
};

use super::AppServer;

/// The webserver that serves statics assets (if fullstack isn't already doing that) and the websocket
/// communication layer that we use to send status updates and hotreloads to the client.
///
/// todo(jon): we should merge the build status and hotreload sockets into just a "devtools" socket
/// which carries all the message types. This would make it easier for us to add more message types
/// and better tooling on the pages that we serve.
pub(crate) struct WebServer {
    devserver_exposed_ip: IpAddr,
    devserver_bind_ip: IpAddr,
    devserver_port: u16,
    proxied_port: Option<u16>,
    hot_reload_sockets: Vec<ConnectedWsClient>,
    build_status_sockets: Vec<ConnectedWsClient>,
    new_hot_reload_sockets: UnboundedReceiver<ConnectedWsClient>,
    new_build_status_sockets: UnboundedReceiver<ConnectedWsClient>,
    build_status: SharedStatus,
    application_name: String,
    platform: Platform,
}

pub(crate) struct ConnectedWsClient {
    socket: WebSocket,
    build_id: Option<BuildId>,
    aslr_reference: Option<u64>,
    pid: Option<u32>,
}

impl WebServer {
    pub const SELF_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

    /// Start the development server.
    /// This will set up the default http server if there's no server specified (usually via fullstack).
    ///
    /// This will also start the websocket server that powers the devtools. If you want to communicate
    /// with connected devtools clients, this is the place to do it.
    pub(crate) fn start(runner: &AppServer) -> Result<Self> {
        let (hot_reload_sockets_tx, hot_reload_sockets_rx) = futures_channel::mpsc::unbounded();
        let (build_status_sockets_tx, build_status_sockets_rx) = futures_channel::mpsc::unbounded();

        // Create the listener that we'll pass into the devserver, but save its IP here so
        // we can display it to the user in the tui
        let devserver_bind_ip = runner.devserver_bind_ip;
        let devserver_port = runner.devserver_port;
        let proxied_port = runner.proxied_port;
        let devserver_exposed_ip = devserver_bind_ip;

        let devserver_bind_address = SocketAddr::new(devserver_bind_ip, devserver_port);
        let listener = std::net::TcpListener::bind(devserver_bind_address).with_context(|| {
            anyhow::anyhow!(
                "Failed to bind server to: {devserver_bind_address}, is there another devserver running?\nTo run multiple devservers, use the --port flag to specify a different port"
            )
        })?;

        let proxied_address = proxied_port.map(|port| SocketAddr::new(devserver_exposed_ip, port));

        // Set up the router with some shared state that we'll update later to reflect the current state of the build
        let build_status = SharedStatus::new_with_starting_build();
        let router = build_devserver_router(
            runner,
            hot_reload_sockets_tx,
            build_status_sockets_tx,
            proxied_address,
            build_status.clone(),
        )?;

        // And finally, start the server mainloop
        tokio::spawn(devserver_mainloop(
            runner.client().build.config.web.https.clone(),
            listener,
            router,
        ));

        Ok(Self {
            build_status,
            proxied_port,
            devserver_bind_ip,
            devserver_exposed_ip,
            devserver_port,
            hot_reload_sockets: Default::default(),
            build_status_sockets: Default::default(),
            new_hot_reload_sockets: hot_reload_sockets_rx,
            new_build_status_sockets: build_status_sockets_rx,
            application_name: runner.app_name().to_string(),
            platform: runner.client.build.platform,
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
            .map(|(idx, socket)| async move { (idx, socket.socket.next().await) })
            .collect::<FuturesUnordered<_>>();

        tokio::select! {
            new_hot_reload_socket = &mut new_hot_reload_socket => {
                if let Some(new_socket) = new_hot_reload_socket {
                    let aslr_reference = new_socket.aslr_reference;
                    let pid = new_socket.pid;
                    let id = new_socket.build_id.unwrap_or(BuildId::CLIENT);

                    drop(new_message);
                    self.hot_reload_sockets.push(new_socket);

                    return ServeUpdate::NewConnection { aslr_reference, id, pid };
                } else {
                    panic!("Could not receive a socket - the devtools could not boot - the port is likely already in use");
                }
            }
            new_build_status_socket = &mut new_build_status_socket => {
                if let Some(mut new_socket) = new_build_status_socket {
                    drop(new_message);

                    // Update the socket with project info and current build status
                    let project_info = SharedStatus::new(Status::ClientInit { application_name: self.application_name.clone(), platform: self.platform });
                    if project_info.send_to(&mut new_socket.socket).await.is_ok() {
                        _ = self.build_status.send_to(&mut new_socket.socket).await;
                        self.build_status_sockets.push(new_socket);
                    }
                    return future::pending::<ServeUpdate>().await;
                } else {
                    panic!("Could not receive a socket - the devtools could not boot - the port is likely already in use");
                }
            }
            Some((idx, message)) = new_message.next() => {
                match message {
                    Some(Ok(msg)) => return ServeUpdate::WsMessage { msg, platform: Platform::Web },
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
        for mut socket in self.hot_reload_sockets.drain(..) {
            _ = socket.socket.send(Message::Close(None)).await;
        }
    }

    /// Sends the current build status to all clients.
    async fn send_build_status(&mut self) {
        let mut i = 0;
        while i < self.build_status_sockets.len() {
            let socket = &mut self.build_status_sockets[i];
            if self.build_status.send_to(&mut socket.socket).await.is_err() {
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
    pub(crate) async fn new_build_update(&mut self, update: &BuilderUpdate) {
        match update {
            BuilderUpdate::Progress { stage } => {
                // Todo(miles): wire up more messages into the splash screen UI
                match stage {
                    BuildStage::Success => {}
                    BuildStage::Failed => self.send_reload_failed().await,
                    BuildStage::Restarting => self.send_reload_start().await,
                    BuildStage::Initializing => {}
                    BuildStage::InstallingTooling => {}
                    BuildStage::Compiling {
                        current,
                        total,
                        krate,
                        ..
                    } => {
                        if !matches!(
                            self.build_status.get(),
                            Status::Ready | Status::BuildError { .. }
                        ) {
                            self.build_status.set(Status::Building {
                                progress: (*current as f64 / *total as f64).clamp(0.0, 1.0),
                                build_message: format!("{krate} compiling"),
                            });
                            self.send_build_status().await;
                        }
                    }
                    BuildStage::OptimizingWasm => {}
                    BuildStage::Aborted => {}
                    BuildStage::CopyingAssets { .. } => {}
                    _ => {}
                }
            }
            BuilderUpdate::CompilerMessage { .. } => {}
            BuilderUpdate::BuildReady { .. } => {}
            BuilderUpdate::BuildFailed { err } => {
                let error = err.to_string();
                self.build_status.set(Status::BuildError {
                    error: ansi_to_html::convert(&error).unwrap_or(error),
                });
                self.send_reload_failed().await;
                self.send_build_status().await;
            }
            BuilderUpdate::StdoutReceived { .. } => {}
            BuilderUpdate::StderrReceived { .. } => {}
            BuilderUpdate::ProcessExited { .. } => {}
            BuilderUpdate::ProcessWaitFailed { .. } => {}
        }
    }

    pub(crate) fn has_hotreload_sockets(&self) -> bool {
        !self.hot_reload_sockets.is_empty()
    }

    /// Sends hot reloadable changes to all clients.
    pub(crate) async fn send_hotreload(&mut self, reload: HotReloadMsg) {
        if reload.is_empty() {
            return;
        }

        tracing::trace!("Sending hotreload to clients {:?}", reload);

        let msg = DevserverMsg::HotReload(reload);
        let msg = serde_json::to_string(&msg).unwrap();

        // Send the changes to any connected clients
        let mut i = 0;
        while i < self.hot_reload_sockets.len() {
            let socket = &mut self.hot_reload_sockets[i];
            if socket
                .socket
                .send(Message::Text(msg.clone().into()))
                .await
                .is_err()
            {
                self.hot_reload_sockets.remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub(crate) async fn send_patch(
        &mut self,
        jump_table: JumpTable,
        time_taken: Duration,
        build: BuildId,
        for_pid: Option<u32>,
    ) {
        let msg = DevserverMsg::HotReload(HotReloadMsg {
            jump_table: Some(jump_table),
            ms_elapsed: time_taken.as_millis() as u64,
            templates: vec![],
            assets: vec![],
            for_pid,
            for_build_id: Some(build.0 as _),
        });
        self.send_devserver_message_to_all(msg).await;
    }

    /// Tells all clients that a hot patch has started.
    pub(crate) async fn send_patch_start(&mut self) {
        self.send_devserver_message_to_all(DevserverMsg::HotPatchStart)
            .await;
    }

    /// Tells all clients that a full rebuild has started.
    pub(crate) async fn send_reload_start(&mut self) {
        self.send_devserver_message_to_all(DevserverMsg::FullReloadStart)
            .await;
    }

    /// Tells all clients that a full rebuild has failed.
    pub(crate) async fn send_reload_failed(&mut self) {
        self.send_devserver_message_to_all(DevserverMsg::FullReloadFailed)
            .await;
    }

    /// Tells all clients to reload if possible for new changes.
    pub(crate) async fn send_reload_command(&mut self) {
        tracing::trace!("Sending reload to toast");

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        self.build_status.set(Status::Ready);
        self.send_build_status().await;
        self.send_devserver_message_to_all(DevserverMsg::FullReloadCommand)
            .await;
    }

    /// Send a shutdown message to all connected clients.
    pub(crate) async fn send_shutdown(&mut self) {
        self.send_devserver_message_to_all(DevserverMsg::Shutdown)
            .await;
    }

    /// Sends a devserver message to all connected clients.
    async fn send_devserver_message_to_all(&mut self, msg: DevserverMsg) {
        for socket in self.hot_reload_sockets.iter_mut() {
            _ = socket
                .socket
                .send(Message::Text(serde_json::to_string(&msg).unwrap().into()))
                .await;
        }
    }

    /// Get the address the devserver should run on
    pub fn devserver_address(&self) -> SocketAddr {
        SocketAddr::new(self.devserver_exposed_ip, self.devserver_port)
    }

    // Get the address the server should run on if we're serving the user's server
    pub fn proxied_server_address(&self) -> Option<SocketAddr> {
        self.proxied_port
            .map(|port| SocketAddr::new(self.devserver_exposed_ip, port))
    }

    pub fn server_address(&self) -> Option<SocketAddr> {
        match self.platform {
            Platform::Web | Platform::Server => Some(self.devserver_address()),
            _ => self.proxied_server_address(),
        }
    }

    /// Get the address the server is running - showing 127.0.0.1 if the devserver is bound to 0.0.0.0
    /// This is designed this way to not confuse users who expect the devserver to be bound to localhost
    /// ... which it is, but they don't know that 0.0.0.0 also serves localhost.
    pub fn displayed_address(&self) -> Option<SocketAddr> {
        let mut address = self.server_address()?;

        // Set the port to the devserver port since that's usually what people expect
        address.set_port(self.devserver_port);

        if self.devserver_bind_ip == IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)) {
            address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), address.port());
        }

        Some(address)
    }
}

async fn devserver_mainloop(
    https_cfg: WebHttpsConfig,
    listener: TcpListener,
    router: Router,
) -> Result<()> {
    // We have a native listener that we're going to give to tokio, so we need to make it non-blocking
    let _ = listener.set_nonblocking(true);

    // If we're not using rustls, just use regular axum
    if https_cfg.enabled != Some(true) {
        axum::serve(
            tokio::net::TcpListener::from_std(listener).unwrap(),
            router.into_make_service(),
        )
        .await?;
        return Ok(());
    }

    // If we're using rustls, we need to get the cert/key paths and then set up rustls
    let (cert_path, key_path) = get_rustls(&https_cfg).await?;
    let rustls = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path).await?;

    axum_server::from_tcp_rustls(listener, rustls)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

/// Sets up and returns a router
///
/// Steps include:
/// - Setting up cors
/// - Setting up the proxy to the endpoint specified in the config
/// - Setting up the file serve service
/// - Setting up the websocket endpoint for devtools
fn build_devserver_router(
    runner: &AppServer,
    hot_reload_sockets: UnboundedSender<ConnectedWsClient>,
    build_status_sockets: UnboundedSender<ConnectedWsClient>,
    fullstack_address: Option<SocketAddr>,
    build_status: SharedStatus,
) -> Result<Router> {
    let mut router = Router::new();
    let build = runner.client();

    // Setup proxy for the endpoint specified in the config
    for proxy_config in build.build.config.web.proxy.iter() {
        router = super::proxy::add_proxy(router, proxy_config)?;
    }

    // For fullstack, liveview, and server, forward all requests to the inner server
    if runner.proxied_port.is_some() {
        tracing::debug!("Proxying requests to fullstack server at {fullstack_address:?}");
        let address = fullstack_address.context("No fullstack address specified")?;
        tracing::debug!("Proxying requests to fullstack server at {address}");
        router = router.fallback_service(super::proxy::proxy_to(
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
            runner
                .client()
                .build
                .config
                .web
                .app
                .base_path
                .as_deref()
                .unwrap_or_default()
                .trim_matches('/')
        );
        if base_path == "/" {
            router = router.fallback_service(build_serve_dir(runner));
        } else {
            router = router.nest_service(&base_path, build_serve_dir(runner));
        }
    }

    // Setup middleware to intercept html requests if the build status is "Building"
    router = router.layer(middleware::from_fn_with_state(
        build_status,
        build_status_middleware,
    ));

    #[derive(Deserialize, Debug)]
    struct ConnectionQuery {
        aslr_reference: Option<u64>,
        build_id: Option<BuildId>,
        pid: Option<u32>,
    }

    // Setup websocket endpoint - and pass in the extension layer immediately after
    router = router.nest(
        "/_dioxus",
        Router::new()
            .route(
                "/",
                get(
                    |ws: WebSocketUpgrade, ext: Extension<UnboundedSender<ConnectedWsClient>>, query: Query<ConnectionQuery>| async move {
                        tracing::debug!("New devtool websocket connection: {:?}", query);
                        ws.on_upgrade(move |socket| async move { _ = ext.0.unbounded_send(ConnectedWsClient { socket, aslr_reference: query.aslr_reference, build_id: query.build_id, pid: query.pid }) })
                    },
                ),
            )
            .layer(Extension(hot_reload_sockets))
            .route(
                "/build_status",
                get(
                    |ws: WebSocketUpgrade, ext: Extension<UnboundedSender<ConnectedWsClient>>| async move {
                        ws.on_upgrade(move |socket| async move { _ = ext.0.unbounded_send(ConnectedWsClient { socket, aslr_reference: None, build_id: None, pid: None }) })
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

fn build_serve_dir(runner: &AppServer) -> axum::routing::MethodRouter {
    use tower::ServiceBuilder;

    static CORS_UNSAFE: (HeaderValue, HeaderValue) = (
        HeaderValue::from_static("unsafe-none"),
        HeaderValue::from_static("unsafe-none"),
    );

    static CORS_REQUIRE: (HeaderValue, HeaderValue) = (
        HeaderValue::from_static("require-corp"),
        HeaderValue::from_static("same-origin"),
    );

    let (coep, coop) = match runner.cross_origin_policy {
        true => CORS_REQUIRE.clone(),
        false => CORS_UNSAFE.clone(),
    };

    let app = &runner.client;
    let cfg = &runner.client.build.config;

    let out_dir = app.build.root_dir();
    let index_on_404: bool = cfg.web.watcher.index_on_404;

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
            .service(ServeDir::new(&out_dir)),
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

async fn get_rustls(web_config: &WebHttpsConfig) -> Result<(String, String)> {
    // If we're not using mkcert, just use the cert/key paths given to use in the config
    if !web_config.mkcert.unwrap_or(false) {
        if let (Some(key), Some(cert)) = (web_config.key_path.clone(), web_config.cert_path.clone())
        {
            return Ok((cert, key));
        } else {
            // missing cert or key
            return Err("https is enabled but cert or key path is missing".into());
        }
    }

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

    let cmd = Command::new("mkcert")
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

/// Middleware that intercepts html requests if the status is "Building" and returns a loading page instead
async fn build_status_middleware(
    state: State<SharedStatus>,
    request: Request,
    next: Next,
) -> axum::response::Response {
    // If the request is for html, and the status is "Building", return the loading page instead of the contents of the response
    let accepts = request.headers().get(hyper::header::ACCEPT);
    let accepts_html = accepts
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/html"));

    if let Some(true) = accepts_html {
        let status = state.get();
        if status != Status::Ready {
            let html = include_str!("../../assets/web/dev.loading.html");
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

#[derive(Debug, Clone)]
struct SharedStatus(Arc<RwLock<Status>>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
enum Status {
    ClientInit {
        application_name: String,
        platform: Platform,
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

    async fn send_to(&self, socket: &mut WebSocket) -> Result<(), axum::Error> {
        let msg = serde_json::to_string(&self.get()).unwrap();
        socket.send(Message::Text(msg.into())).await
    }
}
