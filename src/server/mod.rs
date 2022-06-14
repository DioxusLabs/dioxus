use axum::{
    body::{Full, HttpBody},
    extract::{ws::Message, Extension, TypedHeader, WebSocketUpgrade},
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use notify::{RecommendedWatcher, Watcher};

use std::{path::PathBuf, sync::Arc};
use tower::ServiceBuilder;
use tower_http::services::fs::{ServeDir, ServeFileSystemResponseBody};

use crate::{builder, serve::Serve, CrateConfig, Result};
use tokio::sync::broadcast;

#[cfg(feature = "hot_reload")]
mod hot_reload;
#[cfg(feature = "hot_reload")]
use hot_reload::*;

struct WsReloadState {
    update: broadcast::Sender<String>,
    #[cfg(feature = "hot_reload")]
    last_file_rebuild: Arc<Mutex<FileMap>>,
    watcher_config: CrateConfig,
}

#[cfg(feature = "hot_reload")]
pub async fn startup(config: CrateConfig) -> Result<()> {
    log::info!("🚀 Starting development server...");

    let dist_path = config.out_dir.clone();
    let (reload_tx, _) = broadcast::channel(100);
    let last_file_rebuild = Arc::new(Mutex::new(FileMap::new(config.crate_dir.clone())));
    let hot_reload_tx = broadcast::channel(100).0;
    let hot_reload_state = Arc::new(HotReloadState {
        messages: hot_reload_tx.clone(),
        update: reload_tx.clone(),
        last_file_rebuild: last_file_rebuild.clone(),
        watcher_config: config.clone(),
    });

    let crate_dir = config.crate_dir.clone();
    let ws_reload_state = Arc::new(WsReloadState {
        update: reload_tx.clone(),

        last_file_rebuild: last_file_rebuild.clone(),
        watcher_config: config.clone(),
    });

    let mut last_update_time = chrono::Local::now().timestamp();

    // file watcher: check file change
    let allow_watch_path = config
        .dioxus_config
        .web
        .watcher
        .watch_path
        .clone()
        .unwrap_or_else(|| vec![PathBuf::from("src")]);

    let mut watcher = RecommendedWatcher::new(move |evt: notify::Result<notify::Event>| {
        if let Ok(evt) = evt {
            for path in evt.paths {
                let mut file = File::open(path.clone()).unwrap();
                if path.extension().map(|p| p.to_str()).flatten() != Some("rs") {
                    continue;
                }
                let mut src = String::new();
                file.read_to_string(&mut src).expect("Unable to read file");
                if src.is_empty() {
                    continue;
                }
                // find changes to the rsx in the file
                if let Ok(syntax) = syn::parse_file(&src) {
                    let mut last_file_rebuild = last_file_rebuild.lock().unwrap();
                    if let Some(old_str) = last_file_rebuild.map.get(&path) {
                        if let Ok(old) = syn::parse_file(&old_str) {
                            match find_rsx(&syntax, &old) {
                                DiffResult::CodeChanged => {
                                    log::info!("reload required");
                                    if chrono::Local::now().timestamp() > last_update_time {
                                        let _ = reload_tx.send("reload".into());
                                        last_update_time = chrono::Local::now().timestamp();
                                    }
                                }
                                DiffResult::RsxChanged(changed) => {
                                    log::info!("reloading rsx");
                                    for (old, new) in changed.into_iter() {
                                        if let Some(hr) = get_min_location(
                                            &path.strip_prefix(&crate_dir).unwrap().to_path_buf(),
                                            old.to_token_stream(),
                                        ) {
                                            let rsx = new.to_string();
                                            let _ = hot_reload_tx.send(SetRsxMessage {
                                                location: hr,
                                                new_text: rsx,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // if this is a new file, rebuild the project
                        *last_file_rebuild = FileMap::new(crate_dir.clone());
                    }
                }
            }
        }
    })
    .unwrap();

    for sub_path in allow_watch_path {
        watcher
            .watch(
                &config.crate_dir.join(sub_path),
                notify::RecursiveMode::Recursive,
            )
            .unwrap();
    }

    // start serve dev-server at 0.0.0.0:8080
    let port = "8080";
    log::info!("📡 Dev-Server is started at: http://127.0.0.1:{}/", port);

    let file_service_config = config.clone();
    let file_service = ServiceBuilder::new()
        .and_then(
            |response: Response<ServeFileSystemResponseBody>| async move {
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
        .service(ServeDir::new((&config.crate_dir).join(&dist_path)));

    let router = Router::new()
        .route("/_dioxus/ws", get(ws_handler))
        .fallback(
            get_service(file_service).handle_error(|error: std::io::Error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            }),
        );

    let router = router
        .route("/_dioxus/hot_reload", get(hot_reload_handler))
        .layer(Extension(ws_reload_state))
        .layer(Extension(hot_reload_state));

    axum::Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

#[cfg(not(feature = "hot_reload"))]
pub async fn startup(config: CrateConfig) -> Result<()> {
    log::info!("🚀 Starting development server...");

    let dist_path = config.out_dir.clone();

    let (reload_tx, _) = broadcast::channel(100);

    let ws_reload_state = Arc::new(WsReloadState {
        update: reload_tx.clone(),
        watcher_config: config.clone(),
    });

    let mut last_update_time = chrono::Local::now().timestamp();

    // file watcher: check file change
    let allow_watch_path = config
        .dioxus_config
        .web
        .watcher
        .watch_path
        .clone()
        .unwrap_or_else(|| vec![PathBuf::from("src")]);

    let mut watcher = RecommendedWatcher::new(move |_: notify::Result<notify::Event>| {
        log::info!("reload required");
        if chrono::Local::now().timestamp() > last_update_time {
            let _ = reload_tx.send("reload".into());
            last_update_time = chrono::Local::now().timestamp();
        }
    })
    .unwrap();

    for sub_path in allow_watch_path {
        watcher
            .watch(
                &config.crate_dir.join(sub_path),
                notify::RecursiveMode::Recursive,
            )
            .unwrap();
    }

    // start serve dev-server at 0.0.0.0:8080
    let port = "8080";
    log::info!("📡 Dev-Server is started at: http://127.0.0.1:{}/", port);

    let file_service_config = config.clone();
    let file_service = ServiceBuilder::new()
        .and_then(
            |response: Response<ServeFileSystemResponseBody>| async move {
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
        .service(ServeDir::new((&config.crate_dir).join(&dist_path)));

    let router = Router::new()
        .route("/_dioxus/ws", get(ws_handler))
        .fallback(
            get_service(file_service).handle_error(|error: std::io::Error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            }),
        )
        .layer(Extension(ws_reload_state));

    axum::Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    _: Option<TypedHeader<headers::UserAgent>>,
    Extension(state): Extension<Arc<WsReloadState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        let mut rx = state.update.subscribe();
        let reload_watcher = tokio::spawn(async move {
            loop {
                let v = rx.recv().await.unwrap();
                if v == "reload" {
                    log::info!("Start to rebuild project...");
                    if builder::build(&state.watcher_config).is_ok() {
                        // change the websocket reload state to true;
                        // the page will auto-reload.
                        if state
                            .watcher_config
                            .dioxus_config
                            .web
                            .watcher
                            .reload_html
                            .unwrap_or(false)
                        {
                            let _ = Serve::regen_dev_page(&state.watcher_config);
                        }
                        #[cfg(feature = "hot_reload")]
                        {
                            let mut write = state.last_file_rebuild.lock().unwrap();
                            *write = FileMap::new(state.watcher_config.crate_dir.clone());
                        }
                    }
                    // ignore the error
                    if socket
                        .send(Message::Text(String::from("reload")))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        });

        reload_watcher.await.unwrap();
    })
}
