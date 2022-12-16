use crate::LiveViewError;
use dioxus_core::prelude::*;
use dioxus_html::HtmlEvent;
use futures_util::{pin_mut, SinkExt, StreamExt};
use std::time::Duration;
use tokio_util::task::LocalPoolHandle;

#[derive(Clone)]
pub struct LiveViewPool {
    pub(crate) pool: LocalPoolHandle,
}

impl Default for LiveViewPool {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveViewPool {
    pub fn new() -> Self {
        LiveViewPool {
            pool: LocalPoolHandle::new(16),
        }
    }

    pub async fn launch(
        &self,
        ws: impl LiveViewSocket,
        app: fn(Scope<()>) -> Element,
    ) -> Result<(), LiveViewError> {
        self.launch_with_props(ws, app, ()).await
    }

    pub async fn launch_with_props<T: Send + 'static>(
        &self,
        ws: impl LiveViewSocket,
        app: fn(Scope<T>) -> Element,
        props: T,
    ) -> Result<(), LiveViewError> {
        match self.pool.spawn_pinned(move || run(app, props, ws)).await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(LiveViewError::SendingFailed),
        }
    }
}

/// A LiveViewSocket is a Sink and Stream of Strings that Dioxus uses to communicate with the client
pub trait LiveViewSocket:
    SinkExt<String, Error = LiveViewError>
    + StreamExt<Item = Result<String, LiveViewError>>
    + Send
    + 'static
{
}

impl<S> LiveViewSocket for S where
    S: SinkExt<String, Error = LiveViewError>
        + StreamExt<Item = Result<String, LiveViewError>>
        + Send
        + 'static
{
}

/// The primary event loop for the VirtualDom waiting for user input
///
/// This function makes it easy to integrate Dioxus LiveView with any socket-based framework.
///
/// As long as your framework can provide a Sink and Stream of Strings, you can use this function.
///
/// You might need to transform the error types of the web backend into the LiveView error type.
pub async fn run<T>(
    app: Component<T>,
    props: T,
    ws: impl LiveViewSocket,
) -> Result<(), LiveViewError>
where
    T: Send + 'static,
{
    let mut vdom = VirtualDom::new_with_props(app, props);

    // todo: use an efficient binary packed format for this
    let edits = serde_json::to_string(&vdom.rebuild()).unwrap();

    // pin the futures so we can use select!
    pin_mut!(ws);

    // send the initial render to the client
    ws.send(edits).await?;

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

            evt = ws.next() => {
                match evt {
                    Some(Ok(evt)) => {
                        if let Ok(IpcMessage { params }) = serde_json::from_str::<IpcMessage>(&evt) {
                            vdom.handle_event(&params.name, params.data.into_any(), params.element, params.bubbles);
                        }
                    }
                    // log this I guess? when would we get an error here?
                    Some(Err(_e)) => {},
                    None => return Ok(()),
                }
            }
        }

        let edits = vdom
            .render_with_deadline(tokio::time::sleep(Duration::from_millis(10)))
            .await;

        ws.send(serde_json::to_string(&edits).unwrap()).await?;
    }
}
