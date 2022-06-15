use axum::{
    extract::{ws::Message, Extension, TypedHeader, WebSocketUpgrade},
    response::IntoResponse,
};

use std::{path::PathBuf, sync::Arc};

pub use crate::hot_reload::{find_rsx, DiffResult};
use crate::CrateConfig;
pub use dioxus_rsx_interpreter::{error::Error, CodeLocation, SetRsxMessage};
pub use proc_macro2::TokenStream;
pub use std::collections::HashMap;
pub use std::sync::Mutex;
pub use std::time::SystemTime;
pub use std::{fs, io, path::Path};
pub use std::{fs::File, io::Read};
pub use syn::__private::ToTokens;
use tokio::sync::broadcast;

pub struct HotReloadState {
    pub messages: broadcast::Sender<SetRsxMessage>,
    pub update: broadcast::Sender<String>,
    pub last_file_rebuild: Arc<Mutex<FileMap>>,
    pub watcher_config: CrateConfig,
}

pub struct FileMap {
    pub map: HashMap<PathBuf, String>,
    pub last_updated_time: std::time::SystemTime,
}

impl FileMap {
    pub fn new(path: PathBuf) -> Self {
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

pub async fn hot_reload_handler(
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
                                let error: Error = serde_json::from_str(&err).unwrap();
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

pub fn get_min_location(path: &Path, ts: TokenStream) -> Option<CodeLocation> {
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
