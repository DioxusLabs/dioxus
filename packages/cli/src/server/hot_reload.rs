use std::sync::{Arc, Mutex};

use axum::{
    extract::{ws::Message, WebSocketUpgrade},
    response::IntoResponse,
    Extension, TypedHeader,
};
use dioxus_core::Template;
use dioxus_html::HtmlCtx;
use dioxus_rsx::hot_reload::FileMap;
use tokio::sync::broadcast;

use super::BuildManager;
use crate::CrateConfig;

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
