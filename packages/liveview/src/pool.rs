use crate::LiveViewError;
use dioxus_core::prelude::*;
use dioxus_html::HtmlEvent;
use futures_util::{pin_mut, SinkExt, StreamExt};
use std::time::Duration;
use tokio_util::task::LocalPoolHandle;

#[derive(Clone)]
pub struct LiveView {
    pub(crate) pool: LocalPoolHandle,
}

impl Default for LiveView {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveView {
    pub fn new() -> Self {
        LiveView {
            pool: LocalPoolHandle::new(16),
        }
    }
}

/// The primary event loop for the VirtualDom waiting for user input
///
/// This function makes it easy to integrate Dioxus LiveView with any socket-based framework.
///
/// As long as your framework can provide a Sink and Stream of Strings, you can use this function.
///
/// You might need to transform the error types of the web backend into the LiveView error type.
pub async fn liveview_eventloop<T>(
    app: Component<T>,
    props: T,
    ws_tx: impl SinkExt<String, Error = LiveViewError>,
    ws_rx: impl StreamExt<Item = Result<String, LiveViewError>>,
) -> Result<(), LiveViewError>
where
    T: Send + 'static,
{
    let mut vdom = VirtualDom::new_with_props(app, props);

    // todo: use an efficient binary packed format for this
    let edits = serde_json::to_string(&vdom.rebuild()).unwrap();

    // pin the futures so we can use select!
    pin_mut!(ws_tx);
    pin_mut!(ws_rx);

    ws_tx.send(edits).await?;

    // desktop uses this wrapper struct thing around the actual event itself
    // this is sorta driven by tao/wry
    #[derive(serde::Deserialize)]
    struct IpcMessage {
        params: HtmlEvent,
    }

    loop {
        tokio::select! {
            // poll any futures or suspense
            _ = vdom.wait_for_work() => {}

            evt = ws_rx.next() => {
                match evt {
                    Some(Ok(evt)) => {
                        let event: IpcMessage = serde_json::from_str(&evt).unwrap();
                        let event = event.params;
                        vdom.handle_event(&event.name, event.data.into_any(), event.element, event.bubbles);
                    }
                    Some(Err(_e)) => {
                        // log this I guess?
                        // when would we get an error here?
                    }
                    None => return Ok(()),
                }
            }
        }

        let edits = vdom
            .render_with_deadline(tokio::time::sleep(Duration::from_millis(10)))
            .await;

        ws_tx.send(serde_json::to_string(&edits).unwrap()).await?;
    }
}
