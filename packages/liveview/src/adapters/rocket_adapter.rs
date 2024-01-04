use crate::{LiveViewError, LiveViewSocket};
use rocket::futures::{SinkExt, StreamExt};
use rocket_ws::{result::Error, stream::DuplexStream, Message};

/// Convert a rocket websocket into a LiveViewSocket
///
/// This is required to launch a LiveView app using the rocket web framework
pub fn rocket_socket(stream: DuplexStream) -> impl LiveViewSocket {
    stream
        .map(transform_rx)
        .with(transform_tx)
        .sink_map_err(|_| LiveViewError::SendingFailed)
}

fn transform_rx(message: Result<Message, Error>) -> Result<Vec<u8>, LiveViewError> {
    message
        .map_err(|_| LiveViewError::SendingFailed)?
        .into_text()
        .map(|s| s.into_bytes())
        .map_err(|_| LiveViewError::SendingFailed)
}

async fn transform_tx(message: Vec<u8>) -> Result<Message, Error> {
    Ok(Message::Text(String::from_utf8_lossy(&message).to_string()))
}
