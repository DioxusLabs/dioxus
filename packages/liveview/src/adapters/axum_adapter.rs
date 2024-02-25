use std::sync::Arc;

use crate::{interpreter_glue, LiveViewError, LiveViewSocket, LiveviewRouter};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::Html,
    routing::*,
    Router,
};
use futures_util::{SinkExt, StreamExt};

/// Convert an Axum WebSocket into a `LiveViewSocket`.
///
/// This is required to launch a LiveView app using the Axum web framework.
pub fn axum_socket(ws: WebSocket) -> impl LiveViewSocket {
    ws.map(transform_rx)
        .with(transform_tx)
        .sink_map_err(|_| LiveViewError::SendingFailed)
}

fn transform_rx(message: Result<Message, axum::Error>) -> Result<Vec<u8>, LiveViewError> {
    message
        .map_err(|_| LiveViewError::SendingFailed)?
        .into_text()
        .map(|s| s.into_bytes())
        .map_err(|_| LiveViewError::SendingFailed)
}

async fn transform_tx(message: Vec<u8>) -> Result<Message, axum::Error> {
    Ok(Message::Binary(message))
}

impl LiveviewRouter for Router {
    fn create_default_liveview_router() -> Self {
        Router::new()
    }

    fn with_virtual_dom(
        self,
        route: &str,
        app: impl Fn() -> dioxus_core::prelude::VirtualDom + Send + Sync + 'static,
    ) -> Self {
        let view = crate::LiveViewPool::new();

        let ws_path = format!("{}/ws", route);
        let title = crate::app_title();

        let index_page_with_glue = move |glue: &str| {
            Html(format!(
                r#"
        <!DOCTYPE html>
        <html>
            <head> <title>{title}</title>  </head>
            <body> <div id="main"></div> </body>
            {glue}
        </html>
        "#,
            ))
        };

        let app = Arc::new(app);

        self.route(
            &ws_path,
            get(move |ws: WebSocketUpgrade| async move {
                let app = app.clone();
                ws.on_upgrade(move |socket| async move {
                    _ = view
                        .launch_virtualdom(axum_socket(socket), move || app())
                        .await;
                })
            }),
        )
        .route(
            route,
            get(move || async move { index_page_with_glue(&interpreter_glue(&ws_path)) }),
        )
    }

    async fn start(self, address: impl Into<std::net::SocketAddr>) {
        let listener = tokio::net::TcpListener::bind(address.into()).await.unwrap();
        if let Err(err) = axum::serve(listener, self.into_make_service()).await {
            eprintln!("Failed to start axum server: {}", err);
        }
    }
}
