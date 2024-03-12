use crate::{
    element::LiveviewElement,
    eval::init_eval,
    events::SerializedHtmlEventConverter,
    query::{QueryEngine, QueryResult},
    LiveViewError,
};
use dioxus_core::prelude::*;
use dioxus_html::{EventData, HtmlEvent, PlatformEventData};
use dioxus_interpreter_js::MutationState;
use futures_util::{pin_mut, SinkExt, StreamExt};
use serde::Serialize;
use std::{rc::Rc, time::Duration};
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
        // Set the event converter
        dioxus_html::set_event_converter(Box::new(SerializedHtmlEventConverter));

        LiveViewPool {
            pool: LocalPoolHandle::new(16),
        }
    }

    pub async fn launch(
        &self,
        ws: impl LiveViewSocket,
        app: fn() -> Element,
    ) -> Result<(), LiveViewError> {
        self.launch_with_props(ws, |app| app(), app).await
    }

    pub async fn launch_with_props<T: Clone + Send + 'static>(
        &self,
        ws: impl LiveViewSocket,
        app: fn(T) -> Element,
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
    SinkExt<Vec<u8>, Error = LiveViewError>
    + StreamExt<Item = Result<Vec<u8>, LiveViewError>>
    + Send
    + 'static
{
}

impl<S> LiveViewSocket for S where
    S: SinkExt<Vec<u8>, Error = LiveViewError>
        + StreamExt<Item = Result<Vec<u8>, LiveViewError>>
        + Send
        + 'static
{
}

/// The primary event loop for the VirtualDom waiting for user input
///
/// This function makes it easy to integrate Dioxus LiveView with any socket-based framework.
///
/// As long as your framework can provide a Sink and Stream of Bytes, you can use this function.
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

    let mut mutations = MutationState::default();

    // Create the a proxy for query engine
    let (query_tx, mut query_rx) = tokio::sync::mpsc::unbounded_channel();
    let query_engine = QueryEngine::new(query_tx);
    vdom.in_runtime(|| {
        ScopeId::ROOT.provide_context(query_engine.clone());
        init_eval();
    });

    // pin the futures so we can use select!
    pin_mut!(ws);

    if let Some(edits) = {
        vdom.rebuild(&mut mutations);
        take_edits(&mut mutations)
    } {
        // send the initial render to the client
        ws.send(edits).await?;
    }

    // desktop uses this wrapper struct thing around the actual event itself
    // this is sorta driven by tao/wry
    #[derive(serde::Deserialize, Debug)]
    #[serde(tag = "method", content = "params")]
    enum IpcMessage {
        #[serde(rename = "user_event")]
        Event(HtmlEvent),
        #[serde(rename = "query")]
        Query(QueryResult),
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
                    Some(Ok(b"__ping__")) => {
                        ws.send(text_frame("__pong__")).await?;
                    }
                    Some(Ok(evt)) => {
                        if let Ok(message) = serde_json::from_str::<IpcMessage>(&String::from_utf8_lossy(evt)) {
                            match message {
                                IpcMessage::Event(evt) => {
                                    // Intercept the mounted event and insert a custom element type
                                    if let EventData::Mounted = &evt.data {
                                        let element = LiveviewElement::new(evt.element, query_engine.clone());
                                        vdom.handle_event(
                                            &evt.name,
                                            Rc::new(PlatformEventData::new(Box::new(element))),
                                            evt.element,
                                            evt.bubbles,
                                        );
                                    } else {
                                        vdom.handle_event(
                                            &evt.name,
                                            evt.data.into_any(),
                                            evt.element,
                                            evt.bubbles,
                                        );
                                    }
                                }
                                IpcMessage::Query(result) => {
                                    query_engine.send(result);
                                },
                            }
                        }
                    }
                    // log this I guess? when would we get an error here?
                    Some(Err(_e)) => {}
                    None => return Ok(()),
                }
            }

            // handle any new queries
            Some(query) = query_rx.recv() => {
                ws.send(text_frame(&serde_json::to_string(&ClientUpdate::Query(query)).unwrap())).await?;
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

        // wait for suspense to resolve in a 10ms window
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(10)) => {}
            _ = vdom.wait_for_suspense() => {}
        }

        // render the vdom
        vdom.render_immediate(&mut mutations);

        if let Some(edits) = take_edits(&mut mutations) {
            ws.send(edits).await?;
        }
    }
}

fn text_frame(text: &str) -> Vec<u8> {
    let mut bytes = vec![0];
    bytes.extend(text.as_bytes());
    bytes
}

fn take_edits(mutations: &mut MutationState) -> Option<Vec<u8>> {
    // Add an extra one at the beginning to tell the shim this is a binary frame
    let mut bytes = vec![1];
    mutations.write_memory_into(&mut bytes);
    (bytes.len() > 1).then_some(bytes)
}

#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
enum ClientUpdate {
    #[serde(rename = "query")]
    Query(String),
}
