use futures_util::{SinkExt, StreamExt};
use salvo::extra::ws::{Message, WebSocket};

use crate::{LiveViewError, LiveViewSocket};

/// Convert a salvo websocket into a LiveViewSocket
///
/// This is required to launch a LiveView app using the warp web framework
pub fn salvo_socket(ws: WebSocket) -> impl LiveViewSocket {
    ws.map(transform_rx)
        .with(transform_tx)
        .sink_map_err(|_| LiveViewError::SendingFailed)
}

fn transform_rx(message: Result<Message, salvo::Error>) -> Result<String, LiveViewError> {
    let as_bytes = message.map_err(|_| LiveViewError::SendingFailed)?;

    let msg = String::from_utf8(as_bytes.into_bytes()).map_err(|_| LiveViewError::SendingFailed)?;

    Ok(msg)
}

async fn transform_tx(message: String) -> Result<Message, salvo::Error> {
    Ok(Message::text(message))
}
