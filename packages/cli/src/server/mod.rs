use crate::{cfg::ConfigOptsServe, BuildResult, CrateConfig, Result};

use cargo_metadata::diagnostic::Diagnostic;
use dioxus_core::Template;
use dioxus_html::HtmlCtx;
use dioxus_rsx::hot_reload::*;
use notify::{RecommendedWatcher, Watcher};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast::Sender;

mod output;
use output::*;
pub mod desktop;
pub mod fullstack;
pub mod web;

/// Sets up a file watcher
async fn setup_file_watcher<F: Fn() -> Result<BuildResult> + Send + 'static>(
    build_with: F,
    config: &CrateConfig,
    web_info: Option<WebServerInfo>,
) -> Result<RecommendedWatcher> {
    let mut last_update_time = chrono::Local::now().timestamp();

    // file watcher: check file change
    let allow_watch_path = config
        .dioxus_config
        .web
        .watcher
        .watch_path
        .clone()
        .unwrap_or_else(|| vec![PathBuf::from("src")]);

    let watcher_config = config.clone();
    let mut watcher = notify::recommended_watcher(move |info: notify::Result<notify::Event>| {
        let config = watcher_config.clone();
        if let Ok(e) = info {
            if chrono::Local::now().timestamp() > last_update_time {
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
    Ok(watcher)
}

// Todo: reduce duplication and merge with setup_file_watcher()
/// Sets up a file watcher with hot reload
async fn setup_file_watcher_hot_reload<F: Fn() -> Result<BuildResult> + Send + 'static>(
    config: &CrateConfig,
    hot_reload_tx: Sender<Template<'static>>,
    file_map: Arc<Mutex<FileMap<HtmlCtx>>>,
    build_with: F,
    web_info: Option<WebServerInfo>,
) -> Result<RecommendedWatcher> {
    // file watcher: check file change
    let allow_watch_path = config
        .dioxus_config
        .web
        .watcher
        .watch_path
        .clone()
        .unwrap_or_else(|| vec![PathBuf::from("src")]);

    let watcher_config = config.clone();
    let mut last_update_time = chrono::Local::now().timestamp();

    let mut watcher = RecommendedWatcher::new(
        move |evt: notify::Result<notify::Event>| {
            let config = watcher_config.clone();
            // Give time for the change to take effect before reading the file
            std::thread::sleep(std::time::Duration::from_millis(100));
            if chrono::Local::now().timestamp() > last_update_time {
                if let Ok(evt) = evt {
                    let mut messages: Vec<Template<'static>> = Vec::new();
                    for path in evt.paths.clone() {
                        // if this is not a rust file, rebuild the whole project
                        if path.extension().and_then(|p| p.to_str()) != Some("rs") {
                            match build_with() {
                                Ok(res) => {
                                    print_console_info(
                                        &config,
                                        PrettierOptions {
                                            changed: evt.paths,
                                            warnings: res.warnings,
                                            elapsed_time: res.elapsed_time,
                                        },
                                        web_info.clone(),
                                    );
                                }
                                Err(err) => {
                                    log::error!("{}", err);
                                }
                            }
                            return;
                        }
                        // find changes to the rsx in the file
                        let mut map = file_map.lock().unwrap();

                        match map.update_rsx(&path, &config.crate_dir) {
                            Ok(UpdateResult::UpdatedRsx(msgs)) => {
                                messages.extend(msgs);
                            }
                            Ok(UpdateResult::NeedsRebuild) => {
                                match build_with() {
                                    Ok(res) => {
                                        print_console_info(
                                            &config,
                                            PrettierOptions {
                                                changed: evt.paths,
                                                warnings: res.warnings,
                                                elapsed_time: res.elapsed_time,
                                            },
                                            web_info.clone(),
                                        );
                                    }
                                    Err(err) => {
                                        log::error!("{}", err);
                                    }
                                }
                                return;
                            }
                            Err(err) => {
                                log::error!("{}", err);
                            }
                        }
                    }
                    for msg in messages {
                        let _ = hot_reload_tx.send(msg);
                    }
                }
                last_update_time = chrono::Local::now().timestamp();
            }
        },
        notify::Config::default(),
    )
    .unwrap();

    for sub_path in allow_watch_path {
        if let Err(err) = watcher.watch(
            &config.crate_dir.join(&sub_path),
            notify::RecursiveMode::Recursive,
        ) {
            log::error!("error watching {sub_path:?}: \n{}", err);
        }
    }

    Ok(watcher)
}

pub(crate) trait Platform {
    fn start(config: &CrateConfig, serve: &ConfigOptsServe) -> Result<Self>
    where
        Self: Sized;
    fn rebuild(&mut self, config: &CrateConfig) -> Result<BuildResult>;
}
