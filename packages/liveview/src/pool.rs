use crate::{
    element::LiveviewElement,
    eval::init_eval,
    events::SerializedHtmlEventConverter,
    query::{QueryEngine, QueryResult},
    LiveViewError,
};
use dioxus_core::{prelude::*, BorrowedAttributeValue, Mutations};
use dioxus_html::{event_bubbles, EventData, HtmlEvent, PlatformEventData};
use dioxus_interpreter_js::binary_protocol::Channel;
use futures_util::{pin_mut, SinkExt, StreamExt};
use rustc_hash::FxHashMap;
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

    let mut templates: FxHashMap<String, u16> = Default::default();
    let mut max_template_count = 0;

    // Create the a proxy for query engine
    let (query_tx, mut query_rx) = tokio::sync::mpsc::unbounded_channel();
    let query_engine = QueryEngine::new(query_tx);
    vdom.base_scope().provide_context(query_engine.clone());
    init_eval(vdom.base_scope());

    // pin the futures so we can use select!
    pin_mut!(ws);

    let mut edit_channel = Channel::default();
    if let Some(edits) = {
        let mutations = vdom.rebuild();
        apply_edits(
            mutations,
            &mut edit_channel,
            &mut templates,
            &mut max_template_count,
        )
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

        let edits = vdom
            .render_with_deadline(tokio::time::sleep(Duration::from_millis(10)))
            .await;

        if let Some(edits) = {
            apply_edits(
                edits,
                &mut edit_channel,
                &mut templates,
                &mut max_template_count,
            )
        } {
            ws.send(edits).await?;
        }
    }
}

fn text_frame(text: &str) -> Vec<u8> {
    let mut bytes = vec![0];
    bytes.extend(text.as_bytes());
    bytes
}

fn add_template(
    template: &Template<'static>,
    channel: &mut Channel,
    templates: &mut FxHashMap<String, u16>,
    max_template_count: &mut u16,
) {
    for root in template.roots.iter() {
        create_template_node(channel, root);
        templates.insert(template.name.to_owned(), *max_template_count);
    }
    channel.add_templates(*max_template_count, template.roots.len() as u16);

    *max_template_count += 1
}

fn create_template_node(channel: &mut Channel, v: &'static TemplateNode<'static>) {
    use TemplateNode::*;
    match v {
        Element {
            tag,
            namespace,
            attrs,
            children,
            ..
        } => {
            // Push the current node onto the stack
            match namespace {
                Some(ns) => channel.create_element_ns(tag, ns),
                None => channel.create_element(tag),
            }
            // Set attributes on the current node
            for attr in *attrs {
                if let TemplateAttribute::Static {
                    name,
                    value,
                    namespace,
                } = attr
                {
                    channel.set_top_attribute(name, value, namespace.unwrap_or_default())
                }
            }
            // Add each child to the stack
            for child in *children {
                create_template_node(channel, child);
            }
            // Add all children to the parent
            channel.append_children_to_top(children.len() as u16);
        }
        Text { text } => channel.create_raw_text(text),
        DynamicText { .. } => channel.create_raw_text("p"),
        Dynamic { .. } => channel.add_placeholder(),
    }
}

fn apply_edits(
    mutations: Mutations,
    channel: &mut Channel,
    templates: &mut FxHashMap<String, u16>,
    max_template_count: &mut u16,
) -> Option<Vec<u8>> {
    use dioxus_core::Mutation::*;
    if mutations.templates.is_empty() && mutations.edits.is_empty() {
        return None;
    }
    for template in mutations.templates {
        add_template(&template, channel, templates, max_template_count);
    }
    for edit in mutations.edits {
        match edit {
            AppendChildren { id, m } => channel.append_children(id.0 as u32, m as u16),
            AssignId { path, id } => channel.assign_id(path, id.0 as u32),
            CreatePlaceholder { id } => channel.create_placeholder(id.0 as u32),
            CreateTextNode { value, id } => channel.create_text_node(value, id.0 as u32),
            HydrateText { path, value, id } => channel.hydrate_text(path, value, id.0 as u32),
            LoadTemplate { name, index, id } => {
                if let Some(tmpl_id) = templates.get(name) {
                    channel.load_template(*tmpl_id, index as u16, id.0 as u32)
                }
            }
            ReplaceWith { id, m } => channel.replace_with(id.0 as u32, m as u16),
            ReplacePlaceholder { path, m } => channel.replace_placeholder(path, m as u16),
            InsertAfter { id, m } => channel.insert_after(id.0 as u32, m as u16),
            InsertBefore { id, m } => channel.insert_before(id.0 as u32, m as u16),
            SetAttribute {
                name,
                value,
                id,
                ns,
            } => match value {
                BorrowedAttributeValue::Text(txt) => {
                    channel.set_attribute(id.0 as u32, name, txt, ns.unwrap_or_default())
                }
                BorrowedAttributeValue::Float(f) => {
                    channel.set_attribute(id.0 as u32, name, &f.to_string(), ns.unwrap_or_default())
                }
                BorrowedAttributeValue::Int(n) => {
                    channel.set_attribute(id.0 as u32, name, &n.to_string(), ns.unwrap_or_default())
                }
                BorrowedAttributeValue::Bool(b) => channel.set_attribute(
                    id.0 as u32,
                    name,
                    if b { "true" } else { "false" },
                    ns.unwrap_or_default(),
                ),
                BorrowedAttributeValue::None => {
                    channel.remove_attribute(id.0 as u32, name, ns.unwrap_or_default())
                }
                _ => unreachable!(),
            },
            SetText { value, id } => channel.set_text(id.0 as u32, value),
            NewEventListener { name, id, .. } => {
                channel.new_event_listener(name, id.0 as u32, event_bubbles(name) as u8)
            }
            RemoveEventListener { name, id } => {
                channel.remove_event_listener(name, id.0 as u32, event_bubbles(name) as u8)
            }
            Remove { id } => channel.remove(id.0 as u32),
            PushRoot { id } => channel.push_root(id.0 as u32),
        }
    }

    // Add an extra one at the beginning to tell the shim this is a binary frame
    let mut bytes = vec![1];
    bytes.extend(channel.export_memory());
    channel.reset();
    Some(bytes)
}

#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
enum ClientUpdate {
    #[serde(rename = "query")]
    Query(String),
}
