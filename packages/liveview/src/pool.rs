use crate::{
    document::init_document,
    element::LiveviewElement,
    events::SerializedHtmlEventConverter,
    query::{QueryEngine, QueryResult},
    LiveViewError,
};

use dioxus_core::{provide_context, Element, Event, ScopeId, VirtualDom};
use dioxus_html::{EventData, HtmlEvent, PlatformEventData};
use dioxus_interpreter_js::MutationState;
use futures_util::{pin_mut, SinkExt, StreamExt};
use serde::Serialize;
use std::{any::Any, rc::Rc};
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
            pool: LocalPoolHandle::new(
                std::thread::available_parallelism()
                    .map(usize::from)
                    .unwrap_or(1),
            ),
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
    #[cfg(all(feature = "devtools", debug_assertions))]
    let mut hot_reload_rx = {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        dioxus_devtools::connect(move |template| _ = tx.send(template));
        rx
    };

    let mut mutations = MutationState::default();

    // Create the a proxy for query engine
    let (query_tx, mut query_rx) = tokio::sync::mpsc::unbounded_channel();
    let query_engine = QueryEngine::new(query_tx);
    vdom.runtime().in_scope(ScopeId::ROOT, || {
        provide_context(query_engine.clone());
        init_document();
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
        Event(Box<HtmlEvent>),
        #[serde(rename = "query")]
        Query(QueryResult),
    }

    loop {
        #[cfg(all(feature = "devtools", debug_assertions))]
        let hot_reload_wait = hot_reload_rx.recv();
        #[cfg(not(all(feature = "devtools", debug_assertions)))]
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
                                    let event = if let EventData::Mounted = &evt.data {
                                        let element = LiveviewElement::new(evt.element, query_engine.clone());
                                        Event::new(
                                            Rc::new(PlatformEventData::new(Box::new(element))) as Rc<dyn Any>,
                                            evt.bubbles,
                                        )
                                    } else {
                                        Event::new(
                                            evt.data.into_any(),
                                            evt.bubbles,
                                        )
                                    };
                                    vdom.runtime().handle_event(
                                        &evt.name,
                                        event,
                                        evt.element,
                                    );
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
                #[cfg(all(feature = "devtools", debug_assertions))]
                match msg {
                    dioxus_devtools::DevserverMsg::HotReload(msg)=> {
                        dioxus_devtools::apply_changes(&vdom, &msg);
                    }
                    dioxus_devtools::DevserverMsg::Shutdown => {
                        std::process::exit(0);
                    },
                    dioxus_devtools::DevserverMsg::FullReloadCommand
                    | dioxus_devtools::DevserverMsg::FullReloadStart
                    | dioxus_devtools::DevserverMsg::FullReloadFailed => {
                        // usually only web gets this message - what are we supposed to do?
                        // Maybe we could just binary patch ourselves in place without losing window state?
                    },
                    _ => {}
                }
                #[cfg(not(all(feature = "devtools", debug_assertions)))]
                let () = msg;
            }
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
