use futures_util::{SinkExt, StreamExt};
use salvo::ws::{Message, WebSocket};

use crate::{LiveViewError, LiveViewSocket};

/// Convert a salvo websocket into a LiveViewSocket
///
/// This is required to launch a LiveView app using the warp web framework
pub fn salvo_socket(ws: WebSocket) -> impl LiveViewSocket {
    ws.map(transform_rx)
        .with(transform_tx)
        .sink_map_err(|_| LiveViewError::SendingFailed)
}

fn transform_rx(message: Result<Message, salvo::Error>) -> Result<Vec<u8>, LiveViewError> {
    let as_bytes = message.map_err(|_| LiveViewError::SendingFailed)?;

    Ok(as_bytes.into())
}

async fn transform_tx(message: Vec<u8>) -> Result<Message, salvo::Error> {
    Ok(Message::text(String::from_utf8_lossy(&message).to_string()))
}
