use crate::{LiveViewError, LiveViewSocket};
use futures_util::{SinkExt, StreamExt};
use warp::ws::{Message, WebSocket};

/// Convert a warp websocket into a LiveViewSocket
///
/// This is required to launch a LiveView app using the warp web framework
pub fn warp_socket(ws: WebSocket) -> impl LiveViewSocket {
    ws.map(transform_rx)
        .with(transform_tx)
        .sink_map_err(|_| LiveViewError::SendingFailed)
}

fn transform_rx(message: Result<Message, warp::Error>) -> Result<Vec<u8>, LiveViewError> {
    // destructure the message into the buffer we got from warp
    let msg = message
        .map_err(|_| LiveViewError::SendingFailed)?
        .into_bytes();

    Ok(msg)
}

async fn transform_tx(message: Vec<u8>) -> Result<Message, warp::Error> {
    Ok(Message::text(String::from_utf8_lossy(&message).to_string()))
}
