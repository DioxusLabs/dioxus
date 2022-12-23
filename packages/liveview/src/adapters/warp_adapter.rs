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

fn transform_rx(message: Result<Message, warp::Error>) -> Result<String, LiveViewError> {
    // destructure the message into the buffer we got from warp
    let msg = message
        .map_err(|_| LiveViewError::SendingFailed)?
        .into_bytes();

    // transform it back into a string, saving us the allocation
    let msg = String::from_utf8(msg).map_err(|_| LiveViewError::SendingFailed)?;

    Ok(msg)
}

async fn transform_tx(message: String) -> Result<Message, warp::Error> {
    Ok(Message::text(message))
}
