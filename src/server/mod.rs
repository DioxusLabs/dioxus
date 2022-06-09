use axum::{
    body::{Full, HttpBody},
    extract::{ws::Message, Extension, TypedHeader, WebSocketUpgrade},
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use notify::{RecommendedWatcher, Watcher};
use std::{fs::File, io::Read};
use syn::__private::ToTokens;

use std::{path::PathBuf, sync::Arc};
use tower::ServiceBuilder;
use tower_http::services::fs::{ServeDir, ServeFileSystemResponseBody};

use crate::{builder, find_rsx, serve::Serve, CrateConfig, Result};
use tokio::sync::broadcast;

use std::collections::HashMap;

use dioxus_rsx_interpreter::{error::RecompileReason, CodeLocation, SetRsxMessage};

struct WsReloadState {
    update: broadcast::Sender<String>,
}

struct HotReloadState {
    messages: broadcast::Sender<SetRsxMessage>,
}

pub async fn startup(config: CrateConfig) -> Result<()> {
    log::info!("🚀 Starting development server...");

    let dist_path = config.out_dir.clone();

    let (reload_tx, _) = broadcast::channel(100);

    let ws_reload_state = Arc::new(WsReloadState {
        update: reload_tx.clone(),
    });

    let hot_reload_tx = broadcast::channel(100).0;
    let hot_reload_state = Arc::new(HotReloadState {
        messages: hot_reload_tx.clone(),
    });

    let mut last_update_time = chrono::Local::now().timestamp();

    // file watcher: check file change
    let watcher_conf = config.clone();
    let mut old_files_parsed: HashMap<String, String> = HashMap::new();
    let allow_watch_path = config
        .dioxus_config
        .web
        .watcher
        .watch_path
        .clone()
        .unwrap_or_else(|| vec![PathBuf::from("src")]);

    let crate_dir = config.crate_dir.clone();

    let mut watcher = RecommendedWatcher::new(move |evt: notify::Result<notify::Event>| {
        if let Ok(evt) = evt {
            if let notify::EventKind::Modify(_) = evt.kind {
                for path in evt.paths {
                    log::info!("File changed: {}", path.display());
                    let mut file = File::open(path.clone()).unwrap();
                    let mut src = String::new();
                    file.read_to_string(&mut src).expect("Unable to read file");
                    if src.is_empty() {
                        continue;
                    }
                    if let Ok(syntax) = syn::parse_file(&src) {
                        if let Some(old_str) = old_files_parsed.get(path.to_str().unwrap()) {
                            if let Ok(old) = syn::parse_file(&old_str) {
                                match find_rsx(&syntax, &old) {
                                    crate::DiffResult::CodeChanged => {
                                        log::info!("reload required");
                                        if chrono::Local::now().timestamp() > last_update_time {
                                            log::info!("Start to rebuild project...");
                                            if builder::build(&watcher_conf).is_ok() {
                                                // change the websocket reload state to true;
                                                // the page will auto-reload.
                                                if watcher_conf
                                                    .dioxus_config
                                                    .web
                                                    .watcher
                                                    .reload_html
                                                    .unwrap_or(false)
                                                {
                                                    let _ = Serve::regen_dev_page(&watcher_conf);
                                                }
                                                let _ = reload_tx.send("reload".into());
                                                old_files_parsed.insert(
                                                    path.to_str().unwrap().to_string(),
                                                    src,
                                                );
                                                last_update_time = chrono::Local::now().timestamp();
                                            }
                                        }
                                    }
                                    crate::DiffResult::RsxChanged(changed) => {
                                        for (old, new) in changed.into_iter() {
                                            if let Some(hr) = old
                                                .to_token_stream()
                                                .into_iter()
                                                .map(|tree| {
                                                    let location = tree.span();
                                                    let start = location.start();
                                                    CodeLocation {
                                                        file: path
                                                            .strip_prefix(&crate_dir)
                                                            .unwrap()
                                                            .display()
                                                            .to_string(),
                                                        line: start.line as u32,
                                                        column: start.column as u32 + 1,
                                                    }
                                                })
                                                .min_by(|cl1, cl2| {
                                                    cl1.line
                                                        .cmp(&cl2.line)
                                                        .then(cl1.column.cmp(&cl2.column))
                                                })
                                            {
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
                            old_files_parsed.insert(path.to_str().unwrap().to_string(), src);
                        }
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
        .route("/_dioxus/hot_reload", get(hot_reload_handler))
        .fallback(
            get_service(file_service).handle_error(|error: std::io::Error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            }),
        );

    axum::Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(
            router
                .layer(Extension(ws_reload_state))
                .layer(Extension(hot_reload_state))
                .into_make_service(),
        )
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

async fn hot_reload_handler(
    ws: WebSocketUpgrade,
    _: Option<TypedHeader<headers::UserAgent>>,
    Extension(state): Extension<Arc<HotReloadState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        println!("🔥 Hot Reload WebSocket is connected.");
        let mut rx = state.messages.subscribe();
        loop {
            let read_set_rsx = rx.recv();
            let read_err = socket.recv();
            tokio::select! {
                err = read_err => {
                    if let Some(Ok(Message::Text(err))) = err {
                        let error: RecompileReason = serde_json::from_str(&err).unwrap();
                        log::error!("{:?}", error);
                    };
                },
                set_rsx = read_set_rsx => {
                    if let Ok(rsx) = set_rsx{
                        if socket
                            .send(Message::Text(serde_json::to_string(&rsx).unwrap()))
                            .await
                            .is_err()
                        {
                            break;
                        };
                        // println!("{:?}", rsx);
                    }
                }
            };
        }
    })
}
