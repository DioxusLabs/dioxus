use std::{convert::Infallible, time::Duration};

use crate::{
    events::{self, IpcMessage},
    LiveView, LiveViewError,
};
use dioxus_core::prelude::*;
use dioxus_html::a;
use futures_util::{pin_mut, SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::task::LocalPoolHandle;
use warp::ws::{Message, WebSocket};

impl LiveView {
    pub async fn upgrade_warp(self, ws: WebSocket, app: fn(Scope<()>) -> Element) {}

    pub async fn upgrade_warp_with_props<T: Send + 'static>(
        self,
        ws: WebSocket,
        app: fn(Scope<T>) -> Element,
        props: T,
    ) {
        self.pool
            .spawn_pinned(move || liveview_eventloop(app, props, ws))
            .await;
    }
}

async fn liveview_eventloop<T>(
    app: Component<T>,
    props: T,
    mut ws: WebSocket,
) -> Result<(), LiveViewError>
where
    T: Send + 'static,
{
    let mut vdom = VirtualDom::new_with_props(app, props);
    let edits = serde_json::to_string(&vdom.rebuild()).unwrap();
    ws.send(Message::text(edits)).await.unwrap();

    loop {
        tokio::select! {
            // poll any futures or suspense
            _ = vdom.wait_for_work() => {}

            evt = ws.next() => {
                match evt {
                    Some(Ok(evt)) => {
                        if let Ok(evt) = evt.to_str() {
                            let IpcMessage { name, element, bubbles, data } = serde_json::from_str(evt).unwrap();

                            vdom.handle_event(&name, data, element, bubbles);
                        }
                    }
                    Some(Err(e)) => {
                        // log this I guess?
                        // when would we get an error here?
                    }
                    None => break,
                }

            }
        }

        let edits = vdom
            .render_with_deadline(tokio::time::sleep(Duration::from_millis(10)))
            .await;

        ws.send(Message::text(serde_json::to_string(&edits).unwrap()))
            .await
            .unwrap();
    }
    Ok(()) as Result<(), LiveViewError>
}
