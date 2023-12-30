use crate::{
    plugin::{
        handle_change,
        interface::plugins::main::types::RuntimeEvent::{HotReload, Rebuild},
        plugins_after_runtime, plugins_before_runtime, plugins_watched_paths_changed,
    },
    BuildResult, CrateConfig, Result,
};

use cargo_metadata::diagnostic::Diagnostic;
use dioxus_core::Template;
use dioxus_html::HtmlCtx;
use dioxus_rsx::hot_reload::*;
use notify::{RecommendedWatcher, Watcher};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast::{self, Sender};

mod output;
use output::*;
pub mod desktop;
pub mod web;

/// Sets up a file watcher
async fn setup_file_watcher<F: Fn() -> Result<BuildResult> + Sync + Send + 'static>(
    build_with: F,
    config: &CrateConfig,
    web_info: Option<WebServerInfo>,
    reload: ServerReloadState,
) -> Result<RecommendedWatcher> {
    let build_with = Arc::new(build_with);

    let ServerReloadState {
        hot_reload,
        reload_tx,
    } = reload;

    let mut last_update_time = chrono::Local::now().timestamp();

    // file watcher: check file change
    let allow_watch_path = config
        .dioxus_config
        .watcher
        .watch_path
        .clone()
        .unwrap_or_else(|| vec![PathBuf::from("src"), PathBuf::from("examples")]);

    let watcher_config = config.clone();
    let mut watcher = notify::recommended_watcher(move |info: notify::Result<notify::Event>| {
        let config = watcher_config.clone();

        let Ok(e) = info else {
            return;
        };

        if chrono::Local::now().timestamp() <= last_update_time {
            return;
        }

        let mut needs_full_rebuild = false;
        let change =
            futures::executor::block_on(plugins_watched_paths_changed(&e.paths, &config.crate_dir));
        handle_change(change, &reload_tx, &mut needs_full_rebuild);

        if let Some(hot_reload) = &hot_reload {
            let change = futures::executor::block_on(plugins_before_runtime(HotReload));
            handle_change(change, &reload_tx, &mut needs_full_rebuild);

            // find changes to the rsx in the file
            let mut rsx_file_map = hot_reload.file_map.lock().unwrap();
            let mut messages: Vec<Template<'static>> = Vec::new();

            for path in &e.paths {
                // if this is not a rust file, rebuild the whole project
                if path.extension().and_then(|p| p.to_str()) != Some("rs") {
                    needs_full_rebuild = true;
                    break;
                }

                match rsx_file_map.update_rsx(path, &config.crate_dir) {
                    Ok(UpdateResult::UpdatedRsx(msgs)) => {
                        messages.extend(msgs);
                        needs_full_rebuild = false;
                    }
                    Ok(UpdateResult::NeedsRebuild) => {
                        needs_full_rebuild = true;
                    }
                    Err(err) => {
                        log::error!("{}", err);
                    }
                }
            }

            if needs_full_rebuild {
                // Reset the file map to the new state of the project
                let FileMapBuildResult {
                    map: new_file_map,
                    errors,
                } = FileMap::<HtmlCtx>::create(config.crate_dir.clone()).unwrap();

                for err in errors {
                    log::error!("{}", err);
                }

                *rsx_file_map = new_file_map;
            } else {
                for msg in messages {
                    let _ = hot_reload.messages.send(msg);
                }
            }

            let change = futures::executor::block_on(plugins_after_runtime(HotReload));
            handle_change(change, &reload_tx, &mut needs_full_rebuild);
        } else {
            needs_full_rebuild = true;
        }

        if needs_full_rebuild {
            // Can be ignored, going to rebuild anyway
            let _change = futures::executor::block_on(plugins_before_runtime(Rebuild));

            match build_with() {
                Ok(res) => {
                    last_update_time = chrono::Local::now().timestamp();

                    #[allow(clippy::redundant_clone)]
                    print_console_info(
                        &config,
                        PrettierOptions {
                            changed: e.paths.clone(),
                            warnings: res.warnings,
                            elapsed_time: res.elapsed_time,
                        },
                        web_info.clone(),
                    );
                }
                Err(e) => log::error!("{}", e),
            }

            let change = futures::executor::block_on(plugins_after_runtime(Rebuild));
            // Todo handle plugins requesting rebuild here
            handle_change(change, &reload_tx, &mut needs_full_rebuild);
        }
    })
    .unwrap();

    for sub_path in allow_watch_path {
        if let Err(err) = watcher.watch(
            &config.crate_dir.join(sub_path),
            notify::RecursiveMode::Recursive,
        ) {
            log::error!("Failed to watch path: {}", err);
        }
    }
    Ok(watcher)
}

#[derive(Clone)]
pub struct ServerReloadState {
    pub hot_reload: Option<HotReloadState>,
    reload_tx: Option<Sender<WsMessage>>,
}

impl ServerReloadState {
    pub fn new(hot_reload_state: Option<HotReloadState>) -> Self {
        Self {
            hot_reload: hot_reload_state,
            reload_tx: None,
        }
    }

    pub fn with_reload_tx(self, reload_tx: Option<Sender<WsMessage>>) -> Self {
        Self {
            hot_reload: self.hot_reload,
            reload_tx,
        }
    }

    pub fn reload_browser(&self) {
        if let Some(reload_tx) = &self.reload_tx {
            let _ = reload_tx.send(WsMessage::Reload);
        }
    }

    pub fn refresh_assets(&self, urls: Vec<String>) {
        if let Some(reload_tx) = &self.reload_tx {
            let _ = reload_tx.send(WsMessage::RefreshAssets { urls });
        }
    }
}

#[derive(Clone)]
pub struct HotReloadState {
    pub messages: broadcast::Sender<Template<'static>>,
    pub file_map: Arc<Mutex<FileMap<HtmlCtx>>>,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "method", content = "params")]
pub enum WsMessage {
    #[serde(rename = "reload")]
    Reload,
    #[serde(rename = "refresh_assets")]
    RefreshAssets { urls: Vec<String> },
}
