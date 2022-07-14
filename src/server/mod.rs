use axum::{
    body::{Full, HttpBody},
    extract::{ws::Message, Extension, TypedHeader, WebSocketUpgrade},
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use dioxus::rsx_interpreter::SetRsxMessage;
use notify::{RecommendedWatcher, Watcher};
use syn::spanned::Spanned;

use std::{path::PathBuf, sync::Arc};
use tower::ServiceBuilder;
use tower_http::services::fs::{ServeDir, ServeFileSystemResponseBody};

use crate::{builder, serve::Serve, CrateConfig, Result};
use tokio::sync::broadcast;

mod hot_reload;
use hot_reload::*;

pub struct BuildManager {
    config: CrateConfig,
    reload_tx: broadcast::Sender<()>,
}

impl BuildManager {
    fn build(&self) -> Result<()> {
        log::info!("Start to rebuild project...");
        builder::build(&self.config)?;
        // change the websocket reload state to true;
        // the page will auto-reload.
        if self
            .config
            .dioxus_config
            .web
            .watcher
            .reload_html
            .unwrap_or(false)
        {
            let _ = Serve::regen_dev_page(&self.config);
        }
        let _ = self.reload_tx.send(());
        Ok(())
    }
}

struct WsReloadState {
    update: broadcast::Sender<()>,
}

pub async fn startup(config: CrateConfig) -> Result<()> {
    if config.hot_reload {
        startup_hot_reload(config).await?
    } else {
        startup_default(config).await?
    }
    Ok(())
}

pub async fn startup_hot_reload(config: CrateConfig) -> Result<()> {
    log::info!("🚀 Starting development server...");

    let dist_path = config.out_dir.clone();
    let (reload_tx, _) = broadcast::channel(100);
    let last_file_rebuild = Arc::new(Mutex::new(FileMap::new(config.crate_dir.clone())));
    let build_manager = Arc::new(BuildManager {
        config: config.clone(),
        reload_tx: reload_tx.clone(),
    });
    let hot_reload_tx = broadcast::channel(100).0;
    let hot_reload_state = Arc::new(HotReloadState {
        messages: hot_reload_tx.clone(),
        build_manager: build_manager.clone(),
        last_file_rebuild: last_file_rebuild.clone(),
        watcher_config: config.clone(),
    });

    let crate_dir = config.crate_dir.clone();
    let ws_reload_state = Arc::new(WsReloadState {
        update: reload_tx.clone(),
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
        if chrono::Local::now().timestamp() > last_update_time {
            // Give time for the change to take effect before reading the file
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Ok(evt) = evt {
                let mut messages = Vec::new();
                let mut needs_rebuild = false;
                for path in evt.paths {
                    let mut file = File::open(path.clone()).unwrap();
                    if path.extension().map(|p| p.to_str()).flatten() != Some("rs") {
                        continue;
                    }
                    let mut src = String::new();
                    file.read_to_string(&mut src).expect("Unable to read file");
                    // find changes to the rsx in the file
                    if let Ok(syntax) = syn::parse_file(&src) {
                        let mut last_file_rebuild = last_file_rebuild.lock().unwrap();
                        if let Some(old_str) = last_file_rebuild.map.get(&path) {
                            if let Ok(old) = syn::parse_file(&old_str) {
                                match find_rsx(&syntax, &old) {
                                    DiffResult::CodeChanged => {
                                        needs_rebuild = true;
                                        last_file_rebuild.map.insert(path, src);
                                    }
                                    DiffResult::RsxChanged(changed) => {
                                        log::info!("reloading rsx");
                                        for (old, new) in changed.into_iter() {
                                            let hr = get_location(
                                                &path.to_path_buf(),
                                                old.to_token_stream(),
                                            );
                                            // get the original source code to preserve whitespace
                                            let span = new.span();
                                            let start = span.start();
                                            let end = span.end();
                                            let mut lines: Vec<_> = src
                                                .lines()
                                                .skip(start.line - 1)
                                                .take(end.line - start.line + 1)
                                                .collect();
                                            if let Some(first) = lines.first_mut() {
                                                *first = first.split_at(start.column).1;
                                            }
                                            if let Some(last) = lines.last_mut() {
                                                // if there is only one line the start index of last line will be the start of the rsx!, not the start of the line
                                                if start.line == end.line {
                                                    *last =
                                                        last.split_at(end.column - start.column).0;
                                                } else {
                                                    *last = last.split_at(end.column).0;
                                                }
                                            }
                                            let rsx = lines.join("\n");
                                            messages.push(SetRsxMessage {
                                                location: hr,
                                                new_text: rsx,
                                            });
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
                if needs_rebuild {
                    log::info!("reload required");
                    if let Err(err) = build_manager.build() {
                        log::error!("{}", err);
                    }
                }
                if !messages.is_empty() {
                    let _ = hot_reload_tx.send(SetManyRsxMessage(messages));
                }
            }
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

pub async fn startup_default(config: CrateConfig) -> Result<()> {
    log::info!("🚀 Starting development server...");

    let dist_path = config.out_dir.clone();

    let (reload_tx, _) = broadcast::channel(100);

    let build_manager = BuildManager {
        config: config.clone(),
        reload_tx: reload_tx.clone(),
    };

    let ws_reload_state = Arc::new(WsReloadState {
        update: reload_tx.clone(),
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
            match build_manager.build() {
                Ok(_) => last_update_time = chrono::Local::now().timestamp(),
                Err(e) => log::error!("{}", e),
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
