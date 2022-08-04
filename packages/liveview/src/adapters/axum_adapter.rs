use crate::events;
use axum::extract::ws::{Message, WebSocket};
use dioxus_core::prelude::*;
use futures_util::{
    future::{select, Either},
    pin_mut, SinkExt, StreamExt,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::task::LocalPoolHandle;

impl crate::Liveview {
    pub async fn upgrade_axum(&self, ws: WebSocket, app: fn(Scope) -> Element) {
        connect(ws, self.pool.clone(), app, ()).await;
    }

    pub async fn upgrade_axum_with_props<T>(
        &self,
        ws: WebSocket,
        app: fn(Scope<T>) -> Element,
        props: T,
    ) where
        T: Send + Sync + 'static,
    {
        connect(ws, self.pool.clone(), app, props).await;
    }
}

pub async fn connect<T>(
    socket: WebSocket,
    pool: LocalPoolHandle,
    app: fn(Scope<T>) -> Element,
    props: T,
) where
    T: Send + Sync + 'static,
{
    let (mut user_ws_tx, mut user_ws_rx) = socket.split();
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (edits_tx, edits_rx) = mpsc::unbounded_channel();
    let mut edits_rx = UnboundedReceiverStream::new(edits_rx);
    let mut event_rx = UnboundedReceiverStream::new(event_rx);
    let vdom_fut = pool.clone().spawn_pinned(move || async move {
        let mut vdom = VirtualDom::new_with_props(app, props);
        let edits = vdom.rebuild();
        let serialized = serde_json::to_string(&edits.edits).unwrap();
        edits_tx.send(serialized).unwrap();
        loop {
            let new_event = {
                let vdom_fut = vdom.wait_for_work();
                pin_mut!(vdom_fut);
                match select(event_rx.next(), vdom_fut).await {
                    Either::Left((l, _)) => l,
                    Either::Right((_, _)) => None,
                }
            };
            if let Some(new_event) = new_event {
                vdom.handle_message(dioxus_core::SchedulerMsg::Event(new_event));
            } else {
                let mutations = vdom.work_with_deadline(|| false);
                for mutation in mutations {
                    let edits = serde_json::to_string(&mutation.edits).unwrap();
                    edits_tx.send(edits).unwrap();
                }
            }
        }
    });
    loop {
        match select(user_ws_rx.next(), edits_rx.next()).await {
            Either::Left((l, _)) => {
                if let Some(Ok(msg)) = l {
                    if let Ok(Some(msg)) = msg.to_text().map(events::parse_ipc_message) {
                        let user_event = events::trigger_from_serialized(msg.params);
                        event_tx.send(user_event).unwrap();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            Either::Right((edits, _)) => {
                if let Some(edits) = edits {
                    // send the edits to the client
                    if user_ws_tx.send(Message::Text(edits)).await.is_err() {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
    }
    vdom_fut.abort();
}
