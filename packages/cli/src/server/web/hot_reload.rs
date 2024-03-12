use crate::server::HotReloadState;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::IntoResponse,
    Extension,
};
use dioxus_hot_reload::HotReloadMsg;

pub async fn hot_reload_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<HotReloadState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| async move {
        let err = hotreload_loop(socket, state).await;

        if let Err(err) = err {
            log::error!("Hotreload receiver failed: {}", err);
        }
    })
}

async fn hotreload_loop(mut socket: WebSocket, state: HotReloadState) -> anyhow::Result<()> {
    log::info!("🔥 Hot Reload WebSocket connected");

    // update any rsx calls that changed before the websocket connected.
    log::info!("🔮 Finding updates since last compile...");

    let templates = state
        .file_map
        .lock()
        .unwrap()
        .map
        .values()
        .filter_map(|(_, template_slot)| *template_slot)
        .collect::<Vec<_>>();

    for template in templates {
        socket
            .send(Message::Text(serde_json::to_string(&template).unwrap()))
            .await?;
    }

    let mut rx = state.messages.subscribe();

    loop {
        if let Ok(msg) = rx.recv().await {
            let msg = match msg {
                HotReloadMsg::UpdateTemplate(template) => {
                    Message::Text(serde_json::to_string(&template).unwrap())
                }
                HotReloadMsg::UpdateAsset(_) => todo!(),
                HotReloadMsg::Shutdown => todo!(),
            };

            socket.send(msg).await?;
        }
    }
}
