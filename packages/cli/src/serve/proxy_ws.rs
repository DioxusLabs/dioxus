use crate::logging::TraceSrc;
use axum::body::Body;
use axum::extract::ws::{CloseFrame as ACloseFrame, Message as AMessage};
use axum::extract::{FromRequestParts, WebSocketUpgrade};
use axum::http::request::Parts;
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use hyper::{Request, Response, Uri};
use tokio_tungstenite::tungstenite::protocol::{CloseFrame as TCloseFrame, Message as TMessage};

pub(crate) async fn proxy_websocket(
    mut parts: Parts,
    req: Request<Body>,
    backend_url: &Uri,
) -> Response<Body> {
    let ws = match WebSocketUpgrade::from_request_parts(&mut parts, &()).await {
        Ok(ws) => ws,
        Err(e) => return e.into_response(),
    };

    let new_host = backend_url.host().unwrap_or("localhost");
    let proxied_uri = format!(
        "{scheme}://{host}:{port}{path_and_query}",
        scheme = req.uri().scheme_str().unwrap_or("ws"),
        port = backend_url.port().unwrap(),
        host = new_host,
        path_and_query = req
            .uri()
            .path_and_query()
            .map(|f| f.to_string())
            .unwrap_or_default()
    );

    tracing::info!(dx_src = ?TraceSrc::Dev, "Proxying websocket connection {req:?} to {proxied_uri}");
    ws.on_upgrade(move |client_ws| async move {
        match handle_ws_connection(client_ws, &proxied_uri).await {
            Ok(()) => tracing::info!(dx_src = ?TraceSrc::Dev, "Websocket connection closed"),
            Err(e) => {
                tracing::error!(dx_src = ?TraceSrc::Dev, "Error proxying websocket connection: {e}")
            }
        }
    })
}

#[derive(thiserror::Error, Debug)]
enum WSError {
    #[error("Error connecting to server: {0}")]
    Connect(tokio_tungstenite::tungstenite::Error),
    #[error("Error sending message to server: {0}")]
    ToServer(tokio_tungstenite::tungstenite::Error),
    #[error("Error receiving message from server: {0}")]
    FromServer(tokio_tungstenite::tungstenite::Error),
    #[error("Error sending message to client: {0}")]
    ToClient(axum::Error),
    #[error("Error receiving message from client: {0}")]
    FromClient(axum::Error),
}

async fn handle_ws_connection(
    mut client_ws: axum::extract::ws::WebSocket,
    proxied_url: &str,
) -> Result<(), WSError> {
    let (mut server_ws, _) = tokio_tungstenite::connect_async(proxied_url)
        .await
        .map_err(WSError::Connect)?;

    let mut closed = false;
    while !closed {
        tokio::select! {
            Some(server_msg) = server_ws.next() => {
                closed = matches!(server_msg, Ok(TMessage::Close(..)));
                if let Some(msg) = server_msg.map_err(WSError::FromServer)?.into_msg() {
                    client_ws.send(msg).await.map_err(WSError::ToClient)?;
                }
            },
            Some(client_msg) = client_ws.next() => {
                closed = matches!(client_msg, Ok(AMessage::Close(..)));
                let msg = client_msg.map_err(WSError::FromClient)?.into_msg();
                server_ws.send(msg).await.map_err(WSError::ToServer)?;
            },
            else => break,
        }
    }

    Ok(())
}

trait IntoMsg<T> {
    fn into_msg(self) -> T;
}

impl IntoMsg<TMessage> for AMessage {
    fn into_msg(self) -> TMessage {
        use TMessage as TM;
        match self {
            Self::Text(v) => TM::Text(v.into()),
            Self::Binary(v) => TM::Binary(v.into()),
            Self::Ping(v) => TM::Ping(v.into()),
            Self::Pong(v) => TM::Pong(v.into()),
            Self::Close(v) => TM::Close(v.map(|cf| TCloseFrame {
                code: cf.code.into(),
                reason: cf.reason.into_owned().into(),
            })),
        }
    }
}

impl IntoMsg<Option<AMessage>> for TMessage {
    fn into_msg(self) -> Option<AMessage> {
        use AMessage as AM;
        Some(match self {
            Self::Text(v) => AM::Text(v.to_string()),
            Self::Binary(v) => AM::Binary(v.into()),
            Self::Ping(v) => AM::Ping(v.into()),
            Self::Pong(v) => AM::Pong(v.into()),
            Self::Close(v) => AM::Close(v.map(|cf| ACloseFrame {
                code: cf.code.into(),
                reason: cf.reason.to_string().into(),
            })),
            Self::Frame(_) => {
                // this variant should never be returned by next(), but handle it
                // gracefully by dropping it instead of panicking out of an abundance of caution
                tracing::warn!(dx_src = ?TraceSrc::Dev, "Dropping unexpected raw websocket frame");
                return None;
            }
        })
    }
}
