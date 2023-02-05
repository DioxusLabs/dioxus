use crate::WebSocketMsg as Msg;
use axum::extract::ws::{CloseFrame, Message as AxumMsg, WebSocket};
use futures_util::{SinkExt, StreamExt};

/// See [LiveViewError] for documentation
pub type AxumLiveViewError = crate::LiveViewError<axum::Error, axum::Error>;

/// See [crate::DisconnectReason] for documentation
pub type AxumDisconnectReason = crate::DisconnectReason<axum::Error, axum::Error>;

/// Convert a `axum` WebSocket into a LiveViewSocket
///
/// This is required to launch a LiveView app using the `axum` web framework
pub fn axum_socket(ws: WebSocket) -> impl crate::LiveViewSocket<axum::Error, axum::Error> {
    ws.filter_map(transform_rx)
        .with(transform_tx)
        .sink_map_err(|e| AxumLiveViewError::SendingMsgFailed(e))
}

async fn transform_rx(msg: Result<AxumMsg, axum::Error>) -> Option<Result<Msg, AxumLiveViewError>> {
    match msg {
        Err(e) => Some(Err(AxumLiveViewError::ReceivingMsgFailed(e))),
        Ok(AxumMsg::Ping(_) | AxumMsg::Pong(_)) => None, // See [adapters::WebSocketMsg]
        Ok(msg) => {
            let msg = match msg {
                AxumMsg::Ping(_) | AxumMsg::Pong(_) => unreachable!("see above"),
                AxumMsg::Text(text) => Msg::Text(text),
                AxumMsg::Binary(binary) => Msg::Binary(binary),
                AxumMsg::Close(None) => Msg::Close(None),
                AxumMsg::Close(Some(CloseFrame { code, reason })) => {
                    Msg::Close(Some(crate::CloseFrame { code, reason }))
                }
            };
            Some(Ok(msg))
        }
    }
}

async fn transform_tx(msg: Msg) -> Result<AxumMsg, axum::Error> {
    let msg = match msg {
        Msg::Text(text) => AxumMsg::Text(text),
        Msg::Binary(binary) => AxumMsg::Binary(binary),
        Msg::Close(None) => AxumMsg::Close(None),
        Msg::Close(Some(crate::CloseFrame { code, reason })) => {
            AxumMsg::Close(Some(CloseFrame { code, reason }))
        }
    };
    Ok(msg)
}
