use crate::events;
use dioxus_core::prelude::*;
use futures_util::{pin_mut, SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::task::LocalPoolHandle;
use warp::ws::{Message, WebSocket};

impl crate::Liveview {
    pub async fn upgrade(&self, ws: warp::ws::WebSocket, app: fn(Scope) -> Element) {
        connect(ws, self.pool.clone(), app).await;
    }
}

pub async fn connect(ws: WebSocket, pool: LocalPoolHandle, app: fn(Scope) -> Element) {
    // Use a counter to assign a new unique ID for this user.

    // Split the socket into a sender and receive of messages.
    let (mut user_ws_tx, mut user_ws_rx) = ws.split();

    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (edits_tx, edits_rx) = mpsc::unbounded_channel();

    let mut edits_rx = UnboundedReceiverStream::new(edits_rx);
    let mut event_rx = UnboundedReceiverStream::new(event_rx);

    let vdom_fut = pool.spawn_pinned(move || async move {
        let mut vdom = VirtualDom::new(app);

        let edits = vdom.rebuild();

        let serialized = serde_json::to_string(&edits.edits).unwrap();
        edits_tx.send(serialized).unwrap();

        loop {
            use futures_util::future::{select, Either};

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
        use futures_util::future::{select, Either};

        match select(user_ws_rx.next(), edits_rx.next()).await {
            Either::Left((l, _)) => {
                if let Some(Ok(msg)) = l {
                    if let Ok(Some(msg)) = msg.to_str().map(events::parse_ipc_message) {
                        if msg.method == "user_event" {
                            let user_event = events::trigger_from_serialized(msg.params);
                            event_tx.send(user_event).unwrap();
                        }
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
                    if user_ws_tx.send(Message::text(edits)).await.is_err() {
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
