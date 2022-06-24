use axum::{
    extract::{ws::Message, Extension, TypedHeader, WebSocketUpgrade},
    response::IntoResponse,
};
use dioxus::rsx_interpreter::SetRsxMessage;

use std::{path::PathBuf, sync::Arc};

use super::BuildManager;
pub use crate::hot_reload::{find_rsx, DiffResult};
use crate::CrateConfig;
pub use dioxus::rsx_interpreter::{error::Error, CodeLocation, SetManyRsxMessage};
pub use proc_macro2::TokenStream;
pub use std::collections::HashMap;
pub use std::sync::Mutex;
pub use std::time::SystemTime;
pub use std::{fs, io, path::Path};
pub use std::{fs::File, io::Read};
pub use syn::__private::ToTokens;
use syn::spanned::Spanned;
use tokio::sync::broadcast;

pub struct HotReloadState {
    pub messages: broadcast::Sender<SetManyRsxMessage>,
    pub build_manager: Arc<BuildManager>,
    pub last_file_rebuild: Arc<Mutex<FileMap>>,
    pub watcher_config: CrateConfig,
}

pub struct FileMap {
    pub map: HashMap<PathBuf, String>,
    pub last_updated_time: std::time::SystemTime,
}

impl FileMap {
    pub fn new(path: PathBuf) -> Self {
        log::info!("Searching files for changes since last compile...");
        fn find_rs_files(root: PathBuf) -> io::Result<HashMap<PathBuf, String>> {
            let mut files = HashMap::new();
            if root.is_dir() {
                for entry in fs::read_dir(root)? {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        files.extend(find_rs_files(path)?);
                    }
                }
            } else {
                if root.extension().map(|s| s.to_str()).flatten() == Some("rs") {
                    if let Ok(mut file) = File::open(root.clone()) {
                        let mut src = String::new();
                        file.read_to_string(&mut src).expect("Unable to read file");
                        files.insert(root, src);
                    }
                }
            }
            Ok(files)
        }

        let last_updated_time = SystemTime::now();
        let result = Self {
            last_updated_time,
            map: find_rs_files(path).unwrap(),
        };
        log::info!("Files updated");
        result
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
            // update any rsx calls that changed before the websocket connected.
            let mut messages = Vec::new();

            {
                log::info!("Finding updates since last compile...");
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
                    let mut new_str = String::new();
                    file.read_to_string(&mut new_str).expect("Unable to read file");
                    if let Ok(new_file) = syn::parse_file(&new_str) {
                        if let Ok(old_file) = syn::parse_file(&v) {
                            if let DiffResult::RsxChanged(changed) = find_rsx(&new_file, &old_file) {
                                for (old, new) in changed.into_iter() {
                                    let hr = get_location(
                                        k,
                                        old.to_token_stream(),
                                    );
                                    // get the original source code to preserve whitespace
                                    let span = new.span();
                                    let start = span.start();
                                    let end = span.end();
                                    let mut lines: Vec<_> = new_str
                                        .lines()
                                        .skip(start.line - 1)
                                        .take(end.line - start.line + 1)
                                        .collect();
                                    if let Some(first) = lines.first_mut() {
                                        *first = first.split_at(start.column).1;
                                    }
                                    if let Some(last) = lines.last_mut() {
                                        *last = last.split_at(end.column).0;
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
                }
                log::info!("finished");
            }

            let msg = SetManyRsxMessage(messages);
            if socket
                .send(Message::Text(serde_json::to_string(&msg).unwrap()))
                .await
                .is_err()
            {
                return;
            }
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
                                match error{
                                    Error::ParseError(parse_error) => {
                                        log::error!("parse error:\n--> at {}:{}:{}\n\t{:?}", parse_error.location.file, parse_error.location.line, parse_error.location.column, parse_error.message);
                                    },
                                    Error::RecompileRequiredError(_) => {
                                        if let Err(err) = state.build_manager.build(){
                                            log::error!("{}", err);
                                        }
                                    }
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

pub fn get_location(path: &Path, ts: TokenStream) -> CodeLocation {
    let span = ts.span().start();
    CodeLocation {
        file: path.display().to_string(),
        line: span.line as u32,
        column: span.column as u32 + 1,
    }
}
