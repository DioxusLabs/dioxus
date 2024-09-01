use crate::{builder::BuildRequest, dioxus_crate::DioxusCrate};
use crate::{
    builder::TargetPlatform,
    serve::{next_or_pending, Serve},
};
use crate::{
    config::{Platform, WebHttpsConfig},
    serve::update::ServeUpdate,
};
use crate::{Error, Result};
use axum::extract::{Request, State};
use axum::middleware::{self, Next};
use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket},
        Extension, WebSocketUpgrade,
    },
    http::{
        header::{HeaderName, HeaderValue, CACHE_CONTROL, EXPIRES, PRAGMA},
        Method, Response, StatusCode,
    },
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use dioxus_devtools_types::{DevserverMsg, HotReloadMsg};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{future, stream};
use futures_util::{stream::FuturesUnordered, StreamExt};
use hyper::header::ACCEPT;
use hyper::HeaderMap;
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::RwLock;
use std::{
    convert::Infallible,
    fs, io,
    net::{IpAddr, SocketAddr},
};
use std::{path::Path, process::Stdio};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    services::fs::{ServeDir, ServeFileSystemResponseBody},
    ServiceBuilderExt,
};

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

    fn set(&self, status: Status) {
        *self.0.write().unwrap() = status;
    }

    fn get(&self) -> Status {
        self.0.read().unwrap().clone()
    }
}

pub struct DevServer {
    pub serve: Serve,
    pub hot_reload_sockets: Vec<WebSocket>,
    pub build_status_sockets: Vec<WebSocket>,
    pub ip: SocketAddr,
    pub new_hot_reload_sockets: UnboundedReceiver<WebSocket>,
    pub new_build_status_sockets: UnboundedReceiver<WebSocket>,
    _server_task: JoinHandle<Result<()>>,

    /// We proxy (not hot reloading) fullstack requests to this port
    pub fullstack_port: Option<u16>,

    build_status: SharedStatus,
    application_name: String,
    platform: String,
}

impl DevServer {
    pub fn start(serve: &Serve, cfg: &DioxusCrate) -> Self {
        let (hot_reload_sockets_tx, hot_reload_sockets_rx) = futures_channel::mpsc::unbounded();
        let (build_status_sockets_tx, build_status_sockets_rx) = futures_channel::mpsc::unbounded();

        let build_status = SharedStatus::new(Status::Building {
            progress: 0.0,
            build_message: "Starting the build...".to_string(),
        });

        let addr = serve.server_arguments.address.address();
        let start_browser = serve.server_arguments.open.unwrap_or_default();

        // If we're serving a fullstack app, we need to find a port to proxy to
        let fullstack_port = if matches!(
            serve.build_arguments.platform(),
            Platform::Liveview | Platform::Fullstack
        ) {
            get_available_port(addr.ip())
        } else {
            None
        };

        let fullstack_address = fullstack_port.map(|port| SocketAddr::new(addr.ip(), port));

        let router = Self::setup_router(
            serve,
            cfg,
            hot_reload_sockets_tx,
            build_status_sockets_tx,
            fullstack_address,
            build_status.clone(),
        )
        .unwrap();

        // Actually just start the server, cloning in a few bits of config
        let web_config = cfg.dioxus_config.web.https.clone();
        let base_path = cfg.dioxus_config.web.app.base_path.clone();
        let platform = serve.platform();

        let listener = std::net::TcpListener::bind(addr).expect("Failed to bind port");
        _ = listener.set_nonblocking(true);
        let addr = listener.local_addr().unwrap();

        let _server_task = tokio::spawn(async move {
            let web_config = web_config.clone();
            // HTTPS
            // Before console info so it can stop if mkcert isn't installed or fails
            // todo: this is the only async thing here - might be nice to
            let rustls: Option<RustlsConfig> = get_rustls(&web_config).await.unwrap();

            // Open the browser
            if start_browser && platform != Platform::Desktop {
                open_browser(base_path, addr, rustls.is_some());
            }

            // Start the server with or without rustls
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

            Ok(())
        });

        Self {
            serve: serve.clone(),
            hot_reload_sockets: Default::default(),
            build_status_sockets: Default::default(),
            new_hot_reload_sockets: hot_reload_sockets_rx,
            new_build_status_sockets: build_status_sockets_rx,
            _server_task,
            ip: addr,
            fullstack_port,

            build_status,
            application_name: cfg.dioxus_config.application.name.clone(),
            platform: serve.build_arguments.platform().to_string(),
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
    pub async fn start_build(&mut self) {
        self.build_status.set(Status::Building {
            progress: 0.0,
            build_message: "Starting the build...".to_string(),
        });
        self.send_build_status().await;
    }

    /// Sends an updated build status to all clients.
    pub async fn update_build_status(&mut self, progress: f64, build_message: String) {
        if !matches!(self.build_status.get(), Status::Building { .. }) {
            return;
        }
        self.build_status.set(Status::Building {
            progress,
            build_message,
        });
        self.send_build_status().await;
    }

    /// Sends hot reloadable changes to all clients.
    pub async fn send_hotreload(&mut self, reload: HotReloadMsg) {
        if !reload.assets.is_empty() {
            tracing::debug!("Hot reloading assets {:?}", reload.assets);
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

    /// Wait for new clients to be connected and then save them
    pub async fn wait(&mut self) -> ServeUpdate {
        let mut new_hot_reload_socket = self.new_hot_reload_sockets.next();
        let mut new_build_status_socket = self.new_build_status_sockets.next();
        let mut new_message = self
            .hot_reload_sockets
            .iter_mut()
            .enumerate()
            .map(|(idx, socket)| async move { (idx, socket.next().await) })
            .collect::<FuturesUnordered<_>>();
        let next_new_message = next_or_pending(new_message.next());

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
            (idx, message) = next_new_message => {
                match message {
                    Some(Ok(message)) => return ServeUpdate::Message(message),
                    _ => {
                        drop(new_message);
                        _ = self.hot_reload_sockets.remove(idx);
                    }
                }
            }
        }

        future::pending().await
    }

    /// Converts a `cargo` error to HTML and sends it to clients.
    pub async fn send_build_error(&mut self, error: Error) {
        let error = error.to_string();
        self.build_status.set(Status::BuildError {
            error: ansi_to_html::convert(&error).unwrap_or(error),
        });
        self.send_build_status().await;
    }

    /// Tells all clients that a full rebuild has started.
    pub async fn send_reload_start(&mut self) {
        self.send_devserver_message(DevserverMsg::FullReloadStart)
            .await;
    }

    /// Tells all clients that a full rebuild has failed.
    pub async fn send_reload_failed(&mut self) {
        self.send_devserver_message(DevserverMsg::FullReloadFailed)
            .await;
    }

    /// Tells all clients to reload if possible for new changes.
    pub async fn send_reload_command(&mut self) {
        self.build_status.set(Status::Ready);
        self.send_build_status().await;
        self.send_devserver_message(DevserverMsg::FullReloadCommand)
            .await;
    }

    /// Send a shutdown message to all connected clients.
    pub async fn send_shutdown(&mut self) {
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

    pub async fn shutdown(&mut self) {
        self.send_shutdown().await;
        for socket in self.hot_reload_sockets.drain(..) {
            _ = socket.close().await;
        }
    }

    /// Get the address the fullstack server should run on if we're serving a fullstack app
    pub fn fullstack_address(&self) -> Option<SocketAddr> {
        self.fullstack_port
            .map(|port| SocketAddr::new(self.ip.ip(), port))
    }

    /// Open the executable if this is a native build
    pub fn open(&self, build: &BuildRequest) -> std::io::Result<Option<Child>> {
        match build.target_platform {
            TargetPlatform::Web => Ok(None),
            TargetPlatform::Mobile => self.open_bundled_ios_app(build),
            TargetPlatform::Desktop | TargetPlatform::Server | TargetPlatform::Liveview => {
                self.open_unbundled_native_app(build)
            }
        }
    }

    fn open_unbundled_native_app(&self, build: &BuildRequest) -> std::io::Result<Option<Child>> {
        if build.target_platform == TargetPlatform::Server {
            tracing::trace!(
                "Proxying fullstack server from port {:?}",
                self.fullstack_address()
            );
        }

        tracing::info!(
            "Opening exectuable with dev server ip {}",
            self.ip.to_string()
        );

        //
        // open the exe with some arguments/envvars/etc
        // we're going to try and configure this binary from the environment, if we can
        //
        // web can't be configured like this, so instead, we'll need to plumb a meta tag into the
        // index.html during dev
        //
        let res = Command::new(
            build
                .executable
                .as_deref()
                .expect("executable should be built if we're trying to open it")
                .canonicalize()?,
        )
        .env(
            dioxus_runtime_config::FULLSTACK_ADDRESS_ENV,
            self.fullstack_address()
                .as_ref()
                .map(|addr| addr.to_string())
                .unwrap_or_else(|| "127.0.0.1:8080".to_string()),
        )
        .env(
            dioxus_runtime_config::IOS_DEVSERVER_ADDR_ENV,
            format!("ws://{}/_dioxus", self.ip.to_string()),
        )
        .env(
            dioxus_runtime_config::DEVSERVER_RAW_ADDR_ENV,
            format!("ws://{}/_dioxus", self.ip.to_string()),
        )
        .env("CARGO_MANIFEST_DIR", build.krate.crate_dir())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .current_dir(build.krate.workspace_dir())
        .spawn()?;

        Ok(Some(res))
    }

    fn open_bundled_ios_app(&self, build: &BuildRequest) -> std::io::Result<Option<Child>> {
        // command = "xcrun"
        // args = [
        // "simctl",
        // "install",
        // "booted",
        // "target/aarch64-apple-ios-sim/debug/bundle/ios/DioxusApp.app",
        // ]

        // [tasks.run_ios_sim]
        // args = ["simctl", "launch", "--console", "booted", "com.dioxuslabs"]
        // command = "xcrun"
        // dependencies = ["build_ios_sim", "install_ios_sim"]

        // [tasks.serve-sim]
        // dependencies = ["build_ios_sim", "install_ios_sim", "run_ios_sim"]

        // APP_PATH="target/aarch64-apple-ios/debug/bundle/ios/DioxusApp.app"

        // # get the device id by jq-ing the json of the device list
        // xcrun devicectl list devices --json-output target/deviceid.json
        // DEVICE_UUID=$(jq -r '.result.devices[0].identifier' target/deviceid.json)

        // xcrun devicectl device install app --device "${DEVICE_UUID}" "${APP_PATH}" --json-output target/xcrun.json

        // # get the installation url by jq-ing the json of the device install
        // INSTALLATION_URL=$(jq -r '.result.installedApplications[0].installationURL' target/xcrun.json)

        // # launch the app
        // # todo: we can just background it immediately and then pick it up for loading its logs
        // xcrun devicectl device process launch --device "${DEVICE_UUID}" "${INSTALLATION_URL}"

        // # # launch the app and put it in background
        // # xcrun devicectl device process launch --no-activate --verbose --device "${DEVICE_UUID}" "${INSTALLATION_URL}" --json-output "${XCRUN_DEVICE_PROCESS_LAUNCH_LOG_DIR}"

        // # # Extract background PID of status app
        // # STATUS_PID=$(jq -r '.result.process.processIdentifier' "${XCRUN_DEVICE_PROCESS_LAUNCH_LOG_DIR}")
        // # "${GIT_ROOT}/scripts/wait-for-metro-port.sh"  2>&1

        // # # now that metro is ready, resume the app from background
        // # xcrun devicectl device process resume --device "${DEVICE_UUID}" --pid "${STATUS_PID}" > "${XCRUN_DEVICE_PROCESS_RESUME_LOG_DIR}" 2>&1
        todo!("Open mobile apps")
    }

    /// Sets up and returns a router
    ///
    /// Steps include:
    /// - Setting up cors
    /// - Setting up the proxy to the endpoint specified in the config
    /// - Setting up the file serve service
    /// - Setting up the websocket endpoint for devtools
    fn setup_router(
        serve: &Serve,
        config: &DioxusCrate,
        hot_reload_sockets: UnboundedSender<WebSocket>,
        build_status_sockets: UnboundedSender<WebSocket>,
        fullstack_address: Option<SocketAddr>,
        build_status: SharedStatus,
    ) -> Result<Router> {
        let mut router = Router::new();
        let platform = serve.build_arguments.platform();

        // Setup proxy for the endpoint specified in the config
        for proxy_config in config.dioxus_config.web.proxy.iter() {
            router = super::proxy::add_proxy(router, proxy_config)?;
        }

        // server the dir if it's web, otherwise let the fullstack server itself handle it
        match platform {
            Platform::Web => {
                // Route file service to output the .wasm and assets if this is a web build
                let base_path = format!(
                    "/{}",
                    config
                        .dioxus_config
                        .web
                        .app
                        .base_path
                        .as_deref()
                        .unwrap_or_default()
                        .trim_matches('/')
                );

                router = router.nest_service(&base_path, build_serve_dir(serve, config));
            }
            Platform::Liveview | Platform::Fullstack => {
                // For fullstack and static generation, forward all requests to the server
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
            }
            _ => {}
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
            CorsLayer::new()
                // allow `GET` and `POST` when accessing the resource
                .allow_methods([Method::GET, Method::POST])
                // allow requests from any origin
                .allow_origin(Any)
                .allow_headers(Any),
        );

        Ok(router)
    }
}

fn build_serve_dir(serve: &Serve, cfg: &DioxusCrate) -> axum::routing::MethodRouter {
    static CORS_UNSAFE: (HeaderValue, HeaderValue) = (
        HeaderValue::from_static("unsafe-none"),
        HeaderValue::from_static("unsafe-none"),
    );

    static CORS_REQUIRE: (HeaderValue, HeaderValue) = (
        HeaderValue::from_static("require-corp"),
        HeaderValue::from_static("same-origin"),
    );

    let (coep, coop) = match serve.server_arguments.cross_origin_policy {
        true => CORS_REQUIRE.clone(),
        false => CORS_UNSAFE.clone(),
    };

    let out_dir = cfg.out_dir();
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
        let body = Body::from(std::fs::read_to_string(out_dir.join("index.html")).unwrap());

        response = Response::builder()
            .status(StatusCode::OK)
            .body(body)
            .unwrap();
    };

    insert_no_cache_headers(response.headers_mut());
    response
}

pub fn insert_no_cache_headers(headers: &mut HeaderMap) {
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    headers.insert(PRAGMA, HeaderValue::from_static("no-cache"));
    headers.insert(EXPIRES, HeaderValue::from_static("0"));
}

/// Returns an enum of rustls config
pub async fn get_rustls(web_config: &WebHttpsConfig) -> Result<Option<RustlsConfig>> {
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

pub async fn get_rustls_with_mkcert(web_config: &WebHttpsConfig) -> Result<(String, String)> {
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
                io::ErrorKind::NotFound => tracing::error!("mkcert is not installed. See https://github.com/FiloSottile/mkcert#installation for installation instructions."),
                e => tracing::error!("an error occurred while generating mkcert certificates: {}", e.to_string()),
            };
            return Err("failed to generate mkcert certificates".into());
        }
        Ok(mut cmd) => {
            cmd.wait().await?;
        }
    }

    Ok((cert_path, key_path))
}

pub fn get_rustls_without_mkcert(web_config: &WebHttpsConfig) -> Result<(String, String)> {
    // get paths to cert & key
    if let (Some(key), Some(cert)) = (web_config.key_path.clone(), web_config.cert_path.clone()) {
        Ok((cert, key))
    } else {
        // missing cert or key
        Err("https is enabled but cert or key path is missing".into())
    }
}

/// Open the browser to the address
pub(crate) fn open_browser(base_path: Option<String>, address: SocketAddr, https: bool) {
    let protocol = if https { "https" } else { "http" };
    let base_path = match base_path.as_deref() {
        Some(base_path) => format!("/{}", base_path.trim_matches('/')),
        None => "".to_owned(),
    };
    _ = open::that(format!("{protocol}://{address}{base_path}"));
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
