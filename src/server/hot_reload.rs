use axum::{
    extract::{ws::Message, Extension, TypedHeader, WebSocketUpgrade},
    response::IntoResponse,
};
use dioxus_rsx::CallBody;
// use dioxus_rsx::try_parse_template;

use std::{path::PathBuf, sync::Arc};

use super::BuildManager;
pub use crate::hot_reload::{find_rsx, DiffResult};
use crate::CrateConfig;
use dioxus_core::Template;
use dioxus_html::HtmlCtx;
pub use proc_macro2::TokenStream;
pub use std::collections::HashMap;
pub use std::sync::Mutex;
pub use std::time::SystemTime;
pub use std::{fs, io, path::Path};
pub use std::{fs::File, io::Read};
pub use syn::__private::ToTokens;
use syn::spanned::Spanned;
use tokio::sync::broadcast;

pub(crate) enum UpdateResult {
    UpdatedRsx(Vec<Template<'static>>),
    NeedsRebuild,
}

pub(crate) fn update_rsx(
    path: &Path,
    crate_dir: &Path,
    src: String,
    file_map: &mut FileMap,
) -> UpdateResult {
    if let Ok(syntax) = syn::parse_file(&src) {
        if let Some((old_src, template_slot)) = file_map.map.get_mut(path) {
            if let Ok(old) = syn::parse_file(old_src) {
                match find_rsx(&syntax, &old) {
                    DiffResult::CodeChanged => {
                        file_map.map.insert(path.to_path_buf(), (src, None));
                    }
                    DiffResult::RsxChanged(changed) => {
                        log::info!("🪁 reloading rsx");
                        let mut messages: Vec<Template<'static>> = Vec::new();
                        for (old, new) in changed.into_iter() {
                            let old_start = old.span().start();

                            if let (Ok(old_call_body), Ok(new_call_body)) = (
                                syn::parse2::<CallBody<HtmlCtx>>(old.tokens),
                                syn::parse2::<CallBody<HtmlCtx>>(new),
                            ) {
                                if let Ok(file) = path.strip_prefix(crate_dir) {
                                    let line = old_start.line;
                                    let column = old_start.column + 1;
                                    let location = file.display().to_string()
                                        + ":"
                                        + &line.to_string()
                                        + ":"
                                        + &column.to_string()
                                        // the byte index doesn't matter, but dioxus needs it
                                        + ":0";

                                    if let Some(template) = new_call_body.update_template(
                                        Some(old_call_body),
                                        Box::leak(location.into_boxed_str()),
                                    ) {
                                        // dioxus cannot handle empty templates
                                        if template.roots.is_empty() {
                                            return UpdateResult::NeedsRebuild;
                                        } else {
                                            // if the template is the same, don't send it
                                            if let Some(old_template) = template_slot {
                                                if old_template == &template {
                                                    continue;
                                                }
                                            }
                                            *template_slot = Some(template);
                                            messages.push(template);
                                        }
                                    } else {
                                        return UpdateResult::NeedsRebuild;
                                    }
                                }
                            }
                        }
                        return UpdateResult::UpdatedRsx(messages);
                    }
                }
            }
        } else {
            // if this is a new file, rebuild the project
            *file_map = FileMap::new(crate_dir.to_path_buf());
        }
    }
    UpdateResult::NeedsRebuild
}

pub struct HotReloadState {
    pub messages: broadcast::Sender<Template<'static>>,
    pub build_manager: Arc<BuildManager>,
    pub file_map: Arc<Mutex<FileMap>>,
    pub watcher_config: CrateConfig,
}

pub struct FileMap {
    pub map: HashMap<PathBuf, (String, Option<Template<'static>>)>,
}

impl FileMap {
    pub fn new(path: PathBuf) -> Self {
        log::info!("🔮 Searching files for changes since last compile...");
        fn find_rs_files(
            root: PathBuf,
        ) -> io::Result<HashMap<PathBuf, (String, Option<Template<'static>>)>> {
            let mut files = HashMap::new();
            if root.is_dir() {
                for entry in (fs::read_dir(root)?).flatten() {
                    let path = entry.path();
                    files.extend(find_rs_files(path)?);
                }
            } else if root.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Ok(mut file) = File::open(root.clone()) {
                    let mut src = String::new();
                    file.read_to_string(&mut src).expect("Unable to read file");
                    files.insert(root, (src, None));
                }
            }
            Ok(files)
        }

        let result = Self {
            map: find_rs_files(path).unwrap(),
        };
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
            {
                log::info!("🔮 Finding updates since last compile...");
                let templates: Vec<_> = {
                    state
                        .file_map
                        .lock()
                        .unwrap()
                        .map
                        .values()
                        .filter_map(|(_, template_slot)| *template_slot)
                        .collect()
                };
                for template in templates {
                    if socket
                        .send(Message::Text(serde_json::to_string(&template).unwrap()))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            }
            log::info!("finished");
        }

        let mut rx = state.messages.subscribe();
        loop {
            if let Ok(rsx) = rx.recv().await {
                if socket
                    .send(Message::Text(serde_json::to_string(&rsx).unwrap()))
                    .await
                    .is_err()
                {
                    break;
                };
            }
        }
    })
}
