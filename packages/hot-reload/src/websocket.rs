use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use dioxus_core::Template;
use futures_util::{pin_mut, FutureExt};
use tokio::sync::broadcast;

use crate::HotReloadMsg;

/// A extension trait with utilities for integrating Dioxus hot reloading with your Axum router.
pub trait HotReloadRouterExt<S> {
    /// Register the web RSX hot reloading endpoint. This will enable hot reloading for your application in debug mode when you call [`dioxus_hot_reload::hot_reload_init`].
    ///
    /// # Example
    /// ```rust
    /// #![allow(non_snake_case)]
    /// use dioxus_fullstack::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     hot_reload_init!();
    ///     let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    ///     axum::Server::bind(&addr)
    ///         .serve(
    ///             axum::Router::new()
    ///                 // Connect to hot reloading in debug mode
    ///                 .connect_hot_reload()
    ///                 .into_make_service(),
    ///         )
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    fn connect_hot_reload(self) -> Self;
}

impl<S> HotReloadRouterExt<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn connect_hot_reload(self) -> Self {
        self.nest(
            "/_dioxus",
            Router::new()
                .route(
                    "/ws",
                    get(|ws: axum::extract::WebSocketUpgrade| async {
                        ws.on_upgrade(|mut ws| async move {
                            let _ = ws.send(Message::Text("connected".into())).await;
                            loop {
                                if ws.recv().await.is_none() {
                                    break;
                                }
                            }
                        })
                    }),
                )
                .route("/hot_reload", get(hot_reload_handler)),
        )
    }
}

/// State that is shared between the websocket and the hot reloading watcher
#[derive(Clone)]
pub struct HotReloadReceiver {
    /// Hot reloading messages sent from the client
    // NOTE: We use a send broadcast channel to allow clones
    messages: broadcast::Sender<HotReloadMsg>,

    /// Any template updates that have happened since the last full render
    template_updates: SharedTemplateUpdates,
}

impl HotReloadReceiver {
    /// Create a new [`HotReloadReceiver`]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for HotReloadReceiver {
    fn default() -> Self {
        Self {
            messages: broadcast::channel(100).0,
            template_updates: Default::default(),
        }
    }
}

type SharedTemplateUpdates = Arc<Mutex<HashMap<&'static str, Template>>>;

impl HotReloadReceiver {
    /// Find all templates that have been updated since the last full render
    pub fn all_modified_templates(&self) -> Vec<Template> {
        self.template_updates
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    /// Send a hot reloading message to the client
    pub fn send_message(&self, msg: HotReloadMsg) {
        // Before we send the message, update the list of changed templates
        if let HotReloadMsg::UpdateTemplate(template) = msg {
            let mut template_updates = self.template_updates.lock().unwrap();
            template_updates.insert(template.name, template);
        }
        if let Err(err) = self.messages.send(msg) {
            tracing::error!("Failed to send hot reload message: {}", err);
        }
    }

    /// Subscribe to hot reloading messages
    pub fn subscribe(&self) -> broadcast::Receiver<HotReloadMsg> {
        self.messages.subscribe()
    }
}

pub async fn hot_reload_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<HotReloadReceiver>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| async move {
        let err = hotreload_loop(socket, state).await;

        if let Err(err) = err {
            tracing::error!("Hotreload receiver failed: {}", err);
        }
    })
}

async fn hotreload_loop(
    mut socket: WebSocket,
    state: HotReloadReceiver,
) -> Result<(), axum::Error> {
    tracing::info!("🔥 Hot Reload WebSocket connected");

    // update any rsx calls that changed before the websocket connected.
    // These templates will be sent down immediately so the page is in sync with the hotreloaded version
    // The compiled version will be different from the one we actually want to present
    for template in state.all_modified_templates() {
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
                e = _socket => {
                    if let Some(Err(e)) = e {
                        tracing::info!("🔥 Hot Reload WebSocket Error: {}", e);
                    } else {
                        tracing::info!("🔥 Hot Reload WebSocket Closed");
                    }
                    break;
                },
            };

            let Ok(msg) = msg else { break };

            match msg {
                HotReloadMsg::UpdateTemplate(template) => {
                    Message::Text(serde_json::to_string(&template).unwrap())
                }
                HotReloadMsg::UpdateAsset(asset) => {
                    Message::Text(format!("reload-asset: {}", asset.display()))
                }
                HotReloadMsg::Shutdown => {
                    tracing::info!("🔥 Hot Reload WebSocket shutting down");
                    break;
                }
            }
        };

        socket.send(msg).await?;
    }

    Ok(())
}
