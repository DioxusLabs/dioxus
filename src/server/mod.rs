use crate::{builder, plugin::PluginManager, serve::Serve, BuildResult, CrateConfig, Result};
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
use cargo_metadata::diagnostic::Diagnostic;
use colored::Colorize;
use dioxus_core::Template;
use dioxus_html::HtmlCtx;
use dioxus_rsx::hot_reload::*;
use notify::{RecommendedWatcher, Watcher};
use std::{
    net::UdpSocket,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use tower::ServiceBuilder;
use tower_http::services::fs::{ServeDir, ServeFileSystemResponseBody};
use tower_http::{
    cors::{Any, CorsLayer},
    ServiceBuilderExt,
};
mod proxy;

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

pub async fn startup(port: u16, config: CrateConfig, start_browser: bool) -> Result<()> {
    // ctrl-c shutdown checker
    let crate_config = config.clone();
    let _ = ctrlc::set_handler(move || {
        let _ = PluginManager::on_serve_shutdown(&crate_config);
        std::process::exit(0);
    });

    let ip = get_ip().unwrap_or(String::from("0.0.0.0"));

    if config.hot_reload {
        startup_hot_reload(ip, port, config, start_browser).await?
    } else {
        startup_default(ip, port, config, start_browser).await?
    }
    Ok(())
}

pub struct HotReloadState {
    pub messages: broadcast::Sender<Template<'static>>,
    pub build_manager: Arc<BuildManager>,
    pub file_map: Arc<Mutex<FileMap<HtmlCtx>>>,
    pub watcher_config: CrateConfig,
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

#[allow(unused_assignments)]
pub async fn startup_hot_reload(ip: String, port: u16, config: CrateConfig, start_browser: bool) -> Result<()> {
    let first_build_result = crate::builder::build(&config, false)?;

    log::info!("🚀 Starting development server...");

    PluginManager::on_serve_start(&config)?;

    let dist_path = config.out_dir.clone();
    let (reload_tx, _) = broadcast::channel(100);
    let map = FileMap::<HtmlCtx>::new(config.crate_dir.clone());
    // for err in errors {
    //     log::error!("{}", err);
    // }
    let file_map = Arc::new(Mutex::new(map));
    let build_manager = Arc::new(BuildManager {
        config: config.clone(),
        reload_tx: reload_tx.clone(),
    });
    let hot_reload_tx = broadcast::channel(100).0;
    let hot_reload_state = Arc::new(HotReloadState {
        messages: hot_reload_tx.clone(),
        build_manager: build_manager.clone(),
        file_map: file_map.clone(),
        watcher_config: config.clone(),
    });

    let crate_dir = config.crate_dir.clone();
    let ws_reload_state = Arc::new(WsReloadState {
        update: reload_tx.clone(),
    });

    // file watcher: check file change
    let allow_watch_path = config
        .dioxus_config
        .web
        .watcher
        .watch_path
        .clone()
        .unwrap_or_else(|| vec![PathBuf::from("src")]);

    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any)
        .allow_headers(Any);

    let watcher_config = config.clone();
    let watcher_ip = ip.clone();
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
                                        &watcher_ip,
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
                            return;
                        }
                        // find changes to the rsx in the file
                        let mut map = file_map.lock().unwrap();

                        match map.update_rsx(&path, &crate_dir) {
                            UpdateResult::UpdatedRsx(msgs) => {
                                messages.extend(msgs);
                            }
                            UpdateResult::NeedsRebuild => {
                                match build_manager.rebuild() {
                                    Ok(res) => {
                                        print_console_info(
                                            &watcher_ip,
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
                                return;
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

    // start serve dev-server at 0.0.0.0:8080
    print_console_info(
        &ip,
        port,
        &config,
        PrettierOptions {
            changed: vec![],
            warnings: first_build_result.warnings,
            elapsed_time: first_build_result.elapsed_time,
        },
    );

    let cors = CorsLayer::new()
    // allow `GET` and `POST` when accessing the resource
    .allow_methods([Method::GET, Method::POST])
    // allow requests from any origin
    .allow_origin(Any)
    .allow_headers(Any);

    let (coep, coop) = if config.shared_array_buffer {
        (
            HeaderValue::from_static("require-corp"),
            HeaderValue::from_static("same-origin"),
        )
    } else {
        (
            HeaderValue::from_static("unsafe-none"),
            HeaderValue::from_static("unsafe-none"),
        )
    };

    let file_service_config = config.clone();
    let file_service = ServiceBuilder::new()
        .override_response_header(
            HeaderName::from_static("cross-origin-embedder-policy"),
            coep,
        )
        .override_response_header(
            HeaderName::from_static("cross-origin-opener-policy"),
            coop,
        )
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
        .service(ServeDir::new(config.crate_dir.join(&dist_path)));

    let mut router = Router::new().route("/_dioxus/ws", get(ws_handler));
    for proxy_config in config.dioxus_config.web.proxy.unwrap_or_default() {
        router = proxy::add_proxy(router, &proxy_config)?;
    }
    router = router.fallback(get_service(file_service).handle_error(
        |error: std::io::Error| async move {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unhandled internal error: {}", error),
            )
        },
    ));

    let router = router
        .route("/_dioxus/hot_reload", get(hot_reload_handler))
        .layer(cors)
        .layer(Extension(ws_reload_state))
        .layer(Extension(hot_reload_state));

    let addr = format!("0.0.0.0:{}", port).parse().unwrap();

    let server = axum::Server::bind(&addr).serve(router.into_make_service());

    if start_browser {
        let _ = open::that(format!("http://{}", addr));
    }

    server.await?;

    Ok(())
}

pub async fn startup_default(
    ip: String,
    port: u16,
    config: CrateConfig,
    start_browser: bool,
) -> Result<()> {
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

    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any)
        .allow_headers(Any);

    // file watcher: check file change
    let allow_watch_path = config
        .dioxus_config
        .web
        .watcher
        .watch_path
        .clone()
        .unwrap_or_else(|| vec![PathBuf::from("src")]);

    let watcher_config = config.clone();
    let watcher_ip = ip.clone();
    let mut watcher = notify::recommended_watcher(move |info: notify::Result<notify::Event>| {
        let config = watcher_config.clone();
        if let Ok(e) = info {
            if chrono::Local::now().timestamp() > last_update_time {
                match build_manager.rebuild() {
                    Ok(res) => {
                        last_update_time = chrono::Local::now().timestamp();
                        print_console_info(
                            &watcher_ip,
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
        &ip,
        port,
        &config,
        PrettierOptions {
            changed: vec![],
            warnings: first_build_result.warnings,
            elapsed_time: first_build_result.elapsed_time,
        },
    );

    PluginManager::on_serve_start(&config)?;

    let cors = CorsLayer::new()
    // allow `GET` and `POST` when accessing the resource
    .allow_methods([Method::GET, Method::POST])
    // allow requests from any origin
    .allow_origin(Any)
    .allow_headers(Any);

    let (coep, coop) = if config.shared_array_buffer {
        (
            HeaderValue::from_static("require-corp"),
            HeaderValue::from_static("same-origin"),
        )
    } else {
        (
            HeaderValue::from_static("unsafe-none"),
            HeaderValue::from_static("unsafe-none"),
        )
    };

    let file_service_config = config.clone();
    let file_service = ServiceBuilder::new()
        .override_response_header(
            HeaderName::from_static("cross-origin-embedder-policy"),
            coep,
        )
        .override_response_header(
            HeaderName::from_static("cross-origin-opener-policy"),
            coop,
        )
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
        .service(ServeDir::new(config.crate_dir.join(&dist_path)));

    let mut router = Router::new().route("/_dioxus/ws", get(ws_handler));
    for proxy_config in config.dioxus_config.web.proxy.unwrap_or_default() {
        router = proxy::add_proxy(router, &proxy_config)?;
    }
    router = router
        .fallback(
            get_service(file_service).handle_error(|error: std::io::Error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            }),
        )
        .layer(cors)
        .layer(Extension(ws_reload_state));

    let addr = format!("0.0.0.0:{}", port).parse().unwrap();
    let server = axum::Server::bind(&addr).serve(router.into_make_service());

    if start_browser {
        let _ = open::that(format!("http://{}", addr));
    }

    server.await?;

    Ok(())
}

#[derive(Debug, Default)]
pub struct PrettierOptions {
    changed: Vec<PathBuf>,
    warnings: Vec<Diagnostic>,
    elapsed_time: u128,
}

fn print_console_info(ip: &String, port: u16, config: &CrateConfig, options: PrettierOptions) {
    if let Ok(native_clearseq) = Command::new(if cfg!(target_os = "windows") {
        "cls"
    } else {
        "clear"
    })
    .output()
    {
        print!("{}", String::from_utf8_lossy(&native_clearseq.stdout));
    } else {
        // Try ANSI-Escape characters
        print!("\x1b[2J\x1b[H");
    }

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

    let proxies = config.dioxus_config.web.proxy.as_ref();

    if options.changed.is_empty() {
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
        format!("http://{}:{}/", ip, port).blue()
    );
    println!("");
    println!("\t> Profile : {}", profile.green());
    println!("\t> Hot Reload : {}", hot_reload.cyan());
    if let Some(proxies) = proxies {
        if !proxies.is_empty() {
            println!("\t> Proxies :");
            for proxy in proxies {
                println!("\t\t- {}", proxy.backend.blue());
            }
        }
    }
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
