use axum::{
    extract::{
        ws::{Message, WebSocket},
        Extension, TypedHeader, WebSocketUpgrade,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service},
    AddExtensionLayer, Router,
};
use notify::{watcher, Watcher, DebouncedEvent};
use std::sync::{mpsc::channel, Arc, Mutex};
use std::time::Duration;
use tower_http::services::ServeDir;

use crate::{CrateConfig, builder};

struct WsRelodState {
    update: bool,
}

impl WsRelodState { fn change(&mut self) { self.update = !self.update } }

pub async fn startup(config: CrateConfig) -> anyhow::Result<()> {

    log::info!("Starting development server 🚀");
        
    let (tx, rx) = channel();

    let mut watcher = watcher(tx, Duration::from_secs(2)).unwrap();
    watcher.watch(config.crate_dir.join("src").clone(), notify::RecursiveMode::Recursive).unwrap();

    let ws_reload_state = Arc::new(Mutex::new(WsRelodState { update: false }));

    let watcher_conf = config.clone();
    let watcher_ws_state = ws_reload_state.clone();
    tokio::spawn(async move {
        loop {
            if let Ok(v) = rx.recv() {
                match v {
                    DebouncedEvent::Create(_) | DebouncedEvent::Write(_) |
                    DebouncedEvent::Remove(_) | DebouncedEvent::Rename(_, _) => {
                        builder::build(&watcher_conf).unwrap();
                        watcher_ws_state.lock().unwrap().change();
                    },
                    _ => {}
                }
            }
        }    
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .fallback(
            get_service(ServeDir::new(config.out_dir)).handle_error(
                |error: std::io::Error| async move {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled internal error: {}", error),
                    )
                },
            ),
        )
        .layer(AddExtensionLayer::new(ws_reload_state.clone()));

    let port = "8080";
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
                socket.send(Message::Text(String::from("reload"))).await.unwrap();
                state.lock().unwrap().change();
            }
        }
    })
}
