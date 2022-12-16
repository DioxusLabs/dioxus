use crate::{liveview_eventloop, LiveView, LiveViewError};
use dioxus_core::prelude::*;
use futures_util::{SinkExt, StreamExt};
use warp::ws::{Message, WebSocket};

impl LiveView {
    pub async fn upgrade_warp(
        self,
        ws: WebSocket,
        app: fn(Scope<()>) -> Element,
    ) -> Result<(), LiveViewError> {
        self.upgrade_warp_with_props(ws, app, ()).await
    }

    pub async fn upgrade_warp_with_props<T: Send + 'static>(
        self,
        ws: WebSocket,
        app: fn(Scope<T>) -> Element,
        props: T,
    ) -> Result<(), LiveViewError> {
        let (ws_tx, ws_rx) = ws.split();

        let ws_tx = ws_tx
            .with(transform_warp)
            .sink_map_err(|_| LiveViewError::SendingFailed);

        let ws_rx = ws_rx.map(transform_warp_rx);

        match self
            .pool
            .spawn_pinned(move || liveview_eventloop(app, props, ws_tx, ws_rx))
            .await
        {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(LiveViewError::SendingFailed),
        }
    }
}

fn transform_warp_rx(f: Result<Message, warp::Error>) -> Result<String, LiveViewError> {
    // destructure the message into the buffer we got from warp
    let msg = f.map_err(|_| LiveViewError::SendingFailed)?.into_bytes();

    // transform it back into a string, saving us the allocation
    let msg = String::from_utf8(msg).map_err(|_| LiveViewError::SendingFailed)?;

    Ok(msg)
}

async fn transform_warp(message: String) -> Result<Message, warp::Error> {
    Ok(Message::text(message))
}
