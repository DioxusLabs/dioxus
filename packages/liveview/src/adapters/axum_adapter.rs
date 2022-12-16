use crate::{LiveViewError, LiveViewSocket};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};

/// Convert a warp websocket into a LiveViewSocket
///
/// This is required to launch a LiveView app using the warp web framework
pub fn axum_socket(ws: WebSocket) -> impl LiveViewSocket {
    ws.map(transform_rx)
        .with(transform_tx)
        .sink_map_err(|_| LiveViewError::SendingFailed)
}

fn transform_rx(message: Result<Message, axum::Error>) -> Result<String, LiveViewError> {
    message
        .map_err(|_| LiveViewError::SendingFailed)?
        .into_text()
        .map_err(|_| LiveViewError::SendingFailed)
}

async fn transform_tx(message: String) -> Result<Message, axum::Error> {
    Ok(Message::Text(message))
}

