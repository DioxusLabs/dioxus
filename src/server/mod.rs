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

use std::{path::PathBuf, sync::Arc};
use tower::ServiceBuilder;
use tower_http::services::fs::{ServeDir, ServeFileSystemResponseBody};

use crate::{builder, serve::Serve, CrateConfig, Result};
use tokio::sync::broadcast;

#[cfg(feature = "hot_reload")]
mod hot_reload_improts {
    pub use crate::hot_reload::{find_rsx, DiffResult};
    pub use dioxus_rsx_interpreter::{error::RecompileReason, CodeLocation, SetRsxMessage};
    pub use std::collections::HashMap;
    pub use std::sync::Mutex;
    pub use std::{fs, io};
    pub use syn::__private::ToTokens;
}
#[cfg(feature = "hot_reload")]
use hot_reload_improts::*;

struct WsReloadState {
    update: broadcast::Sender<String>,
    #[cfg(feature = "hot_reload")]
    last_file_rebuild: Arc<Mutex<HashMap<String, String>>>,
    watcher_config: CrateConfig,
}

#[cfg(feature = "hot_reload")]
struct HotReloadState {
    messages: broadcast::Sender<SetRsxMessage>,
    update: broadcast::Sender<String>,
}

pub async fn startup(config: CrateConfig) -> Result<()> {
    log::info!("🚀 Starting development server...");

    let dist_path = config.out_dir.clone();

    let (reload_tx, _) = broadcast::channel(100);

    #[cfg(feature = "hot_reload")]
    let last_file_rebuild = Arc::new(Mutex::new(HashMap::new()));
    #[cfg(feature = "hot_reload")]
    find_rs_files(&config.crate_dir, &mut *last_file_rebuild.lock().unwrap()).unwrap();
    #[cfg(feature = "hot_reload")]
    let hot_reload_tx = broadcast::channel(100).0;
    #[cfg(feature = "hot_reload")]
    let hot_reload_state = Arc::new(HotReloadState {
        messages: hot_reload_tx.clone(),
        update: reload_tx.clone(),
    });
    #[cfg(feature = "hot_reload")]
    let crate_dir = config.crate_dir.clone();

    let ws_reload_state = Arc::new(WsReloadState {
        update: reload_tx.clone(),
        #[cfg(feature = "hot_reload")]
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
            if let notify::EventKind::Modify(_) = evt.kind {
                for path in evt.paths {
                    let mut file = File::open(path.clone()).unwrap();
                    let mut src = String::new();
                    file.read_to_string(&mut src).expect("Unable to read file");
                    if src.is_empty() {
                        continue;
                    }
                    #[cfg(feature = "hot_reload")]
                    {
                        if let Ok(syntax) = syn::parse_file(&src) {
                            let mut last_file_rebuild = last_file_rebuild.lock().unwrap();
                            if let Some(old_str) = last_file_rebuild.get(path.to_str().unwrap()) {
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
                                last_file_rebuild.insert(path.to_str().unwrap().to_string(), src);
                            }
                        }
                    }
                    #[cfg(not(feature = "hot_reload"))]
                    {
                        log::info!("reload required");
                        if chrono::Local::now().timestamp() > last_update_time {
                            let _ = reload_tx.send("reload".into());
                            last_update_time = chrono::Local::now().timestamp();
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
        .fallback(
            get_service(file_service).handle_error(|error: std::io::Error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            }),
        );

    #[cfg(feature = "hot_reload")]
    let router = router.route("/_dioxus/hot_reload", get(hot_reload_handler));

    let router = router.layer(Extension(ws_reload_state));

    #[cfg(feature = "hot_reload")]
    let router = router.layer(Extension(hot_reload_state));

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
                            *write = HashMap::new();
                            find_rs_files(&state.watcher_config.crate_dir, &mut *write).unwrap();
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

#[cfg(feature = "hot_reload")]
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
                        state.update.send("reload".to_string()).unwrap();
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
                    }
                }
            };
        }
    })
}

#[cfg(feature = "hot_reload")]
fn find_rs_files(root: &PathBuf, files: &mut HashMap<String, String>) -> io::Result<()> {
    for entry in fs::read_dir(root)? {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                find_rs_files(&path, files)?;
            } else {
                if path.extension().map(|s| s.to_str()).flatten() == Some("rs") {
                    let mut file = File::open(path.clone()).unwrap();
                    let mut src = String::new();
                    file.read_to_string(&mut src).expect("Unable to read file");
                    files.insert(path.display().to_string(), src);
                }
            }
        }
    }
    Ok(())
}
