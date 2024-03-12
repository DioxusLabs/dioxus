use crate::{cfg::ConfigOptsServe, BuildResult, Result};
use dioxus_cli_config::CrateConfig;

use cargo_metadata::diagnostic::Diagnostic;
use dioxus_core::Template;
use dioxus_hot_reload::HotReloadMsg;
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
pub mod fullstack;
pub mod web;

#[derive(Clone)]
pub struct HotReloadState {
    pub messages: broadcast::Sender<HotReloadMsg>,
    pub file_map: Arc<Mutex<FileMap<HtmlCtx>>>,
}

/// Sets up a file watcher.
///
/// Will attempt to hotreload HTML, RSX (.rs), and CSS
async fn setup_file_watcher<F: Fn() -> Result<BuildResult> + Send + 'static>(
    build_with: F,
    config: &CrateConfig,
    web_info: Option<WebServerInfo>,
    hot_reload: Option<HotReloadState>,
) -> Result<RecommendedWatcher> {
    let mut last_update_time = chrono::Local::now().timestamp();

    // file watcher: check file change
    let mut allow_watch_path = config.dioxus_config.web.watcher.watch_path.clone();

    // Extend the watch path to include the assets directory
    allow_watch_path.push(config.dioxus_config.application.asset_dir.clone());

    // Create the file watcher
    let mut watcher = notify::recommended_watcher({
        let watcher_config = config.clone();
        move |info: notify::Result<notify::Event>| {
            let Ok(e) = info else {
                return;
            };

            watch_event(
                e,
                &mut last_update_time,
                &hot_reload,
                &watcher_config,
                &build_with,
                &web_info,
            );
        }
    })
    .expect("Failed to create file watcher - please ensure you have the required permissions to watch the specified directories.");

    // Watch the specified paths
    for sub_path in allow_watch_path {
        let path = &config.crate_dir.join(sub_path);
        let mode = notify::RecursiveMode::Recursive;

        if let Err(err) = watcher.watch(path, mode) {
            log::warn!("Failed to watch path: {}", err);
        }
    }

    Ok(watcher)
}

fn watch_event<F>(
    event: notify::Event,
    last_update_time: &mut i64,
    hot_reload: &Option<HotReloadState>,
    config: &CrateConfig,
    build_with: &F,
    web_info: &Option<WebServerInfo>,
) where
    F: Fn() -> Result<BuildResult> + Send + 'static,
{
    // Ensure that we're tracking only modifications
    if !matches!(
        event.kind,
        notify::EventKind::Create(_) | notify::EventKind::Remove(_) | notify::EventKind::Modify(_)
    ) {
        return;
    }

    // Ensure that we're not rebuilding too frequently
    if chrono::Local::now().timestamp() <= *last_update_time {
        return;
    }

    // By default we want to opt into a full rebuild, but hotreloading will actually set this force us
    let mut needs_full_rebuild = true;

    if let Some(hot_reload) = &hot_reload {
        hotreload_files(hot_reload, &mut needs_full_rebuild, &event, &config);
    }

    if needs_full_rebuild {
        full_rebuild(build_with, last_update_time, config, event, web_info);
    }
}

fn full_rebuild<F>(
    build_with: &F,
    last_update_time: &mut i64,
    config: &CrateConfig,
    event: notify::Event,
    web_info: &Option<WebServerInfo>,
) where
    F: Fn() -> Result<BuildResult> + Send + 'static,
{
    match build_with() {
        Ok(res) => {
            *last_update_time = chrono::Local::now().timestamp();

            #[allow(clippy::redundant_clone)]
            print_console_info(
                &config,
                PrettierOptions {
                    changed: event.paths.clone(),
                    warnings: res.warnings,
                    elapsed_time: res.elapsed_time,
                },
                web_info.clone(),
            );
        }
        Err(e) => {
            *last_update_time = chrono::Local::now().timestamp();
            log::error!("{:?}", e);
        }
    }
}

fn hotreload_files(
    hot_reload: &HotReloadState,
    needs_full_rebuild: &mut bool,
    event: &notify::Event,
    config: &CrateConfig,
) {
    // find changes to the rsx in the file
    let mut rsx_file_map = hot_reload.file_map.lock().unwrap();
    let mut messages: Vec<HotReloadMsg> = Vec::new();

    // In hot reload mode, we only need to rebuild if non-rsx code is changed
    *needs_full_rebuild = false;

    for path in &event.paths {
        // for various assets that might be linked in, we just try to hotreloading them forcefully
        // That is, unless they appear in an include! macro, in which case we need to a full rebuild....

        // if this is not a rust file, rebuild the whole project
        let path_extension = path.extension().and_then(|p| p.to_str());

        if path_extension != Some("rs") {
            *needs_full_rebuild = true;
            if path_extension == Some("rs~") {
                *needs_full_rebuild = false;
            }
            continue;
        }

        // Workaround for notify and vscode-like editor:
        // when edit & save a file in vscode, there will be two notifications,
        // the first one is a file with empty content.
        // filter the empty file notification to avoid false rebuild during hot-reload
        if let Ok(metadata) = fs::metadata(path) {
            if metadata.len() == 0 {
                continue;
            }
        }

        match rsx_file_map.update_rsx(path, &config.crate_dir) {
            Ok(UpdateResult::UpdatedRsx(msgs)) => {
                messages.extend(
                    msgs.into_iter()
                        .map(|msg| HotReloadMsg::UpdateTemplate(msg)),
                );
                *needs_full_rebuild = false;
            }
            Ok(UpdateResult::NeedsRebuild) => {
                *needs_full_rebuild = true;
            }
            Err(err) => {
                log::error!("{}", err);
            }
        }
    }

    if *needs_full_rebuild {
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
}

pub(crate) trait Platform {
    fn start(config: &CrateConfig, serve: &ConfigOptsServe) -> Result<Self>
    where
        Self: Sized;
    fn rebuild(&mut self, config: &CrateConfig) -> Result<BuildResult>;
}

// Some("bin") => "application/octet-stream",
// Some("css") => "text/css",
// Some("csv") => "text/csv",
// Some("html") => "text/html",
// Some("ico") => "image/vnd.microsoft.icon",
// Some("js") => "text/javascript",
// Some("json") => "application/json",
// Some("jsonld") => "application/ld+json",
// Some("mjs") => "text/javascript",
// Some("rtf") => "application/rtf",
// Some("svg") => "image/svg+xml",
// Some("mp4") => "video/mp4",
