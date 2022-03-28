use axum::{
    extract::{ws::Message, Extension, TypedHeader, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use notify::{RecommendedWatcher, Watcher};

use std::{path::PathBuf, sync::Arc};
use tower_http::services::ServeDir;

use crate::{builder, serve::Serve, CrateConfig, Result};
use tokio::sync::broadcast;

struct WsRelodState {
    update: broadcast::Sender<String>,
}

pub async fn startup(config: CrateConfig) -> Result<()> {
    log::info!("🚀 Starting development server...");

    let dist_path = config.out_dir.clone();

    let (reload_tx, _) = broadcast::channel(100);

    let ws_reload_state = Arc::new(WsRelodState {
        update: reload_tx.clone(),
    });

    let mut last_update_time = chrono::Local::now().timestamp();

    // file watcher: check file change
    let watcher_conf = config.clone();
    let mut watcher = RecommendedWatcher::new(move |_: notify::Result<notify::Event>| {
        if chrono::Local::now().timestamp() > last_update_time {
            log::info!("Start to rebuild project...");
            if builder::build(&watcher_conf).is_ok() {
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
                let _ = reload_tx.send("reload".into());
                last_update_time = chrono::Local::now().timestamp();
            }
        }
    })
    .unwrap();
    let allow_watch_path = config
        .dioxus_config
        .web
        .watcher
        .watch_path
        .clone()
        .unwrap_or_else(|| vec![PathBuf::from("src")]);

    for sub_path in allow_watch_path {
        watcher
            .watch(
                &config.crate_dir.join(sub_path),
                notify::RecursiveMode::Recursive,
            )
            .unwrap();
    }

    // start serve dev-server at 0.0.0.0:8080
    let port = "8080";
    log::info!("📡 Dev-Server is started at: http://127.0.0.1:{}/", port);

    axum::Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(
            Router::new()
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
                .layer(Extension(ws_reload_state))
                .into_make_service(),
        )
        .await?;

    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    _: Option<TypedHeader<headers::UserAgent>>,
    Extension(state): Extension<Arc<WsRelodState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        let mut rx = state.update.subscribe();
        let reload_watcher = tokio::spawn(async move {
            loop {
                let v = rx.recv().await.unwrap();
                if v == "reload" {
                    // ignore the error
                    if socket
                        .send(Message::Text(String::from("reload")))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        });

        reload_watcher.await.unwrap();
    })
}
