use crate::{BuildResult, CrateConfig, Result};

use cargo_metadata::diagnostic::Diagnostic;
use dioxus_core::Template;
use dioxus_html::HtmlCtx;
use dioxus_rsx::hot_reload::*;
use notify::{RecommendedWatcher, Watcher};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast::{self};

mod output;
use output::*;
pub mod desktop;
pub mod web;

/// Sets up a file watcher
async fn setup_file_watcher<F: Fn() -> Result<BuildResult> + Send + 'static>(
    build_with: F,
    config: &CrateConfig,
    web_info: Option<WebServerInfo>,
    hot_reload: Option<HotReloadState>,
) -> Result<RecommendedWatcher> {
    let mut last_update_time = chrono::Local::now().timestamp();

    // file watcher: check file change
    let allow_watch_path = config
        .dioxus_config
        .web
        .watcher
        .watch_path
        .clone()
        .unwrap_or_else(|| vec![PathBuf::from("src"), PathBuf::from("examples")]);

    let watcher_config = config.clone();
    let mut watcher = notify::recommended_watcher(move |info: notify::Result<notify::Event>| {
        let config = watcher_config.clone();
        if let Ok(e) = info {
            if chrono::Local::now().timestamp() > last_update_time {
                let mut needs_full_rebuild;
                if let Some(hot_reload) = &hot_reload {
                    // find changes to the rsx in the file
                    let mut rsx_file_map = hot_reload.file_map.lock().unwrap();
                    let mut messages: Vec<Template<'static>> = Vec::new();

                    // In hot reload mode, we only need to rebuild if non-rsx code is changed
                    needs_full_rebuild = false;

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
                } else {
                    needs_full_rebuild = true;
                }

                if needs_full_rebuild {
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

                            #[cfg(feature = "plugin")]
                            let _ = PluginManager::on_serve_rebuild(
                                chrono::Local::now().timestamp(),
                                e.paths,
                            );
                        }
                        Err(e) => log::error!("{}", e),
                    }
                }
            }
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
pub struct HotReloadState {
    pub messages: broadcast::Sender<Template<'static>>,
    pub file_map: Arc<Mutex<FileMap<HtmlCtx>>>,
}
