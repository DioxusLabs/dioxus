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
use futures_util::{pin_mut, FutureExt};

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
        .flat_map(|v| v.templates.values().copied())
        .collect::<Vec<_>>();

    println!("previously changed: {:?}", templates);

    for template in templates {
        socket
            .send(Message::Text(serde_json::to_string(&template).unwrap()))
            .await?;
    }

    let mut rx = state.messages.subscribe();

    loop {
        let msg = {
            // Poll both the receiver and the socket
            //
            // This shuts us down if the connection is closed.
            let mut _socket = socket.recv().fuse();
            let mut _rx = rx.recv().fuse();

            pin_mut!(_socket, _rx);

            let msg = futures_util::select! {
                msg = _rx => msg,
                _ = _socket => break,
            };

            let Ok(msg) = msg else { break };

            println!("msg: {:?}", msg);

            match msg {
                HotReloadMsg::UpdateTemplate(template) => {
                    Message::Text(serde_json::to_string(&template).unwrap())
                }
                HotReloadMsg::UpdateAsset(asset) => {
                    Message::Text(format!("asset: {}", asset.display()))
                }
                HotReloadMsg::Shutdown => todo!(),
            }
        };

        socket.send(msg).await?;
    }

    Ok(())
}
