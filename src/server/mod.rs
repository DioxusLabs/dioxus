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
mod hot_reload_improts {
    pub use crate::hot_reload::{find_rsx, DiffResult};
    pub use dioxus_rsx_interpreter::{error::RecompileReason, CodeLocation, SetRsxMessage};
    pub use proc_macro2::TokenStream;
    pub use std::collections::HashMap;
    pub use std::sync::Mutex;
    pub use std::time::SystemTime;
    pub use std::{fs, io, path::Path};
    pub use std::{fs::File, io::Read};
    pub use syn::__private::ToTokens;
}
#[cfg(feature = "hot_reload")]
use hot_reload_improts::*;

struct WsReloadState {
    update: broadcast::Sender<String>,
    #[cfg(feature = "hot_reload")]
    last_file_rebuild: Arc<Mutex<FileMap>>,
    watcher_config: CrateConfig,
}

#[cfg(feature = "hot_reload")]
struct HotReloadState {
    messages: broadcast::Sender<SetRsxMessage>,
    update: broadcast::Sender<String>,
    last_file_rebuild: Arc<Mutex<FileMap>>,
    watcher_config: CrateConfig,
}

#[cfg(feature = "hot_reload")]
struct FileMap {
    map: HashMap<PathBuf, String>,
    last_updated_time: std::time::SystemTime,
}

#[cfg(feature = "hot_reload")]
impl FileMap {
    fn new(path: PathBuf) -> Self {
        fn find_rs_files(root: PathBuf) -> io::Result<HashMap<PathBuf, String>> {
            let mut files = HashMap::new();
            if root.is_dir() {
                let mut handles = Vec::new();
                for entry in fs::read_dir(root)? {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        handles.push(std::thread::spawn(move || find_rs_files(path)));
                    }
                }
                for handle in handles {
                    files.extend(handle.join().unwrap()?);
                }
            } else {
                if root.extension().map(|s| s.to_str()).flatten() == Some("rs") {
                    let mut file = File::open(root.clone()).unwrap();
                    let mut src = String::new();
                    file.read_to_string(&mut src).expect("Unable to read file");
                    files.insert(root, src);
                }
            }
            Ok(files)
        }
        Self {
            last_updated_time: SystemTime::now(),
            map: find_rs_files(path).unwrap(),
        }
    }
}

pub async fn startup(config: CrateConfig) -> Result<()> {
    log::info!("🚀 Starting development server...");

    let dist_path = config.out_dir.clone();

    let (reload_tx, _) = broadcast::channel(100);

    #[cfg(feature = "hot_reload")]
    let last_file_rebuild = Arc::new(Mutex::new(FileMap::new(config.crate_dir.clone())));
    #[cfg(feature = "hot_reload")]
    let hot_reload_tx = broadcast::channel(100).0;
    #[cfg(feature = "hot_reload")]
    let hot_reload_state = Arc::new(HotReloadState {
        messages: hot_reload_tx.clone(),
        update: reload_tx.clone(),
        last_file_rebuild: last_file_rebuild.clone(),
        watcher_config: config.clone(),
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
                #[cfg(feature = "hot_reload")]
                {
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
                                                    &path
                                                        .strip_prefix(&crate_dir)
                                                        .unwrap()
                                                        .to_path_buf(),
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
                                *last_file_rebuild = FileMap::new(crate_dir.clone());
                            }
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

#[cfg(feature = "hot_reload")]
async fn hot_reload_handler(
    ws: WebSocketUpgrade,
    _: Option<TypedHeader<headers::UserAgent>>,
    Extension(state): Extension<Arc<HotReloadState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        log::info!("🔥 Hot Reload WebSocket connected");
        {
            log::info!("Searching files for changes since last run...");
            // update any files that changed before the websocket connected.
            let mut messages = Vec::new();

            {
                let handle = state.last_file_rebuild.lock().unwrap();
                let update_time = handle.last_updated_time.clone();
                for (k, v) in handle.map.iter() {
                    let mut file = File::open(k).unwrap();
                    if let Ok(md) = file.metadata() {
                        if let Ok(time) = md.modified() {
                            if time < update_time {
                                continue;
                            }
                        }
                    }
                    let mut new = String::new();
                    file.read_to_string(&mut new).expect("Unable to read file");
                    if let Ok(new) = syn::parse_file(&new) {
                        if let Ok(old) = syn::parse_file(&v) {
                            if let DiffResult::RsxChanged(changed) = find_rsx(&new, &old) {
                                for (old, new) in changed.into_iter() {
                                    if let Some(hr) = get_min_location(
                                        k.strip_prefix(&state.watcher_config.crate_dir).unwrap(),
                                        old.to_token_stream(),
                                    ) {
                                        let rsx = new.to_string();
                                        let msg = SetRsxMessage {
                                            location: hr,
                                            new_text: rsx,
                                        };
                                        messages.push(msg);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            for msg in &messages {
                if socket
                    .send(Message::Text(serde_json::to_string(msg).unwrap()))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            log::info!("Updated page");
        }

        let mut rx = state.messages.subscribe();
        let hot_reload_handle = tokio::spawn(async move {
            loop {
                let read_set_rsx = rx.recv();
                let read_err = socket.recv();
                tokio::select! {
                    err = read_err => {
                        if let Some(Ok(err)) = err {
                            if let Message::Text(err) = err {
                                let error: RecompileReason = serde_json::from_str(&err).unwrap();
                                log::error!("{:?}", error);
                                if state.update.send("reload".to_string()).is_err() {
                                    break;
                                }
                            }
                        } else {
                            break;
                        }
                    },
                    set_rsx = read_set_rsx => {
                        if let Ok(rsx) = set_rsx {
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
        });

        hot_reload_handle.await.unwrap();
    })
}

#[cfg(feature = "hot_reload")]
fn get_min_location(path: &Path, ts: TokenStream) -> Option<CodeLocation> {
    ts.into_iter()
        .map(|tree| {
            let location = tree.span();
            let start = location.start();
            CodeLocation {
                file: path.display().to_string(),
                line: start.line as u32,
                column: start.column as u32 + 1,
            }
        })
        .min_by(|cl1, cl2| cl1.line.cmp(&cl2.line).then(cl1.column.cmp(&cl2.column)))
}
