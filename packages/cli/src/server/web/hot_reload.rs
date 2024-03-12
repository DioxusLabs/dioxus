use crate::server::HotReloadState;
use axum::{
    extract::{ws::Message, WebSocketUpgrade},
    response::IntoResponse,
    Extension,
};

pub async fn hot_reload_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<HotReloadState>,
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
