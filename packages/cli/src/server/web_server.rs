use crate::Result;
use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket},
        Extension, WebSocketUpgrade,
    },
    http::{
        self,
        header::{HeaderName, HeaderValue, CACHE_CONTROL, EXPIRES, PRAGMA},
        Method, Response, StatusCode,
    },
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use dioxus_cli_config::CrateConfig;
use dioxus_cli_config::WebHttpsConfig;
use dioxus_rsx::HotReload;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
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

use crate::cfg::ConfigOptsServe;

pub struct DevServer {
    pub sockets: Vec<WebSocket>,
    pub ip: IpAddr,
    pub new_socket: UnboundedReceiver<WebSocket>,
    pub server_task: JoinHandle<Result<()>>,
}

impl DevServer {
    pub async fn start(opts: &ConfigOptsServe, cfg: &CrateConfig) -> Self {
        let (tx, rx) = futures_channel::mpsc::unbounded();

        let router = setup_router(&cfg, tx).await.unwrap();
        let port = opts.server_arguments.port;
        let start_browser = opts.open.unwrap_or(false);

        let ip = opts
            .server_arguments
            .addr
            .or_else(get_ip)
            .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)));

        // #[cfg(feature = "plugin")]
        // crate::plugin::PluginManager::on_serve_start(&cfg)?;

        let addr: SocketAddr = SocketAddr::from((ip, port));

        // HTTPS
        // Before console info so it can stop if mkcert isn't installed or fails
        // todo: this is the only async thing here - might be nice to
        let rustls = get_rustls(&cfg).await.unwrap();

        // Open the browser
        if start_browser {
            open_browser(&cfg, ip, port, rustls.is_some());
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

    pub fn update(&mut self, cfg: &ConfigOptsServe, crate_config: &CrateConfig) {}

    pub async fn send_hotreload(&mut self, reload: HotReload) {
        // to our connected clients, send the changes to the sockets
        for socket in self.sockets.iter_mut() {
            let msg = serde_json::to_string(&reload).unwrap();
            if socket.send(Message::Text(msg)).await.is_err() {
                // the socket is likely disconnected, we should remove it
            }
        }
    }

    pub async fn send_binary_patch(&mut self, patch: Vec<u8>) {
        // to our connected clients, send the changes to the sockets
    }

    pub async fn wait(&mut self) {
        todo!()
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
pub async fn setup_router(config: &CrateConfig, tx: UnboundedSender<WebSocket>) -> Result<Router> {
    let mut router = Router::new();

    // Setup cors
    router = router.layer(
        CorsLayer::new()
            // allow `GET` and `POST` when accessing the resource
            .allow_methods([Method::GET, Method::POST])
            // allow requests from any origin
            .allow_origin(Any)
            .allow_headers(Any),
    );

    // Setup proxy for the /api/ endpoint
    for proxy_config in config.dioxus_config.web.proxy.iter() {
        todo!("Configure proxy");
        // router = super::proxy::add_proxy(router, &proxy_config)?;
    }

    // Setup websocket endpoint
    // todo: we used to have multiple routes here but we just need the one
    router = router.layer(Extension(tx));
    router = router.nest(
        "/_dioxus",
        Router::new().route(
            "/ws",
            get(
                // Simply bounce the websocket handle up to the webserver handle
                |ws: WebSocketUpgrade, ext: Extension<UnboundedSender<WebSocket>>| async move {
                    ws.on_upgrade(move |socket| async move { _ = ext.0.unbounded_send(socket) })
                },
            ),
        ),
    );

    // Route file service to output the .wasm and assets
    router = router.fallback(builder_serve_dir(config));

    // Setup base path redirection
    if let Some(base_path) = config.dioxus_config.web.app.base_path.clone() {
        let base_path = format!("/{}", base_path.trim_matches('/'));
        router = Router::new()
            .nest(&base_path, router)
            .fallback(get(move || async move {
                format!("Outside of the base path: {}", base_path)
            }));
    }

    Ok(router)
}

fn builder_serve_dir(cfg: &CrateConfig) -> axum::routing::MethodRouter {
    const CORS_UNSAFE: (HeaderValue, HeaderValue) = (
        HeaderValue::from_static("unsafe-none"),
        HeaderValue::from_static("unsafe-none"),
    );

    const CORS_REQUIRE: (HeaderValue, HeaderValue) = (
        HeaderValue::from_static("require-corp"),
        HeaderValue::from_static("same-origin"),
    );

    let (coep, coop) = match cfg.cross_origin_policy {
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

fn no_cache(cfg: CrateConfig, response: Response<ServeFileSystemResponseBody>) -> Response<Body> {
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
pub async fn get_rustls(config: &CrateConfig) -> Result<Option<RustlsConfig>> {
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
pub(crate) fn open_browser(config: &CrateConfig, ip: IpAddr, port: u16, https: bool) {
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

mod old {

    // /// Handle websockets
    // async fn ws_handler(
    //     ws: WebSocketUpgrade,
    //     Extension(state): Extension<HotReloadReceiver>,
    // ) -> impl IntoResponse {
    //     ws.on_upgrade(move |socket| ws_reload_handler(socket, state))
    // }

    // /// Handles full-reloads (e.g. page refresh).
    // async fn ws_reload_handler(mut socket: WebSocket, mut state: HotReloadReceiver) {
    //     let mut rx = state.reload.subscribe();

    //     loop {
    //         rx.recv().await.unwrap();

    //         // We need to clear the cached templates because we are doing a "fresh" build
    //         // and the templates may force the page to a state before the full reload.
    //         state.clear_all_modified_templates();

    //         // ignore the error
    //         let _ = socket.send(Message::Text(String::from("reload"))).await;
    //         tracing::info!("forcing reload");

    //         // flush the errors after recompling
    //         rx = rx.resubscribe();
    //     }
    // }

    // /// State that is shared between the websocket and the hot reloading watcher
    // #[derive(Clone)]
    // pub struct HotReloadReceiver {
    //     /// Hot reloading messages sent from the client
    //     // NOTE: We use a send broadcast channel to allow clones
    //     messages: broadcast::Sender<HotReloadMsg>,

    //     /// Rebuilds sent from the client
    //     reload: broadcast::Sender<()>,

    //     /// Any template updates that have happened since the last full render
    //     template_updates: SharedTemplateUpdates,
    // }

    // impl HotReloadReceiver {
    //     /// Create a new [`HotReloadReceiver`]
    //     pub fn new() -> Self {
    //         Self::default()
    //     }
    // }

    // impl Default for HotReloadReceiver {
    //     fn default() -> Self {
    //         Self {
    //             messages: broadcast::channel(100).0,
    //             reload: broadcast::channel(100).0,
    //             template_updates: Default::default(),
    //         }
    //     }
    // }

    // type SharedTemplateUpdates = Arc<Mutex<HashMap<&'static str, Template>>>;

    // impl HotReloadReceiver {
    //     /// Find all templates that have been updated since the last full render
    //     pub fn all_modified_templates(&self) -> Vec<Template> {
    //         self.template_updates
    //             .lock()
    //             .unwrap()
    //             .values()
    //             .cloned()
    //             .collect()
    //     }

    //     /// Clears the cache of modified templates.
    //     pub fn clear_all_modified_templates(&mut self) {
    //         self.template_updates.lock().unwrap().clear();
    //     }

    //     /// Send a hot reloading message to the client
    //     pub fn send_message(&self, msg: HotReloadMsg) {
    //         // Before we send the message, update the list of changed templates
    //         if let HotReloadMsg::Update {
    //             templates,
    //             changed_strings,
    //             assets,
    //         } = &msg
    //         {
    //             let mut template_updates = self.template_updates.lock().unwrap();
    //             for template in templates {
    //                 template_updates.insert(template.name, template.clone());
    //             }
    //         }
    //         if let Err(err) = self.messages.send(msg) {
    //             tracing::error!("Failed to send hot reload message: {}", err);
    //         }
    //     }

    //     /// Subscribe to hot reloading messages
    //     pub fn subscribe(&self) -> broadcast::Receiver<HotReloadMsg> {
    //         self.messages.subscribe()
    //     }

    //     /// Reload the website
    //     pub fn reload(&self) {
    //         self.reload.send(()).unwrap();
    //     }
    // }

    // pub async fn hot_reload_handler(
    //     ws: WebSocketUpgrade,
    //     Extension(state): Extension<HotReloadReceiver>,
    // ) -> impl IntoResponse {
    //     ws.on_upgrade(|socket| async move {
    //         let err = hotreload_loop(socket, state).await;

    //         if let Err(err) = err {
    //             tracing::error!("Hotreload receiver failed: {}", err);
    //         }
    //     })
    // }

    // async fn hotreload_loop(
    //     mut socket: WebSocket,
    //     state: HotReloadReceiver,
    // ) -> Result<(), axum::Error> {
    //     tracing::info!("🔥 Hot Reload WebSocket connected");

    //     let mut rx = state.messages.subscribe();

    //     // update any rsx calls that changed before the websocket connected.
    //     // These templates will be sent down immediately so the page is in sync with the hotreloaded version
    //     // The compiled version will be different from the one we actually want to present
    //     for template in state.all_modified_templates() {
    //         socket
    //             .send(Message::Text(serde_json::to_string(&template).unwrap()))
    //             .await?;
    //     }

    //     loop {
    //         let msg = {
    //             // Poll both the receiver and the socket
    //             //
    //             // This shuts us down if the connection is closed.
    //             let mut _socket = socket.recv().fuse();
    //             let mut _rx = rx.recv().fuse();

    //             pin_mut!(_socket, _rx);

    //             let msg = futures_util::select! {
    //                 msg = _rx => msg,
    //                 e = _socket => {
    //                     if let Some(Err(e)) = e {
    //                         tracing::info!("🔥 Hot Reload WebSocket Error: {}", e);
    //                     } else {
    //                         tracing::info!("🔥 Hot Reload WebSocket Closed");
    //                     }
    //                     break;
    //                 },
    //             };

    //             let Ok(msg) = msg else { break };

    //             match msg {
    //                 HotReloadMsg::Update {
    //                     templates,
    //                     changed_strings,
    //                     assets,
    //                 } => {
    //                     // todo: fix the assets bug
    //                     Message::Text(serde_json::to_string(&templates).unwrap())
    //                 }
    //                 // HotReloadMsg::Update(template) => {
    //                 //     Message::Text(serde_json::to_string(&template).unwrap())
    //                 // }
    //                 // HotReloadMsg::UpdateAsset(asset) => {
    //                 //     Message::Text(format!("reload-asset: {}", asset.display()))
    //                 // }
    //                 HotReloadMsg::Shutdown => {
    //                     tracing::info!("🔥 Hot Reload WebSocket shutting down");
    //                     break;
    //                 }
    //             }
    //         };

    //         socket.send(msg).await?;
    //     }

    //     Ok(())
    // }

    // pub(crate) fn forward_cli_hot_reload() -> HotReloadReceiver {
    //     let hot_reload_state = HotReloadReceiver::default();

    //     // Hot reloading can be expensive to start so we spawn a new thread
    //     std::thread::spawn({
    //         let hot_reload_state = hot_reload_state.clone();
    //         move || {
    //             crate::connect(move |msg| hot_reload_state.send_message(msg));
    //         }
    //     });

    //     hot_reload_state
    // }

    //     #[derive(Clone)]
    // pub struct HotReloadState {
    //     /// The receiver for hot reload messages
    //     pub receiver: HotReloadReceiver,

    //     /// The file map that tracks the state of the projecta
    //     pub file_map: Option<SharedFileMap>,
    // }

    // type SharedFileMap = Arc<Mutex<FileMap<HtmlCtx>>>;

    // /// Sets up a file watcher.
    // ///
    // /// Will attempt to hotreload HTML, RSX (.rs), and CSS
    // async fn setup_file_watcher<F: Fn() -> Result<BuildResult> + Send + 'static>(
    //     build_with: F,
    //     config: &CrateConfig,
    //     web_info: Option<WebServerInfo>,
    //     hot_reload: HotReloadState,
    // ) -> Result<RecommendedWatcher> {
    //     let mut last_update_time = chrono::Local::now().timestamp();

    //     // file watcher: check file change
    //     let mut allow_watch_path = config.dioxus_config.web.watcher.watch_path.clone();

    //     // Extend the watch path to include the assets directory - this is so we can hotreload CSS and other assets by default
    //     allow_watch_path.push(config.dioxus_config.application.asset_dir.clone());

    //     // Extend the watch path to include Cargo.toml and Dioxus.toml
    //     allow_watch_path.push("Cargo.toml".to_string().into());
    //     allow_watch_path.push("Dioxus.toml".to_string().into());
    //     allow_watch_path.dedup();

    //     // Create the file watcher
    //     let mut watcher = notify::recommended_watcher({
    //         let watcher_config = config.clone();
    //         move |info: notify::Result<notify::Event>| {
    //             let Ok(e) = info else {
    //                 return;
    //             };
    //             watch_event(
    //                 e,
    //                 &mut last_update_time,
    //                 &hot_reload,
    //                 &watcher_config,
    //                 &build_with,
    //                 &web_info,
    //             );
    //         }
    //     })
    //     .expect("Failed to create file watcher - please ensure you have the required permissions to watch the specified directories.");

    //     // Watch the specified paths
    //     for sub_path in allow_watch_path {
    //         let path = &config.crate_dir.join(sub_path);
    //         let mode = notify::RecursiveMode::Recursive;

    //         if let Err(err) = watcher.watch(path, mode) {
    //             tracing::warn!("Failed to watch path: {}", err);
    //         }
    //     }

    //     Ok(watcher)
    // }

    // fn watch_event<F>(
    //     event: notify::Event,
    //     last_update_time: &mut i64,
    //     hot_reload: &HotReloadState,
    //     config: &CrateConfig,
    //     build_with: &F,
    //     web_info: &Option<WebServerInfo>,
    // ) where
    //     F: Fn() -> Result<BuildResult> + Send + 'static,
    // {
    //     // Ensure that we're tracking only modifications
    //     if !matches!(
    //         event.kind,
    //         notify::EventKind::Create(_) | notify::EventKind::Remove(_) | notify::EventKind::Modify(_)
    //     ) {
    //         return;
    //     }

    //     // Ensure that we're not rebuilding too frequently
    //     if chrono::Local::now().timestamp() <= *last_update_time {
    //         return;
    //     }

    //     // By default we want to not do a full rebuild, and instead let the hot reload system invalidate it
    //     let mut needs_full_rebuild = false;

    //     if let Some(file_map) = &hot_reload.file_map {
    //         hotreload_files(
    //             hot_reload,
    //             file_map,
    //             &mut needs_full_rebuild,
    //             &event,
    //             config,
    //         );
    //     }

    //     if needs_full_rebuild {
    //         full_rebuild(build_with, last_update_time, config, event, web_info);
    //     }
    // }

    // fn full_rebuild<F>(
    //     build_with: &F,
    //     last_update_time: &mut i64,
    //     config: &CrateConfig,
    //     event: notify::Event,
    //     web_info: &Option<WebServerInfo>,
    // ) where
    //     F: Fn() -> Result<BuildResult> + Send + 'static,
    // {
    //     match build_with() {
    //         Ok(res) => {
    //             *last_update_time = chrono::Local::now().timestamp();

    //             #[allow(clippy::redundant_clone)]
    //             print_console_info(
    //                 config,
    //                 PrettierOptions {
    //                     changed: event.paths.clone(),
    //                     warnings: res.warnings,
    //                     elapsed_time: res.elapsed_time,
    //                 },
    //                 web_info.clone(),
    //             );
    //         }
    //         Err(e) => {
    //             *last_update_time = chrono::Local::now().timestamp();
    //             tracing::error!("{:?}", e);
    //         }
    //     }
    // }

    // fn hotreload_files(
    //     hot_reload: &HotReloadState,
    //     file_map: &SharedFileMap,
    //     needs_full_rebuild: &mut bool,
    //     event: &notify::Event,
    //     config: &CrateConfig,
    // ) {
    //     // find changes to the rsx in the file
    //     let mut rsx_file_map = file_map.lock().unwrap();
    //     let mut messages: Vec<HotReloadMsg> = Vec::new();

    //     for path in &event.paths {
    //         // Attempt to hotreload this file
    //         let is_potentially_reloadable = hotreload_file(
    //             path,
    //             config,
    //             &rsx_file_map,
    //             &mut messages,
    //             needs_full_rebuild,
    //         );

    //         // If the file was not hotreloaded, continue
    //         if is_potentially_reloadable.is_none() {
    //             continue;
    //         }

    //         // If the file was hotreloaded, update the file map in place
    //         match rsx_file_map.update_rsx(path, &config.crate_dir) {
    //             Ok(UpdateResult::UpdatedRsx {
    //                 templates,
    //                 changed_lits: changed_strings,
    //             }) => {
    //                 messages.push(HotReloadMsg::Update {
    //                     templates,
    //                     changed_strings,
    //                     assets: vec![],
    //                 });
    //             }

    //             // If the file was not updated, we need to do a full rebuild
    //             Ok(UpdateResult::NeedsRebuild) => {
    //                 tracing::trace!("Needs full rebuild because file changed: {:?}", path);
    //                 *needs_full_rebuild = true;
    //             }

    //             // Not necessarily a fatal error, but we should log it
    //             Err(err) => tracing::error!("{}", err),
    //         }
    //     }

    //     // If full rebuild, extend the file map with the new file map
    //     // This will wipe away any previous cached changed templates
    //     if *needs_full_rebuild {
    //         // Reset the file map to the new state of the project
    //         let FileMapBuildResult {
    //             map: new_file_map,
    //             errors,
    //         } = FileMap::<HtmlCtx>::create(config.crate_dir.clone()).unwrap();

    //         for err in errors {
    //             tracing::error!("{}", err);
    //         }

    //         *rsx_file_map = new_file_map;

    //         return;
    //     }

    //     for msg in messages {
    //         hot_reload.receiver.send_message(msg);
    //     }
    // }

    // fn hotreload_file(
    //     path: &Path,
    //     config: &CrateConfig,
    //     rsx_file_map: &std::sync::MutexGuard<'_, FileMap<HtmlCtx>>,
    //     messages: &mut Vec<HotReloadMsg>,
    //     needs_full_rebuild: &mut bool,
    // ) -> Option<()> {
    //     // for various assets that might be linked in, we just try to hotreloading them forcefully
    //     // That is, unless they appear in an include! macro, in which case we need to a full rebuild....
    //     let ext = path.extension().and_then(|v| v.to_str())?;

    //     // Workaround for notify and vscode-like editor:
    //     // when edit & save a file in vscode, there will be two notifications,
    //     // the first one is a file with empty content.
    //     // filter the empty file notification to avoid false rebuild during hot-reload
    //     if let Ok(metadata) = fs::metadata(path) {
    //         if metadata.len() == 0 {
    //             return None;
    //         }
    //     }

    //     // If the extension is a backup file, or a hidden file, ignore it completely (no rebuilds)
    //     if is_backup_file(path) {
    //         tracing::trace!("Ignoring backup file: {:?}", path);
    //         return None;
    //     }

    //     // Attempt to hotreload css in the asset directory
    //     // Currently no other assets are hotreloaded, but in theory we could hotreload pngs/jpegs, etc
    //     //
    //     // All potential hotreloadable mime types:
    //     // "bin" |"css" | "csv" | "html" | "ico" | "js" | "json" | "jsonld" | "mjs" | "rtf" | "svg" | "mp4"
    //     if ext == "css" {
    //         let asset_dir = config
    //             .crate_dir
    //             .join(&config.dioxus_config.application.asset_dir);

    //         // Only if the CSS is in the asset directory, and we're tracking it, do we hotreload it
    //         // Otherwise, we need to do a full rebuild since the user might be doing an include_str! on it
    //         if attempt_css_reload(path, asset_dir, rsx_file_map, config, messages).is_none() {
    //             *needs_full_rebuild = true;
    //         }

    //         return None;
    //     }

    //     // If the file is not rsx or css and we've already not needed a full rebuild, return
    //     if ext != "rs" && ext != "css" {
    //         *needs_full_rebuild = true;
    //         return None;
    //     }

    //     Some(())
    // }

    // fn attempt_css_reload(
    //     path: &Path,
    //     asset_dir: PathBuf,
    //     rsx_file_map: &std::sync::MutexGuard<'_, FileMap<HtmlCtx>>,
    //     config: &CrateConfig,
    //     messages: &mut Vec<HotReloadMsg>,
    // ) -> Option<()> {
    //     // If the path is not in the asset directory, return
    //     if !path.starts_with(asset_dir) {
    //         return None;
    //     }

    //     // Get the local path of the asset (ie var.css or some_dir/var.css as long as the dir is under the asset dir)
    //     let local_path = local_path_of_asset(path)?;

    //     // Make sure we're actually tracking this asset...
    //     _ = rsx_file_map.is_tracking_asset(&local_path)?;

    //     // copy the asset over to the output directory
    //     // todo this whole css hotreloading should be less hacky and more robust
    //     _ = fs_extra::copy_items(
    //         &[path],
    //         config.out_dir(),
    //         &CopyOptions::new().overwrite(true),
    //     );

    //     messages.push(HotReloadMsg::Update {
    //         templates: Default::default(),
    //         changed_strings: Default::default(),
    //         assets: vec![local_path],
    //     });

    //     Some(())
    // }

    // fn local_path_of_asset(path: &Path) -> Option<PathBuf> {
    //     path.file_name()?.to_str()?.to_string().parse().ok()
    // }

    // pub(crate) trait Platform {
    //     fn start(
    //         config: &CrateConfig,
    //         serve: &ConfigOptsServe,
    //         env: Vec<(String, String)>,
    //     ) -> Result<Self>
    //     where
    //         Self: Sized;
    //     fn rebuild(
    //         &mut self,
    //         config: &CrateConfig,
    //         serve: &ConfigOptsServe,
    //         env: Vec<(String, String)>,
    //     ) -> Result<BuildResult>;
    // }

    // fn is_backup_file(path: &Path) -> bool {
    //     // If there's a tilde at the end of the file, it's a backup file
    //     if let Some(name) = path.file_name() {
    //         if let Some(name) = name.to_str() {
    //             if name.ends_with('~') {
    //                 return true;
    //             }
    //         }
    //     }

    //     // if the file is hidden, it's a backup file
    //     if let Some(name) = path.file_name() {
    //         if let Some(name) = name.to_str() {
    //             if name.starts_with('.') {
    //                 return true;
    //             }
    //         }
    //     }

    //     false
    // }

    // #[test]
    // fn test_is_backup_file() {
    //     assert!(is_backup_file(&PathBuf::from("examples/test.rs~")));
    //     assert!(is_backup_file(&PathBuf::from("examples/.back")));
    //     assert!(is_backup_file(&PathBuf::from("test.rs~")));
    //     assert!(is_backup_file(&PathBuf::from(".back")));

    //     assert!(!is_backup_file(&PathBuf::from("val.rs")));
    //     assert!(!is_backup_file(&PathBuf::from(
    //         "/Users/jonkelley/Development/Tinkering/basic_05_example/src/lib.rs"
    //     )));
    //     assert!(!is_backup_file(&PathBuf::from("exmaples/val.rs")));
    // }
}
