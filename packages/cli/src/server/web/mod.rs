use crate::{
    builder,
    serve::Serve,
    server::{
        output::{print_console_info, PrettierOptions, WebServerInfo},
        setup_file_watcher, HotReloadState,
    },
    BuildResult, CrateConfig, Result, WebHttpsConfig,
};
use axum::{
    body::{Full, HttpBody},
    extract::{ws::Message, Extension, TypedHeader, WebSocketUpgrade},
    http::{
        header::{HeaderName, HeaderValue},
        Method, Response, StatusCode,
    },
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;

use dioxus_html::HtmlCtx;
use dioxus_rsx::hot_reload::*;
use std::{
    net::UdpSocket,
    process::Command,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast::{self, Sender};
use tower::ServiceBuilder;
use tower_http::services::fs::{ServeDir, ServeFileSystemResponseBody};
use tower_http::{
    cors::{Any, CorsLayer},
    ServiceBuilderExt,
};

#[cfg(feature = "plugin")]
use plugin::PluginManager;

mod proxy;

mod hot_reload;
use hot_reload::*;

struct WsReloadState {
    update: broadcast::Sender<()>,
}

pub async fn startup(port: u16, config: CrateConfig, start_browser: bool) -> Result<()> {
    // ctrl-c shutdown checker
    let _crate_config = config.clone();
    let _ = ctrlc::set_handler(move || {
        #[cfg(feature = "plugin")]
        let _ = PluginManager::on_serve_shutdown(&_crate_config);
        std::process::exit(0);
    });

    let ip = get_ip().unwrap_or(String::from("0.0.0.0"));

    let hot_reload_state = match config.hot_reload {
        true => {
            let FileMapBuildResult { map, errors } =
                FileMap::<HtmlCtx>::create(config.crate_dir.clone()).unwrap();

            for err in errors {
                log::error!("{}", err);
            }

            let file_map = Arc::new(Mutex::new(map));

            let hot_reload_tx = broadcast::channel(100).0;

            Some(HotReloadState {
                messages: hot_reload_tx.clone(),
                file_map: file_map.clone(),
            })
        }
        false => None,
    };

    serve(ip, port, config, start_browser, hot_reload_state).await?;

    Ok(())
}

/// Start the server without hot reload
pub async fn serve(
    ip: String,
    port: u16,
    config: CrateConfig,
    start_browser: bool,
    hot_reload_state: Option<HotReloadState>,
) -> Result<()> {
    let first_build_result = crate::builder::build(&config, true)?;

    log::info!("🚀 Starting development server...");

    // WS Reload Watching
    let (reload_tx, _) = broadcast::channel(100);

    // We got to own watcher so that it exists for the duration of serve
    // Otherwise full reload won't work.
    let _watcher = setup_file_watcher(
        {
            let config = config.clone();
            let reload_tx = reload_tx.clone();
            move || build(&config, &reload_tx)
        },
        &config,
        Some(WebServerInfo {
            ip: ip.clone(),
            port,
        }),
        hot_reload_state.clone(),
    )
    .await?;

    let ws_reload_state = Arc::new(WsReloadState {
        update: reload_tx.clone(),
    });

    // HTTPS
    // Before console info so it can stop if mkcert isn't installed or fails
    let rustls_config = get_rustls(&config).await?;

    // Print serve info
    print_console_info(
        &config,
        PrettierOptions {
            changed: vec![],
            warnings: first_build_result.warnings,
            elapsed_time: first_build_result.elapsed_time,
        },
        Some(crate::server::output::WebServerInfo {
            ip: ip.clone(),
            port,
        }),
    );

    // Router
    let router = setup_router(config, ws_reload_state, hot_reload_state).await?;

    // Start server
    start_server(port, router, start_browser, rustls_config).await?;

    Ok(())
}

const DEFAULT_KEY_PATH: &str = "ssl/key.pem";
const DEFAULT_CERT_PATH: &str = "ssl/cert.pem";

/// Returns an enum of rustls config and a bool if mkcert isn't installed
async fn get_rustls(config: &CrateConfig) -> Result<Option<RustlsConfig>> {
    let web_config = &config.dioxus_config.web.https;
    if web_config.enabled != Some(true) {
        return Ok(None);
    }

    let (cert_path, key_path) = if let Some(true) = web_config.mkcert {
        // mkcert, use it
        get_rustls_with_mkcert(web_config)?
    } else {
        // if mkcert not specified or false, don't use it
        get_rustls_without_mkcert(web_config)?
    };

    Ok(Some(
        RustlsConfig::from_pem_file(cert_path, key_path).await?,
    ))
}

fn get_rustls_with_mkcert(web_config: &WebHttpsConfig) -> Result<(String, String)> {
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
                io::ErrorKind::NotFound => log::error!("mkcert is not installed. See https://github.com/FiloSottile/mkcert#installation for installation instructions."),
                e => log::error!("an error occured while generating mkcert certificates: {}", e.to_string()),
            };
            return Err("failed to generate mkcert certificates".into());
        }
        Ok(mut cmd) => {
            cmd.wait()?;
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

/// Sets up and returns a router
async fn setup_router(
    config: CrateConfig,
    ws_reload: Arc<WsReloadState>,
    hot_reload: Option<HotReloadState>,
) -> Result<Router> {
    // Setup cors
    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any)
        .allow_headers(Any);

    let (coep, coop) = if config.cross_origin_policy {
        (
            HeaderValue::from_static("require-corp"),
            HeaderValue::from_static("same-origin"),
        )
    } else {
        (
            HeaderValue::from_static("unsafe-none"),
            HeaderValue::from_static("unsafe-none"),
        )
    };

    // Create file service
    let file_service_config = config.clone();
    let file_service = ServiceBuilder::new()
        .override_response_header(
            HeaderName::from_static("cross-origin-embedder-policy"),
            coep,
        )
        .override_response_header(HeaderName::from_static("cross-origin-opener-policy"), coop)
        .and_then(
            move |response: Response<ServeFileSystemResponseBody>| async move {
                let response = if file_service_config
                    .dioxus_config
                    .web
                    .watcher
                    .index_on_404
                    .unwrap_or(false)
                    && response.status() == StatusCode::NOT_FOUND
                {
                    let body = Full::from(
                        // TODO: Cache/memoize this.
                        std::fs::read_to_string(
                            file_service_config
                                .crate_dir
                                .join(file_service_config.out_dir)
                                .join("index.html"),
                        )
                        .ok()
                        .unwrap(),
                    )
                    .map_err(|err| match err {})
                    .boxed();
                    Response::builder()
                        .status(StatusCode::OK)
                        .body(body)
                        .unwrap()
                } else {
                    response.map(|body| body.boxed())
                };
                Ok(response)
            },
        )
        .service(ServeDir::new(config.crate_dir.join(&config.out_dir)));

    // Setup websocket
    let mut router = Router::new().route("/_dioxus/ws", get(ws_handler));

    // Setup proxy
    for proxy_config in config.dioxus_config.web.proxy.unwrap_or_default() {
        router = proxy::add_proxy(router, &proxy_config)?;
    }

    // Route file service
    router = router.fallback(get_service(file_service).handle_error(
        |error: std::io::Error| async move {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unhandled internal error: {}", error),
            )
        },
    ));

    // Setup routes
    router = router
        .route("/_dioxus/hot_reload", get(hot_reload_handler))
        .layer(cors)
        .layer(Extension(ws_reload));

    if let Some(hot_reload) = hot_reload {
        router = router.layer(Extension(hot_reload))
    }

    Ok(router)
}

/// Starts dx serve with no hot reload
async fn start_server(
    port: u16,
    router: Router,
    start_browser: bool,
    rustls: Option<RustlsConfig>,
) -> Result<()> {
    // If plugins, call on_serve_start event
    #[cfg(feature = "plugin")]
    PluginManager::on_serve_start(&config)?;

    // Parse address
    let addr = format!("0.0.0.0:{}", port).parse().unwrap();

    // Open the browser
    if start_browser {
        match rustls {
            Some(_) => _ = open::that(format!("https://{}", addr)),
            None => _ = open::that(format!("http://{}", addr)),
        }
    }

    // Start the server with or without rustls
    match rustls {
        Some(rustls) => {
            axum_server::bind_rustls(addr, rustls)
                .serve(router.into_make_service())
                .await?
        }
        None => {
            axum::Server::bind(&addr)
                .serve(router.into_make_service())
                .await?
        }
    }

    Ok(())
}

/// Get the network ip
fn get_ip() -> Option<String> {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return None,
    };

    match socket.connect("8.8.8.8:80") {
        Ok(()) => (),
        Err(_) => return None,
    };

    match socket.local_addr() {
        Ok(addr) => Some(addr.ip().to_string()),
        Err(_) => None,
    }
}

/// Handle websockets
async fn ws_handler(
    ws: WebSocketUpgrade,
    _: Option<TypedHeader<headers::UserAgent>>,
    Extension(state): Extension<Arc<WsReloadState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        let mut rx = state.update.subscribe();
        let reload_watcher = tokio::spawn(async move {
            loop {
                rx.recv().await.unwrap();
                // ignore the error
                if socket
                    .send(Message::Text(String::from("reload")))
                    .await
                    .is_err()
                {
                    break;
                }

                // flush the errors after recompling
                rx = rx.resubscribe();
            }
        });

        reload_watcher.await.unwrap();
    })
}

fn build(config: &CrateConfig, reload_tx: &Sender<()>) -> Result<BuildResult> {
    let result = builder::build(config, true)?;
    // change the websocket reload state to true;
    // the page will auto-reload.
    if config
        .dioxus_config
        .web
        .watcher
        .reload_html
        .unwrap_or(false)
    {
        let _ = Serve::regen_dev_page(config);
    }
    let _ = reload_tx.send(());
    Ok(result)
}
