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
        self.launch_virtualdom(ws, move || VirtualDom::new_with_props(app, props))
            .await
    }

    pub async fn launch_virtualdom<F: FnOnce() -> VirtualDom + Send + 'static>(
        &self,
        ws: impl LiveViewSocket,
        make_app: F,
    ) -> Result<(), LiveViewError> {
        match self.pool.spawn_pinned(move || run(make_app(), ws)).await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(LiveViewError::SendingFailed),
        }
    }
}

/// A LiveViewSocket is a Sink and Stream of Strings that Dioxus uses to communicate with the client
///
/// Most websockets from most HTTP frameworks can be converted into a LiveViewSocket using the appropriate adapter.
///
/// You can also convert your own socket into a LiveViewSocket by implementing this trait. This trait is an auto trait,
/// meaning that as long as your type implements Stream and Sink, you can use it as a LiveViewSocket.
///
/// For example, the axum implementation is a really small transform:
///
/// ```rust, ignore
/// pub fn axum_socket(ws: WebSocket) -> impl LiveViewSocket {
///     ws.map(transform_rx)
///         .with(transform_tx)
///         .sink_map_err(|_| LiveViewError::SendingFailed)
/// }
///
/// fn transform_rx(message: Result<Message, axum::Error>) -> Result<String, LiveViewError> {
///     message
///         .map_err(|_| LiveViewError::SendingFailed)?
///         .into_text()
///         .map_err(|_| LiveViewError::SendingFailed)
/// }
///
/// async fn transform_tx(message: String) -> Result<Message, axum::Error> {
///     Ok(Message::Text(message))
/// }
/// ```
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
pub async fn run(mut vdom: VirtualDom, ws: impl LiveViewSocket) -> Result<(), LiveViewError> {
    #[cfg(all(feature = "hot-reload", debug_assertions))]
    let mut hot_reload_rx = {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        dioxus_hot_reload::connect(move |template| {
            let _ = tx.send(template);
        });
        rx
    };

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
        #[cfg(all(feature = "hot-reload", debug_assertions))]
        let hot_reload_wait = hot_reload_rx.recv();
        #[cfg(not(all(feature = "hot-reload", debug_assertions)))]
        let hot_reload_wait: std::future::Pending<Option<()>> = std::future::pending();

        tokio::select! {
            // poll any futures or suspense
            _ = vdom.wait_for_work() => {}

            evt = ws.next() => {
                match evt.as_ref().map(|o| o.as_deref()) {
                    // respond with a pong every ping to keep the websocket alive
                    Some(Ok("__ping__")) => {
                        ws.send("__pong__".to_string()).await?;
                    }
                    Some(Ok(evt)) => {
                        if let Ok(IpcMessage { params }) = serde_json::from_str::<IpcMessage>(evt) {
                            vdom.handle_event(&params.name, params.data.into_any(), params.element, params.bubbles);
                        }
                    }
                    // log this I guess? when would we get an error here?
                    Some(Err(_e)) => {},
                    None => return Ok(()),
                }
            }

            Some(msg) = hot_reload_wait => {
                #[cfg(all(feature = "hot-reload", debug_assertions))]
                match msg{
                    dioxus_hot_reload::HotReloadMsg::UpdateTemplate(new_template) => {
                        vdom.replace_template(new_template);
                    }
                    dioxus_hot_reload::HotReloadMsg::Shutdown => {
                        std::process::exit(0);
                    },
                }
                #[cfg(not(all(feature = "hot-reload", debug_assertions)))]
                let () = msg;
            }
        }

        let edits = vdom
            .render_with_deadline(tokio::time::sleep(Duration::from_millis(10)))
            .await;

        ws.send(serde_json::to_string(&edits).unwrap()).await?;
    }
}
