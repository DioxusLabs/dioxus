use axum::{
    extract::{ws::Message, Extension, TypedHeader, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service},
    AddExtensionLayer, Router,
};
use notify::{watcher, DebouncedEvent, Watcher};
use std::time::Duration;
use std::{
    path::PathBuf,
    sync::{mpsc::channel, Arc, Mutex},
};
use tower_http::services::ServeDir;

use crate::{builder, serve::Serve, CrateConfig};

struct WsRelodState {
    update: bool,
}

impl WsRelodState {
    fn change(&mut self) {
        self.update = !self.update;
    }
}

pub async fn startup(config: CrateConfig) -> anyhow::Result<()> {
    log::info!("🚀 Starting development server...");

    let (tx, rx) = channel();

    let dist_path = config.out_dir.clone();

    // file watcher: check file change
    let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();
    watcher
        .watch(config.crate_dir.clone(), notify::RecursiveMode::Recursive)
        .unwrap();

    let ws_reload_state = Arc::new(Mutex::new(WsRelodState { update: false }));

    let watcher_conf = config.clone();
    let watcher_ws_state = ws_reload_state.clone();
    tokio::spawn(async move {
        let allow_watch_path = watcher_conf
            .dioxus_config
            .web
            .watcher
            .watch_path
            .clone()
            .unwrap_or(vec![PathBuf::from("src")]);
        let crate_dir = watcher_conf.crate_dir.clone();
        loop {
            if let Ok(v) = rx.recv() {
                match v {
                    DebouncedEvent::Create(e)
                    | DebouncedEvent::Write(e)
                    | DebouncedEvent::Remove(e)
                    | DebouncedEvent::Rename(e, _) => {
                        let mut reload = false;
                        for path in &allow_watch_path {
                            let temp = crate_dir.clone().join(path);
                            if e.starts_with(temp) {
                                reload = true;
                                break;
                            }
                        }

                        if reload {
                            if let Ok(_) = builder::build(&watcher_conf) {
                                // change the websocket reload state to true;
                                // the page will auto-reload.
                                if watcher_conf
                                    .dioxus_config
                                    .web
                                    .watcher
                                    .reload_html
                                    .unwrap_or(false)
                                {
                                    let _ = Serve::regen_dev_page(&watcher_conf);
                                }
                                watcher_ws_state.lock().unwrap().change();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    let app = Router::new()
        .route("/_dioxus/ws", get(ws_handler))
        .fallback(
            get_service(ServeDir::new(config.crate_dir.join(&dist_path))).handle_error(
                |error: std::io::Error| async move {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled internal error: {}", error),
                    )
                },
            ),
        )
        .layer(AddExtensionLayer::new(ws_reload_state.clone()));

    // start serve dev-server at 0.0.0.0:8080
    let port = "8080";
    log::info!("📡 Dev-Server is started at: http://127.0.0.1:{}/", port);
    axum::Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    _: Option<TypedHeader<headers::UserAgent>>,
    Extension(state): Extension<Arc<Mutex<WsRelodState>>>,
) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        loop {
            if state.lock().unwrap().update {
                socket
                    .send(Message::Text(String::from("reload")))
                    .await
                    .unwrap();
                state.lock().unwrap().change();
            }
        }
    })
}
