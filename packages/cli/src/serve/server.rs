use crate::dioxus_crate::DioxusCrate;
use crate::serve::Serve;
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
use dioxus_cli_config::{Platform, WebHttpsConfig};
use dioxus_hot_reload::{DevserverMsg, HotReloadMsg};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::stream;
use futures_util::{stream::FuturesUnordered, StreamExt};
use hyper::header::ACCEPT;
use hyper::HeaderMap;
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock;
use std::{
    convert::Infallible,
    fs, io,
    net::{IpAddr, SocketAddr},
    process::Command,
};
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

pub(crate) struct Server {
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

impl Server {
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
            Platform::Fullstack | Platform::StaticGeneration
        ) {
            get_available_port(addr.ip())
        } else {
            None
        };

        let fullstack_address = fullstack_port.map(|port| SocketAddr::new(addr.ip(), port));

        let router = setup_router(
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
        let _server_task = tokio::spawn(async move {
            let web_config = web_config.clone();
            // HTTPS
            // Before console info so it can stop if mkcert isn't installed or fails
            // todo: this is the only async thing here - might be nice to
            let rustls: Option<RustlsConfig> = get_rustls(&web_config).await.unwrap();

            // Open the browser
            if start_browser {
                open_browser(base_path, addr, rustls.is_some());
            }

            // Start the server with or without rustls
            if let Some(rustls) = rustls {
                axum_server::bind_rustls(addr, rustls)
                    .serve(router.into_make_service())
                    .await?
            } else {
                // Create a TCP listener bound to the address
                axum::serve(
                    tokio::net::TcpListener::bind(&addr).await?,
                    router.into_make_service(),
                )
                .await?
            }

            Ok(())
        });

        Self {
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

    pub async fn start_build(&mut self) {
        self.build_status.set(Status::Building {
            progress: 0.0,
            build_message: "Starting the build...".to_string(),
        });
        self.send_build_status().await;
    }

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

    pub async fn send_hotreload(&mut self, reload: HotReloadMsg) {
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
    pub async fn wait(&mut self) -> Option<Message> {
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
                    return None;
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
                    return None;
                } else {
                    panic!("Could not receive a socket - the devtools could not boot - the port is likely already in use");
                }
            }
            Some((idx, message)) = new_message.next() => {
                match message {
                    Some(Ok(message)) => return Some(message),
                    _ => {
                        drop(new_message);
                        _ = self.hot_reload_sockets.remove(idx);
                    }
                }
            }
        }

        None
    }

    pub async fn send_build_error(&mut self, error: Error) {
        let error = error.to_string();
        self.build_status.set(Status::BuildError {
            error: ansi_to_html::convert(&error).unwrap_or(error),
        });
        self.send_build_status().await;
    }

    pub async fn send_reload(&mut self) {
        self.build_status.set(Status::Ready);
        self.send_build_status().await;
        for socket in self.hot_reload_sockets.iter_mut() {
            _ = socket
                .send(Message::Text(
                    serde_json::to_string(&DevserverMsg::FullReload).unwrap(),
                ))
                .await;
        }
    }

    /// Send a shutdown message to all connected clients
    pub async fn send_shutdown(&mut self) {
        for socket in self.hot_reload_sockets.iter_mut() {
            _ = socket
                .send(Message::Text(
                    serde_json::to_string(&DevserverMsg::Shutdown).unwrap(),
                ))
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

    // Setup base path redirection
    if let Some(base_path) = config.dioxus_config.web.app.base_path.clone() {
        let base_path = format!("/{}", base_path.trim_matches('/'));
        router = Router::new()
            .nest(&base_path, router)
            .fallback(get(move || async move {
                format!("Outside of the base path: {}", base_path)
            }));
    }

    // server the dir if it's web, otherwise let the fullstack server itself handle it
    match platform {
        Platform::Web => {
            // Route file service to output the .wasm and assets if this is a web build
            router = router.nest_service("/", build_serve_dir(serve, config));
        }
        Platform::Fullstack | Platform::StaticGeneration => {
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
        Some(true) => get_rustls_with_mkcert(web_config)?,
        _ => get_rustls_without_mkcert(web_config)?,
    };

    Ok(Some(
        RustlsConfig::from_pem_file(cert_path, key_path).await?,
    ))
}

pub fn get_rustls_with_mkcert(web_config: &WebHttpsConfig) -> Result<(String, String)> {
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
            cmd.wait()?;
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
