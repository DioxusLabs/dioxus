use crate::{
    builder, cfg::Platform, serve::Serve, server::desktop::start_desktop, BuildResult, CrateConfig,
    Result,
};
use axum::{
    body::{Full, HttpBody},
    extract::{ws::Message, Extension, TypedHeader, WebSocketUpgrade},
    http::{
        header::{HeaderName, HeaderValue},
        Method, Response, StatusCode,
    },
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use cargo_metadata::diagnostic::Diagnostic;
use dioxus_core::Template;
use dioxus_html::HtmlCtx;
use dioxus_rsx::hot_reload::*;
use notify::{RecommendedWatcher, Watcher};
use std::{
    net::UdpSocket,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast::{self, Sender};
use tower::ServiceBuilder;
use tower_http::services::fs::{ServeDir, ServeFileSystemResponseBody};
use tower_http::{
    cors::{Any, CorsLayer},
    ServiceBuilderExt,
};

mod output;
use output::*;
pub mod desktop;
pub mod web;

/// Sets up a file watcher
async fn setup_file_watcher(
    platform: Platform,
    config: &CrateConfig,
    reload_tx: Option<Sender<()>>,
    web_info: Option<WebServerInfo>,
) -> Result<RecommendedWatcher> {
    let build_manager = BuildManager {
        platform,
        config: config.clone(),
        reload_tx,
    };

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
                match build_manager.rebuild() {
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
async fn setup_file_watcher_hot_reload(
    config: &CrateConfig,
    hot_reload_tx: Sender<Template<'static>>,
    file_map: Arc<Mutex<FileMap<HtmlCtx>>>,
    build_manager: Arc<BuildManager>,
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
                            match build_manager.rebuild() {
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
                                match build_manager.rebuild() {
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

pub struct BuildManager {
    platform: Platform,
    config: CrateConfig,
    reload_tx: Option<broadcast::Sender<()>>,
}

impl BuildManager {
    fn rebuild(&self) -> Result<BuildResult> {
        log::info!("🪁 Rebuild project");
        match self.platform {
            Platform::Web => {
                let result = builder::build(&self.config, true)?;
                // change the websocket reload state to true;
                // the page will auto-reload.
                if self
                    .config
                    .dioxus_config
                    .web
                    .watcher
                    .reload_html
                    .unwrap_or(false)
                {
                    let _ = Serve::regen_dev_page(&self.config);
                }
                let _ = self.reload_tx.as_ref().map(|tx| tx.send(()));
                Ok(result)
            }
            Platform::Desktop => start_desktop(&self.config),
        }
    }
}
