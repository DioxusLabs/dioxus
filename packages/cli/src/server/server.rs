use crate::dioxus_crate::DioxusCrate;
use crate::server::Serve;
use crate::Result;
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
    routing::{any, get, get_service},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use dioxus_cli_config::WebHttpsConfig;
use dioxus_hot_reload::{DevserverMsg, HotReloadMsg};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;
use std::{
    convert::Infallible,
    fs, io,
    net::{IpAddr, SocketAddr, UdpSocket},
    process::Command,
};
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    services::fs::{ServeDir, ServeFileSystemResponseBody},
    ServiceBuilderExt,
};

pub struct Server {
    pub sockets: Vec<WebSocket>,
    pub ip: IpAddr,
    pub new_socket: UnboundedReceiver<WebSocket>,
    pub server_task: JoinHandle<Result<()>>,
}

impl Server {
    pub async fn start(serve: &Serve, cfg: &DioxusCrate) -> Self {
        let (tx, rx) = futures_channel::mpsc::unbounded();

        let router = setup_router(serve, cfg, tx).await.unwrap();
        let port = serve.server_arguments.port;
        let start_browser = serve.server_arguments.open.unwrap_or_default();

        let ip = serve
            .server_arguments
            .addr
            .or_else(get_ip)
            .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)));

        let addr: SocketAddr = SocketAddr::from((ip, port));

        // HTTPS
        // Before console info so it can stop if mkcert isn't installed or fails
        // todo: this is the only async thing here - might be nice to
        let rustls: Option<RustlsConfig> = get_rustls(cfg).await.unwrap();

        // Open the browser
        if start_browser {
            open_browser(cfg, ip, port, rustls.is_some());
        }

        // Actually just start the server
        // todo: we might just be able to poll this future instead
        let server_task = tokio::spawn(async move {
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
            sockets: vec![],
            new_socket: rx,
            server_task,
            ip,
        }
    }

    pub fn update(&mut self, cfg: &Serve, crate_config: &DioxusCrate) {}

    pub async fn send_hotreload(&mut self, reload: HotReloadMsg) {
        let msg = DevserverMsg::HotReload(reload);
        let msg = serde_json::to_string(&msg).unwrap();

        // to our connected clients, send the changes to the sockets
        for socket in self.sockets.iter_mut() {
            if socket.send(Message::Text(msg.clone())).await.is_err() {
                // the socket is likely disconnected, we should remove it
                println!("error sending message to socket - it's likely disconnected");
            }
        }
    }

    /// Wait for new clients to be connected and then save them
    pub async fn wait(&mut self) {
        let new_socket = self
            .new_socket
            .next()
            .await
            .expect("receiver to receive a socket");
        println!("new socket connected: {:?}", new_socket);

        self.sockets.push(new_socket);
    }

    pub async fn shutdown(&self) {
        todo!()
    }
}

/// Sets up and returns a router
///
/// Steps include:
/// - Setting up cors
/// - Setting up the proxy to the /api/ endpoint specifed in the config
/// - Setting up the file serve service
/// - Setting up the websocket endpoint for devtools
pub async fn setup_router(
    serve: &Serve,
    config: &DioxusCrate,
    tx: UnboundedSender<WebSocket>,
) -> Result<Router> {
    let mut router = Router::new();

    // Setup proxy for the /api/ endpoint
    for proxy_config in config.dioxus_config.web.proxy.iter() {
        todo!("Configure proxy");
        // router = super::proxy::add_proxy(router, &proxy_config)?;
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

    // Route file service to output the .wasm and assets
    router = router.fallback(build_serve_dir(serve, config));

    // Setup websocket endpoint - and pass in the extension layer immediately after
    router = router
        .route(
            "/_dioxus",
            get(
                |ws: WebSocketUpgrade, ext: Extension<UnboundedSender<WebSocket>>| async move {
                    ws.on_upgrade(move |socket| async move { _ = ext.0.unbounded_send(socket) })
                },
            ),
        )
        .layer(Extension(tx));

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
    const CORS_UNSAFE: (HeaderValue, HeaderValue) = (
        HeaderValue::from_static("unsafe-none"),
        HeaderValue::from_static("unsafe-none"),
    );

    const CORS_REQUIRE: (HeaderValue, HeaderValue) = (
        HeaderValue::from_static("require-corp"),
        HeaderValue::from_static("same-origin"),
    );

    let (coep, coop) = match serve.server_arguments.cross_origin_policy {
        true => CORS_REQUIRE,
        false => CORS_UNSAFE,
    };

    let _cfg = cfg.clone();

    get_service(
        ServiceBuilder::new()
            .override_response_header(
                HeaderName::from_static("cross-origin-embedder-policy"),
                coep,
            )
            .override_response_header(HeaderName::from_static("cross-origin-opener-policy"), coop)
            .and_then(move |response| async move { Ok(no_cache(_cfg, response)) })
            .service(ServeDir::new(cfg.out_dir())),
    )
    .handle_error(|error: Infallible| async move {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled internal error: {}", error),
        )
    })
}

fn no_cache(cfg: DioxusCrate, response: Response<ServeFileSystemResponseBody>) -> Response<Body> {
    let index_on_404 = cfg.dioxus_config.web.watcher.index_on_404;

    // By default we just decompose into the response
    let mut response = response.into_response();

    // If there's a 404 and we're supposed to index on 404, upgrade that failed request to the index.html
    // We migth want to isnert a header here saying we *did* that but oh well
    if response.status() == StatusCode::NOT_FOUND && index_on_404 {
        let body = Body::from(
            std::fs::read_to_string(cfg.out_dir().join("index.html"))
                .ok()
                .unwrap(),
        );

        response = Response::builder()
            .status(StatusCode::OK)
            .body(body)
            .unwrap();
    };

    let headers = response.headers_mut();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    headers.insert(PRAGMA, HeaderValue::from_static("no-cache"));
    headers.insert(EXPIRES, HeaderValue::from_static("0"));
    response
}

/// Returns an enum of rustls config
pub async fn get_rustls(config: &DioxusCrate) -> Result<Option<RustlsConfig>> {
    let web_config = &config.dioxus_config.web.https;

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
pub(crate) fn open_browser(config: &DioxusCrate, ip: IpAddr, port: u16, https: bool) {
    let protocol = if https { "https" } else { "http" };
    let base_path = match config.dioxus_config.web.app.base_path.as_deref() {
        Some(base_path) => format!("/{}", base_path.trim_matches('/')),
        None => "".to_owned(),
    };
    _ = open::that(format!("{protocol}://{ip}:{port}{base_path}"));
}

/// Get the network ip
fn get_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().map(|addr| addr.ip()).ok()
}
