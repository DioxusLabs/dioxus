use axum::{
    body::{Full, HttpBody},
    extract::{ws::Message, Extension, TypedHeader, WebSocketUpgrade},
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use cargo_metadata::diagnostic::Diagnostic;
use colored::Colorize;
use dioxus_rsx_interpreter::SetRsxMessage;
use notify::Watcher;
use syn::spanned::Spanned;

use std::{net::UdpSocket, path::PathBuf, process::Command, sync::Arc};
use tower::ServiceBuilder;
use tower_http::services::fs::{ServeDir, ServeFileSystemResponseBody};

use crate::{builder, plugin::PluginManager, serve::Serve, BuildResult, CrateConfig, Result};
use tokio::sync::broadcast;

mod hot_reload;
use hot_reload::*;

pub struct BuildManager {
    config: CrateConfig,
    reload_tx: broadcast::Sender<()>,
}

impl BuildManager {
    fn rebuild(&self) -> Result<BuildResult> {
        log::info!("🪁 Rebuild project");
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
        let _ = self.reload_tx.send(());
        Ok(result)
    }
}

struct WsReloadState {
    update: broadcast::Sender<()>,
}

pub async fn startup(port: u16, config: CrateConfig) -> Result<()> {
    // ctrl-c shutdown checker
    let crate_config = config.clone();
    let _ = ctrlc::set_handler(move || {
        let _ = PluginManager::on_serve_shutdown(&crate_config);
        std::process::exit(0);
    });

    if config.hot_reload {
        startup_hot_reload(port, config).await?
    } else {
        startup_default(port, config).await?
    }
    Ok(())
}

pub async fn startup_hot_reload(port: u16, config: CrateConfig) -> Result<()> {
    let first_build_result = crate::builder::build(&config, false)?;

    log::info!("🚀 Starting development server...");

    PluginManager::on_serve_start(&config)?;

    let dist_path = config.out_dir.clone();
    let (reload_tx, _) = broadcast::channel(100);
    let last_file_rebuild = Arc::new(Mutex::new(FileMap::new(config.crate_dir.clone())));
    let build_manager = Arc::new(BuildManager {
        config: config.clone(),
        reload_tx: reload_tx.clone(),
    });
    let hot_reload_tx = broadcast::channel(100).0;
    let hot_reload_state = Arc::new(HotReloadState {
        messages: hot_reload_tx.clone(),
        build_manager: build_manager.clone(),
        last_file_rebuild: last_file_rebuild.clone(),
        watcher_config: config.clone(),
    });

    let crate_dir = config.crate_dir.clone();
    let ws_reload_state = Arc::new(WsReloadState {
        update: reload_tx.clone(),
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

    let watcher_config = config.clone();
    let mut watcher = notify::recommended_watcher(move |evt: notify::Result<notify::Event>| {
        let config = watcher_config.clone();
        if chrono::Local::now().timestamp() > last_update_time {
            // Give time for the change to take effect before reading the file
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Ok(evt) = evt {
                let mut messages = Vec::new();
                let mut needs_rebuild = false;
                for path in evt.paths.clone() {
                    let mut file = File::open(path.clone()).unwrap();
                    if path.extension().map(|p| p.to_str()).flatten() != Some("rs") {
                        continue;
                    }
                    let mut src = String::new();
                    file.read_to_string(&mut src).expect("Unable to read file");
                    // find changes to the rsx in the file
                    if let Ok(syntax) = syn::parse_file(&src) {
                        let mut last_file_rebuild = last_file_rebuild.lock().unwrap();
                        if let Some(old_str) = last_file_rebuild.map.get(&path) {
                            if let Ok(old) = syn::parse_file(&old_str) {
                                match find_rsx(&syntax, &old) {
                                    DiffResult::CodeChanged => {
                                        needs_rebuild = true;
                                        last_file_rebuild.map.insert(path, src);
                                    }
                                    DiffResult::RsxChanged(changed) => {
                                        log::info!("🪁 reloading rsx");
                                        for (old, new) in changed.into_iter() {
                                            let hr = get_location(
                                                &crate_dir,
                                                &path.to_path_buf(),
                                                old.to_token_stream(),
                                            );
                                            // get the original source code to preserve whitespace
                                            let span = new.span();
                                            let start = span.start();
                                            let end = span.end();
                                            let mut lines: Vec<_> = src
                                                .lines()
                                                .skip(start.line - 1)
                                                .take(end.line - start.line + 1)
                                                .collect();
                                            if let Some(first) = lines.first_mut() {
                                                *first = first.split_at(start.column).1;
                                            }
                                            if let Some(last) = lines.last_mut() {
                                                // if there is only one line the start index of last line will be the start of the rsx!, not the start of the line
                                                if start.line == end.line {
                                                    *last =
                                                        last.split_at(end.column - start.column).0;
                                                } else {
                                                    *last = last.split_at(end.column).0;
                                                }
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
                        } else {
                            // if this is a new file, rebuild the project
                            *last_file_rebuild = FileMap::new(crate_dir.clone());
                        }
                    }
                }
                if needs_rebuild {
                    match build_manager.rebuild() {
                        Ok(res) => {
                            print_console_info(
                                port,
                                &config,
                                PrettierOptions {
                                    changed: evt.paths,
                                    warnings: res.warnings,
                                    elapsed_time: res.elapsed_time,
                                },
                            );
                        }
                        Err(err) => {
                            log::error!("{}", err);
                        }
                    }
                }
                if !messages.is_empty() {
                    let _ = hot_reload_tx.send(SetManyRsxMessage(messages));
                }
            }
            last_update_time = chrono::Local::now().timestamp();
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
    print_console_info(
        port,
        &config,
        PrettierOptions {
            changed: vec![],
            warnings: first_build_result.warnings,
            elapsed_time: first_build_result.elapsed_time,
        },
    );

    let file_service_config = config.clone();
    let file_service = ServiceBuilder::new()
        .and_then(
            move |response: Response<ServeFileSystemResponseBody>| async move {
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

    let router = router
        .route("/_dioxus/hot_reload", get(hot_reload_handler))
        .layer(Extension(ws_reload_state))
        .layer(Extension(hot_reload_state));

    axum::Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

pub async fn startup_default(port: u16, config: CrateConfig) -> Result<()> {
    let first_build_result = crate::builder::build(&config, false)?;

    log::info!("🚀 Starting development server...");

    let dist_path = config.out_dir.clone();

    let (reload_tx, _) = broadcast::channel(100);

    let build_manager = BuildManager {
        config: config.clone(),
        reload_tx: reload_tx.clone(),
    };

    let ws_reload_state = Arc::new(WsReloadState {
        update: reload_tx.clone(),
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

    let watcher_config = config.clone();
    let mut watcher = notify::recommended_watcher(move |info: notify::Result<notify::Event>| {
        let config = watcher_config.clone();
        if let Ok(e) = info {
            if chrono::Local::now().timestamp() > last_update_time {
                match build_manager.rebuild() {
                    Ok(res) => {
                        last_update_time = chrono::Local::now().timestamp();
                        print_console_info(
                            port,
                            &config,
                            PrettierOptions {
                                changed: e.paths.clone(),
                                warnings: res.warnings,
                                elapsed_time: res.elapsed_time,
                            },
                        );
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

    // start serve dev-server at 0.0.0.0
    print_console_info(
        port,
        &config,
        PrettierOptions {
            changed: vec![],
            warnings: first_build_result.warnings,
            elapsed_time: first_build_result.elapsed_time,
        },
    );

    PluginManager::on_serve_start(&config)?;

    let file_service_config = config.clone();
    let file_service = ServiceBuilder::new()
        .and_then(
            move |response: Response<ServeFileSystemResponseBody>| async move {
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
        )
        .layer(Extension(ws_reload_state));

    axum::Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

#[derive(Debug, Default)]
pub struct PrettierOptions {
    changed: Vec<PathBuf>,
    warnings: Vec<Diagnostic>,
    elapsed_time: u128,
}

fn print_console_info(port: u16, config: &CrateConfig, options: PrettierOptions) {
    print!(
        "{}",
        String::from_utf8_lossy(
            &Command::new(if cfg!(target_os = "windows") {
                "cls"
            } else {
                "clear"
            })
            .output()
            .unwrap()
            .stdout
        )
    );

    // for path in &changed {
    //     let path = path
    //         .strip_prefix(crate::crate_root().unwrap())
    //         .unwrap()
    //         .to_path_buf();
    //     log::info!("Updated {}", format!("{}", path.to_str().unwrap()).green());
    // }

    let mut profile = if config.release { "Release" } else { "Debug" }.to_string();
    if config.custom_profile.is_some() {
        profile = config.custom_profile.as_ref().unwrap().to_string();
    }
    let hot_reload = if config.hot_reload { "RSX" } else { "Normal" };
    let crate_root = crate::cargo::crate_root().unwrap();
    let custom_html_file = if crate_root.join("index.html").is_file() {
        "Custom [index.html]"
    } else {
        "Default"
    };
    let url_rewrite = if config
        .dioxus_config
        .web
        .watcher
        .index_on_404
        .unwrap_or(false)
    {
        "True"
    } else {
        "False"
    };

    if options.changed.len() <= 0 {
        println!(
            "{} @ v{} [{}] \n",
            "Dioxus".bold().green(),
            crate::DIOXUS_CLI_VERSION,
            chrono::Local::now().format("%H:%M:%S").to_string().dimmed()
        );
    } else {
        println!(
            "Project Reloaded: {}\n",
            format!(
                "Changed {} files. [{}]",
                options.changed.len(),
                chrono::Local::now().format("%H:%M:%S").to_string().dimmed()
            )
            .purple()
            .bold()
        );
    }
    println!(
        "\t> Local : {}",
        format!("http://localhost:{}/", port).blue()
    );
    println!(
        "\t> NetWork : {}",
        format!(
            "http://{}:{}/",
            get_ip().unwrap_or(String::from("0.0.0.0")),
            port
        )
        .blue()
    );
    println!("");
    println!("\t> Profile : {}", profile.green());
    println!("\t> Hot Reload : {}", hot_reload.cyan());
    println!("\t> Index Template : {}", custom_html_file.green());
    println!("\t> URL Rewrite [index_on_404] : {}", url_rewrite.purple());
    println!("");
    println!(
        "\t> Build Time Use : {} millis",
        options.elapsed_time.to_string().green().bold()
    );
    println!("");

    if options.warnings.len() == 0 {
        log::info!("{}\n", "A perfect compilation!".green().bold());
    } else {
        log::warn!(
            "{}",
            format!(
                "There were {} warning messages during the build.",
                options.warnings.len() - 1
            )
            .yellow()
            .bold()
        );
        // for info in &options.warnings {
        //     let message = info.message.clone();
        //     if message == format!("{} warnings emitted", options.warnings.len() - 1) {
        //         continue;
        //     }
        //     let mut console = String::new();
        //     for span in &info.spans {
        //         let file = &span.file_name;
        //         let line = (span.line_start, span.line_end);
        //         let line_str = if line.0 == line.1 {
        //             line.0.to_string()
        //         } else {
        //             format!("{}~{}", line.0, line.1)
        //         };
        //         let code = span.text.clone();
        //         let span_info = if code.len() == 1 {
        //             let code = code.get(0).unwrap().text.trim().blue().bold().to_string();
        //             format!(
        //                 "[{}: {}]: '{}' --> {}",
        //                 file,
        //                 line_str,
        //                 code,
        //                 message.yellow().bold()
        //             )
        //         } else {
        //             let code = code
        //                 .iter()
        //                 .enumerate()
        //                 .map(|(_i, s)| format!("\t{}\n", s.text).blue().bold().to_string())
        //                 .collect::<String>();
        //             format!("[{}: {}]:\n{}\n#:{}", file, line_str, code, message)
        //         };
        //         console = format!("{console}\n\t{span_info}");
        //     }
        //     println!("{console}");
        // }
        // println!(
        //     "\n{}\n",
        //     "Resolving all warnings will help your code run better!".yellow()
        // );
    }
}

fn get_ip() -> Option<String> {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return None,
    };

    match socket.connect("8.8.8.8:80") {
        Ok(()) => (),
        Err(_) => return None,
    };

    match socket.local_addr() {
        Ok(addr) => return Some(addr.ip().to_string()),
        Err(_) => return None,
    };
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
                rx.recv().await.unwrap();
                // ignore the error
                if socket
                    .send(Message::Text(String::from("reload")))
                    .await
                    .is_err()
                {
                    break;
                }

                // flush the errors after recompling
                rx = rx.resubscribe();
            }
        });

        reload_watcher.await.unwrap();
    })
}
