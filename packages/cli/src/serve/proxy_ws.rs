use crate::logging::TraceSrc;
use crate::serve::proxy::handle_proxy_error;
use anyhow::Context;
use axum::body::Body;
use axum::extract::ws::{CloseFrame as ClientCloseFrame, Message as ClientMessage};
use axum::extract::{FromRequestParts, WebSocketUpgrade};
use axum::http::request::Parts;
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use hyper::{Request, Response, Uri};
use tokio_tungstenite::tungstenite::protocol::{
    CloseFrame as ServerCloseFrame, Message as ServerMessage,
};

pub(crate) async fn proxy_websocket(
    mut parts: Parts,
    req: Request<Body>,
    backend_url: &Uri,
) -> Result<Response<Body>, Response<Body>> {
    let ws = WebSocketUpgrade::from_request_parts(&mut parts, &())
        .await
        .map_err(IntoResponse::into_response)?;

    tracing::trace!(dx_src = ?TraceSrc::Dev, "Proxying websocket connection {req:?}");
    let proxied_request = into_proxied_request(req, backend_url).map_err(handle_proxy_error)?;
    tracing::trace!(dx_src = ?TraceSrc::Dev, "Connection proxied to {proxied_uri}", proxied_uri = proxied_request.uri());

    Ok(ws.on_upgrade(move |client_ws| async move {
        match handle_ws_connection(client_ws, proxied_request).await {
            Ok(()) => tracing::trace!(dx_src = ?TraceSrc::Dev, "Websocket connection closed"),
            Err(e) => {
                tracing::error!(dx_src = ?TraceSrc::Dev, "Error proxying websocket connection: {e}")
            }
        }
    }))
}

fn into_proxied_request(
    req: Request<Body>,
    backend_url: &Uri,
) -> crate::Result<tokio_tungstenite::tungstenite::handshake::client::Request> {
    // ensure headers from original request are preserved
    let (mut request_parts, _) = req.into_parts();
    let mut uri_parts = request_parts.uri.into_parts();
    uri_parts.scheme = uri_parts.scheme.or("ws".parse().ok());
    uri_parts.authority = backend_url.authority().cloned();
    request_parts.uri = Uri::from_parts(uri_parts).context("Could not construct proxy URI")?;
    Ok(Request::from_parts(request_parts, ()))
}

#[derive(thiserror::Error, Debug)]
enum WsError {
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
    proxied_request: tokio_tungstenite::tungstenite::handshake::client::Request,
) -> Result<(), WsError> {
    let (mut server_ws, _) = tokio_tungstenite::connect_async(proxied_request)
        .await
        .map_err(WsError::Connect)?;

    let mut closed = false;
    while !closed {
        tokio::select! {
            Some(server_msg) = server_ws.next() => {
                closed = matches!(server_msg, Ok(ServerMessage::Close(..)));
                match server_msg.map_err(WsError::FromServer)?.into_msg() {
                    Ok(msg) => client_ws.send(msg).await.map_err(WsError::ToClient)?,
                    Err(UnexpectedRawFrame) => tracing::warn!(dx_src = ?TraceSrc::Dev, "Dropping unexpected raw websocket frame"),
                }
            },
            Some(client_msg) = client_ws.next() => {
                closed = matches!(client_msg, Ok(ClientMessage::Close(..)));
                let Ok(msg) = client_msg.map_err(WsError::FromClient)?.into_msg();
                server_ws.send(msg).await.map_err(WsError::ToServer)?;
            },
            else => break,
        }
    }

    Ok(())
}

trait IntoMsg<T> {
    type Error;
    fn into_msg(self) -> Result<T, Self::Error>;
}

impl IntoMsg<ServerMessage> for ClientMessage {
    type Error = std::convert::Infallible;
    fn into_msg(self) -> Result<ServerMessage, Self::Error> {
        use ServerMessage as SM;
        Ok(match self {
            Self::Text(v) => SM::Text(v.as_str().into()),
            Self::Binary(v) => SM::Binary(v),
            Self::Ping(v) => SM::Ping(v),
            Self::Pong(v) => SM::Pong(v),
            Self::Close(v) => SM::Close(v.map(|cf| ServerCloseFrame {
                code: cf.code.into(),
                reason: cf.reason.as_str().into(),
            })),
        })
    }
}

struct UnexpectedRawFrame;
impl IntoMsg<ClientMessage> for ServerMessage {
    type Error = UnexpectedRawFrame;
    fn into_msg(self) -> Result<ClientMessage, Self::Error> {
        use ClientMessage as CM;
        Ok(match self {
            Self::Text(v) => CM::Text(v.as_str().into()),
            Self::Binary(v) => CM::Binary(v),
            Self::Ping(v) => CM::Ping(v),
            Self::Pong(v) => CM::Pong(v),
            Self::Close(v) => CM::Close(v.map(|cf| ClientCloseFrame {
                code: cf.code.into(),
                reason: cf.reason.as_str().into(),
            })),
            Self::Frame(_) => {
                // this variant should never be returned by next(), but handle it
                // gracefully by dropping it instead of panicking out of an abundance of caution
                return Err(UnexpectedRawFrame);
            }
        })
    }
}
