use crate::{
    LiveViewError,
    document::init_document,
    element::LiveviewElement,
    events::SerializedHtmlEventConverter,
    query::{QueryEngine, QueryResult},
};

use crate::{
    file_data::{FileStorage, LiveviewFormData},
    file_transfer::{FileCommand, PendingFileUpload, RemoteFile},
};
use dioxus_core::{Element, Event, ScopeId, VirtualDom, provide_context};
use dioxus_html::{EventData, HtmlEvent, PlatformEventData};
use dioxus_interpreter_js::MutationState;
use futures_util::{
    SinkExt, StreamExt,
    future::{AbortHandle, Abortable},
    pin_mut,
    stream::FuturesUnordered,
};
use serde::{Deserialize, Serialize};
use std::{any::Any, collections::HashMap, rc::Rc, sync::Arc};
use tokio_util::task::LocalPoolHandle;

#[derive(Deserialize, Debug)]
#[serde(tag = "method", content = "params")]
enum IpcMessage {
    #[serde(rename = "user_event")]
    Event(Box<HtmlEvent>),
    #[serde(rename = "file_event")]
    FileEvent {
        // Keep file IDs available even when the event's metadata cannot be deserialized.
        event: serde_json::Value,
        file_ids: Vec<u64>,
    },
    #[serde(rename = "file_upload_complete")]
    FileUploadComplete { token: String },
    #[serde(rename = "file_upload_error")]
    FileUploadError { token: String, error: String },
    #[serde(rename = "query")]
    Query(QueryResult),
}

fn dispatch_event(
    vdom: &VirtualDom,
    query_engine: &QueryEngine,
    event: Box<HtmlEvent>,
    files: Vec<FileStorage>,
) {
    let HtmlEvent {
        element,
        name,
        bubbles,
        data,
    } = *event;
    // Intercept the mounted event and insert a custom element type.
    let event = if let EventData::Mounted = &data {
        let element = LiveviewElement::new(element, query_engine.clone());
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(element))) as Rc<dyn Any>,
            bubbles,
        )
    } else if let EventData::Form(form) = data {
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(LiveviewFormData::new(
                form, files,
            )))) as Rc<dyn Any>,
            bubbles,
        )
    } else {
        Event::new(data.into_any(), bubbles)
    };
    vdom.runtime().handle_event(&name, event, element);
}

#[derive(Clone)]
pub struct LiveViewPool {
    pub(crate) pool: LocalPoolHandle,
    pub(crate) uploads: crate::upload::FileUploadRegistry,
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
            uploads: Default::default(),
        }
    }

    /// Set the upload storage limit in bytes for each LiveView connection.
    ///
    /// Unread files reserve their declared sizes without transferring contents. A read streams
    /// the file to temporary storage; the reservation lasts until its final handle or reader
    /// is dropped, or the transfer fails. Defaults to [`crate::DEFAULT_UPLOAD_STORAGE_LIMIT`] (1 GiB).
    pub fn with_upload_storage_limit(mut self, limit: u64) -> Self {
        self.uploads = self.uploads.with_limit(limit);
        self
    }

    /// Set the maximum number of unread, incoming, and retained files per LiveView connection.
    ///
    /// Each file counts from creation of its handle until its final handle or reader is dropped,
    /// or the transfer fails. Zero-byte files also count. Defaults to
    /// [`crate::DEFAULT_UPLOAD_FILE_LIMIT`] (1024 files).
    pub fn with_upload_file_limit(mut self, limit: usize) -> Self {
        self.uploads = self.uploads.with_file_limit(limit);
        self
    }

    /// Set how long a requested file transfer may wait for its HTTP request to start.
    ///
    /// The timeout begins on the first read, not on selection. Expired requests release their
    /// quota and report an error to readers. Once HTTP uploading starts, it remains valid until
    /// completion or cancellation. Defaults to [`crate::DEFAULT_UPLOAD_TIMEOUT`] (five minutes).
    pub fn with_upload_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.uploads = self.uploads.with_timeout(timeout);
        self
    }

    /// Run  an existing [`VirtualDom`] over a [`LiveViewSocket`] on the current executor.
    ///
    /// Use this method to integrate a preconfigured `VirtualDom` with any backend that
    /// provides a `LiveViewSocket`. Use [`Self::launch_virtualdom`] to construct and run the
    /// `VirtualDom` on LiveView's thread pool instead.
    ///
    /// For file uploads, pass a clone of this pool to the HTTP upload handler.
    pub async fn run(
        &self,
        vdom: VirtualDom,
        ws: impl LiveViewSocket,
    ) -> Result<(), LiveViewError> {
        run_with_uploads(vdom, ws, self.uploads.clone()).await
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
        let uploads = self.uploads.clone();
        match self
            .pool
            .spawn_pinned(move || run_with_uploads(make_app(), ws, uploads))
            .await
        {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(LiveViewError::SendingFailed),
        }
    }
}

/// A LiveViewSocket is a Sink and Stream of bytes that Dioxus uses to communicate with the client.
///
/// Most websockets from most HTTP frameworks can be converted into a LiveViewSocket using the appropriate adapter.
///
/// You can also convert your own socket into a LiveViewSocket by implementing this trait. This trait is an auto trait,
/// meaning that as long as your type implements Stream and Sink, you can use it as a LiveViewSocket.
///
/// For example, the axum implementation is a really small transform:
///
/// ```rust
/// use axum::extract::ws::{Message, WebSocket};
/// use dioxus_liveview::{LiveViewError, LiveViewSocket};
/// use futures_util::{SinkExt, StreamExt};
///
/// pub fn axum_socket(ws: WebSocket) -> impl LiveViewSocket {
///     ws.map(transform_rx)
///         .with(transform_tx)
///         .sink_map_err(|_| LiveViewError::SendingFailed)
/// }
///
/// fn transform_rx(message: Result<Message, axum::Error>) -> Result<Vec<u8>, LiveViewError> {
///     message
///         .map_err(|_| LiveViewError::SendingFailed)?
///         .into_text()
///         .map(|text| text.as_str().into())
///         .map_err(|_| LiveViewError::SendingFailed)
/// }
///
/// async fn transform_tx(message: Vec<u8>) -> Result<Message, axum::Error> {
///     Ok(Message::Binary(message.into()))
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

async fn run_with_uploads(
    mut vdom: VirtualDom,
    ws: impl LiveViewSocket,
    uploads: crate::upload::FileUploadRegistry,
) -> Result<(), LiveViewError> {
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

    let upload_session = uploads.new_session();
    let (file_tx, mut file_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut remote_files = HashMap::<u64, std::sync::Weak<RemoteFile>>::new();
    let mut pending_file_uploads = HashMap::<String, PendingFileUpload>::new();
    let mut upload_cleanups = FuturesUnordered::new();

    loop {
        #[cfg(all(feature = "devtools", debug_assertions))]
        let hot_reload_wait = hot_reload_rx.recv();
        #[cfg(not(all(feature = "devtools", debug_assertions)))]
        let hot_reload_wait: std::future::Pending<Option<()>> = std::future::pending();

        tokio::select! {
            // poll any futures or suspense
            _ = vdom.wait_for_work() => {}

            Some(result) = upload_cleanups.next() => {
                if let Ok(token) = result {
                    pending_file_uploads.remove(&token);
                    ws.send(text_frame(&serde_json::to_string(
                        &ClientUpdate::FileUploadCanceled { token }
                    ).unwrap())).await?;
                }
            }

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
                                    dispatch_event(&vdom, &query_engine, evt, Vec::new());
                                }
                                IpcMessage::FileEvent { event, file_ids } => {
                                    let event = match serde_json::from_value::<Box<HtmlEvent>>(event) {
                                        Ok(event) => event,
                                        Err(error) => {
                                            tracing::warn!(%error, "Invalid LiveView file event");
                                            for id in file_ids {
                                                let _ = file_tx.send(FileCommand::Release { id });
                                            }
                                            continue;
                                        }
                                    };
                                    let metadata = match &event.data {
                                        EventData::Form(form) => form.values.iter()
                                            .filter_map(|value| value.file.as_ref()).collect::<Vec<_>>(),
                                        _ => Vec::new(),
                                    };
                                    if metadata.len() != file_ids.len() {
                                        tracing::warn!("LiveView file event metadata does not match its handles");
                                        for id in file_ids {
                                            let _ = file_tx.send(FileCommand::Release { id });
                                        }
                                        continue;
                                    }
                                    remote_files.retain(|_, file| file.strong_count() > 0);
                                    let storage = file_ids.into_iter().zip(metadata).map(|(id, metadata)| {
                                        let file = if let Some(file) = remote_files.get(&id).and_then(std::sync::Weak::upgrade) {
                                            // The existing owner already retains the browser File. Release the
                                            // extra reference acquired when this event was sent.
                                            let _ = file_tx.send(FileCommand::Release { id });
                                            file
                                        } else {
                                            let file = Arc::new(RemoteFile::new(
                                                id, metadata.size, &upload_session, uploads.clone(), file_tx.clone(),
                                            ));
                                            remote_files.insert(id, Arc::downgrade(&file));
                                            file
                                        };
                                        FileStorage::Remote(file)
                                    }).collect();
                                    dispatch_event(&vdom, &query_engine, event, storage);
                                }
                                IpcMessage::FileUploadComplete { token } => {
                                    if let Some(upload) = pending_file_uploads.remove(&token) {
                                        upload.finish(None);
                                    }
                                }
                                IpcMessage::FileUploadError { token, error } => {
                                    if let Some(upload) = pending_file_uploads.remove(&token) {
                                        upload.finish(Some(error));
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

            Some(command) = file_rx.recv() => {
                let update = match command {
                    FileCommand::Read { id, mut upload } => {
                        if upload.is_closed() {
                            continue;
                        }
                        let token = upload.token.clone();
                        let cleanup_uploads = uploads.clone();
                        let cleanup_token = token.clone();
                        let (abort, registration) = AbortHandle::new_pair();
                        upload.cleanup = Some(abort);
                        upload_cleanups.push(Abortable::new(async move {
                            cleanup_uploads.wait_for_cleanup(&cleanup_token).await;
                            cleanup_token
                        }, registration));
                        pending_file_uploads.insert(token.clone(), upload);
                        ClientUpdate::FileUpload { id, token }
                    }
                    FileCommand::Cancel { token } => {
                        pending_file_uploads.remove(&token);
                        ClientUpdate::FileUploadCanceled { token }
                    }
                    FileCommand::Release { id } => ClientUpdate::FileRelease { id },
                };
                ws.send(text_frame(&serde_json::to_string(&update).unwrap())).await?;
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
    #[serde(rename = "file_upload")]
    FileUpload { id: u64, token: String },
    #[serde(rename = "file_upload_canceled")]
    FileUploadCanceled { token: String },
    #[serde(rename = "file_release")]
    FileRelease { id: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
    use futures_util::{Sink, Stream};
    use std::{
        pin::Pin,
        task::{Context, Poll},
        time::Duration,
    };

    struct TestSocket {
        incoming: UnboundedReceiver<Result<Vec<u8>, LiveViewError>>,
        outgoing: UnboundedSender<Vec<u8>>,
    }

    impl Stream for TestSocket {
        type Item = Result<Vec<u8>, LiveViewError>;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Pin::new(&mut self.incoming).poll_next(cx)
        }
    }

    impl Sink<Vec<u8>> for TestSocket {
        type Error = LiveViewError;

        fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Self::Error> {
            self.outgoing
                .unbounded_send(item)
                .map_err(|_| LiveViewError::SendingFailed)
        }

        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
    }

    struct TestConnection {
        tx: UnboundedSender<Result<Vec<u8>, LiveViewError>>,
        rx: UnboundedReceiver<Vec<u8>>,
        forms: tokio::sync::mpsc::UnboundedReceiver<Rc<dioxus_html::FormData>>,
        server: tokio::task::JoinHandle<Result<(), LiveViewError>>,
    }

    impl TestConnection {
        fn new(uploads: crate::upload::FileUploadRegistry) -> Self {
            fn app(
                forms: tokio::sync::mpsc::UnboundedSender<Rc<dioxus_html::FormData>>,
            ) -> Element {
                use dioxus::prelude::*;
                let mut visible = use_signal(|| true);
                if !visible() {
                    return rsx! {
                        div {}
                    };
                }
                rsx! {
                    input {
                        r#type: "file",
                        onchange: move |event| {
                            let _ = forms.send(event.data());
                        },
                        onreset: move |_| visible.set(false),
                    }
                }
            }
            let (tx, incoming) = unbounded();
            let (outgoing, rx) = unbounded();
            let (forms_tx, forms) = tokio::sync::mpsc::unbounded_channel();
            let socket = TestSocket { incoming, outgoing };
            let server = tokio::task::spawn_local(run_with_uploads(
                VirtualDom::new_with_props(app, forms_tx),
                socket,
                uploads,
            ));
            Self {
                tx,
                rx,
                forms,
                server,
            }
        }

        fn send(&self, method: &str, params: serde_json::Value) {
            self.tx
                .unbounded_send(Ok(serde_json::to_vec(&serde_json::json!({
                    "method": method, "params": params,
                }))
                .unwrap()))
                .unwrap();
        }

        async fn select(&mut self, files: &[(u64, &str, u64)]) -> Rc<dioxus_html::FormData> {
            let mut values = vec![serde_json::json!({"key": "description", "text": "upload"})];
            values.extend(files.iter().map(|(_, name, size)| {
                serde_json::json!({
                    "key": "files", "file": {
                        "name": name, "path": "", "size": size, "last_modified": 123,
                        "content_type": "application/octet-stream",
                    },
                })
            }));
            self.send(
                "file_event",
                serde_json::json!({
                    "file_ids": files.iter().map(|(id, _, _)| id).collect::<Vec<_>>(),
                    "event": {
                        "element": 1, "name": "change", "bubbles": true,
                        "data": { "values": values },
                    },
                }),
            );
            tokio::time::timeout(Duration::from_secs(1), self.forms.recv())
                .await
                .expect("file events must be delivered before any HTTP transfer")
                .unwrap()
        }

        async fn token(&mut self, id: u64) -> String {
            tokio::time::timeout(Duration::from_secs(1), async {
                loop {
                    let frame = self.rx.next().await.expect("connection should stay open");
                    if frame[0] != 0 {
                        continue;
                    }
                    let message: serde_json::Value = serde_json::from_slice(&frame[1..]).unwrap();
                    if message["type"] == "file_upload" {
                        assert_eq!(message["data"]["id"], id);
                        break message["data"]["token"].as_str().unwrap().to_string();
                    }
                }
            })
            .await
            .expect("reading a file must request its transfer")
        }

        async fn unmount_input(&mut self) {
            self.send(
                "user_event",
                serde_json::json!({
                    "element": 1, "name": "reset", "bubbles": true, "data": {"values": []},
                }),
            );
            tokio::time::timeout(Duration::from_secs(1), async {
                while let Some(frame) = self.rx.next().await {
                    if frame[0] == 1 {
                        return;
                    }
                }
                panic!("connection closed before removing the input");
            })
            .await
            .unwrap();
        }

        async fn close(self) {
            drop(self.tx);
            self.server.await.unwrap().unwrap();
        }
    }

    #[tokio::test]
    async fn concurrent_files_share_quota_and_clean_up_independently() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let uploads = crate::upload::FileUploadRegistry::new(6);
                let mut client = TestConnection::new(uploads.clone());
                let first = client
                    .select(&[(1, "first.bin", 3)])
                    .await
                    .files()
                    .remove(0);
                let second = client
                    .select(&[(2, "second.bin", 3)])
                    .await
                    .files()
                    .remove(0);
                let rejected = client
                    .select(&[(3, "rejected.bin", 1)])
                    .await
                    .files()
                    .remove(0);
                assert!(
                    rejected
                        .read_bytes()
                        .await
                        .unwrap_err()
                        .to_string()
                        .contains("upload data limit")
                );
                assert!(rejected.path().as_os_str().is_empty());
                drop(rejected);

                let mut first_read = first.byte_stream();
                let mut second_read = second.byte_stream();
                assert!(futures_util::poll!(first_read.next()).is_pending());
                let first_token = client.token(1).await;
                assert!(futures_util::poll!(second_read.next()).is_pending());
                let second_token = client.token(2).await;
                let mut writer = uploads.begin(&first_token, Some(3)).await.unwrap();
                writer
                    .write(bytes::Bytes::from_static(b"abc"))
                    .await
                    .unwrap();
                writer.finish().unwrap();

                // Dropping every owner cancels this transfer without affecting the other reader.
                drop(first);
                drop(first_read);
                assert!(matches!(
                    uploads.begin(&first_token, None).await,
                    Err(crate::upload::UploadError::UnknownOrExpired)
                ));
                let third = client
                    .select(&[(4, "third.bin", 3)])
                    .await
                    .files()
                    .remove(0);
                let mut writer = uploads.begin(&second_token, Some(3)).await.unwrap();
                writer
                    .write(bytes::Bytes::from_static(b"def"))
                    .await
                    .unwrap();
                writer.finish().unwrap();
                client.send(
                    "file_upload_complete",
                    serde_json::json!({"token": second_token}),
                );
                assert_eq!(second_read.next().await.unwrap().unwrap(), b"def"[..]);
                drop(second_read);
                drop(second);

                let mut third_read = third.byte_stream();
                assert!(futures_util::poll!(third_read.next()).is_pending());
                let third_token = client.token(4).await;
                let writer = uploads.begin(&third_token, Some(3)).await.unwrap();
                let unread = client
                    .select(&[(5, "unread.bin", 3)])
                    .await
                    .files()
                    .remove(0);
                client.close().await;
                writer.cancelled().await;
                assert!(third_read.next().await.unwrap().is_err());
                assert!(unread.read_bytes().await.is_err());
                assert!(matches!(
                    uploads.begin(&third_token, None).await,
                    Err(crate::upload::UploadError::UnknownOrExpired)
                ));
            })
            .await;
    }

    #[tokio::test(start_paused = true)]
    async fn an_expired_read_does_not_cancel_another_active_file() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let uploads =
                    crate::upload::FileUploadRegistry::new(3).with_timeout(Duration::from_secs(10));
                let mut client = TestConnection::new(uploads.clone());
                let first = client
                    .select(&[(1, "first.bin", 1)])
                    .await
                    .files()
                    .remove(0);
                let second = client
                    .select(&[(2, "second.bin", 2)])
                    .await
                    .files()
                    .remove(0);
                // Selecting files does not start the upload timeout.
                tokio::time::advance(Duration::from_secs(11)).await;
                let mut first_read = first.byte_stream();
                let mut second_read = second.byte_stream();
                assert!(futures_util::poll!(first_read.next()).is_pending());
                let first_token = client.token(1).await;
                assert!(futures_util::poll!(second_read.next()).is_pending());
                let second_token = client.token(2).await;
                let mut writer = uploads.begin(&second_token, Some(2)).await.unwrap();
                tokio::time::advance(Duration::from_secs(11)).await;
                assert!(first_read.next().await.unwrap().is_err());
                assert!(matches!(
                    uploads.begin(&first_token, None).await,
                    Err(crate::upload::UploadError::UnknownOrExpired)
                ));
                let third = client
                    .select(&[(3, "third.bin", 1)])
                    .await
                    .files()
                    .remove(0);
                writer
                    .write(bytes::Bytes::from_static(b"ok"))
                    .await
                    .unwrap();
                writer.finish().unwrap();
                client.send(
                    "file_upload_complete",
                    serde_json::json!({"token": second_token}),
                );
                assert_eq!(second_read.next().await.unwrap().unwrap(), b"ok"[..]);
                client.close().await;
                assert!(third.read_bytes().await.is_err());
            })
            .await;
    }

    #[tokio::test]
    async fn file_events_deliver_lazy_owned_handles() {
        tokio::task::LocalSet::new()
            .run_until(async {
                #[derive(Deserialize)]
                struct Values {
                    description: String,
                    files: Vec<dioxus_html::SerializedFileData>,
                }
                let uploads = crate::upload::FileUploadRegistry::default();
                let mut client = TestConnection::new(uploads.clone());
                let form = client
                    .select(&[(1, "hello.bin", 3), (2, "empty.bin", 0)])
                    .await;
                let parsed: Values = form.deserialize_values().unwrap();
                assert_eq!(parsed.description, "upload");
                assert_eq!(parsed.files.len(), 2);
                assert_eq!(parsed.files[0].name, "hello.bin");
                assert_eq!(parsed.files[1].name, "empty.bin");
                assert_eq!(parsed.files[0].size, 3);
                assert_eq!(parsed.files[1].size, 0);
                assert!(
                    parsed
                        .files
                        .iter()
                        .all(|file| file.path.as_os_str().is_empty())
                );
                let files: Vec<_> = form
                    .get("files")
                    .into_iter()
                    .map(|value| match value {
                        dioxus_html::FormValue::File(Some(file)) => file,
                        _ => panic!("expected a selected file"),
                    })
                    .collect();
                assert_eq!(files[0].name(), "hello.bin");
                assert_eq!(files[0].size(), 3);
                assert_eq!(files[0].last_modified(), 123);
                assert_eq!(
                    files[0].content_type().as_deref(),
                    Some("application/octet-stream")
                );
                assert!(files[0].path().as_os_str().is_empty());
                drop(form);
                let again = client
                    .select(&[(1, "hello.bin", 3)])
                    .await
                    .files()
                    .remove(0);
                let mut read = Box::pin(files[0].read_bytes());
                let mut another_read = Box::pin(files[0].read_bytes());
                assert!(futures_util::poll!(read.as_mut()).is_pending());
                assert!(futures_util::poll!(another_read.as_mut()).is_pending());
                assert!(files[0].path().as_os_str().is_empty());
                let token = client.token(1).await;
                // The existing read and the second, unread file both outlive their input.
                client.unmount_input().await;
                let mut writer = uploads.begin(&token, Some(3)).await.unwrap();
                writer
                    .write(bytes::Bytes::from_static(&[0, 255, 128]))
                    .await
                    .unwrap();
                writer.finish().unwrap();
                client.send("file_upload_complete", serde_json::json!({"token": token}));
                assert_eq!(read.await.unwrap().as_ref(), &[0, 255, 128]);
                assert_eq!(another_read.await.unwrap().as_ref(), &[0, 255, 128]);
                assert!(files[0].path().is_file());
                // The second event's handle reuses the first handle's transfer and storage.
                assert_eq!(again.path(), files[0].path());
                assert_eq!(again.read_bytes().await.unwrap().as_ref(), &[0, 255, 128]);
                drop(again);
                let mut read = Box::pin(files[1].read_bytes());
                assert!(futures_util::poll!(read.as_mut()).is_pending());
                let token = client.token(2).await;
                uploads
                    .begin(&token, Some(0))
                    .await
                    .unwrap()
                    .finish()
                    .unwrap();
                client.send("file_upload_complete", serde_json::json!({"token": token}));
                assert!(read.await.unwrap().is_empty());
                assert_eq!(files[1].name(), "empty.bin");
                let paths: Vec<_> = files.iter().map(|file| file.path()).collect();
                assert!(paths.iter().all(|path| path.is_file()));
                drop(files);
                assert!(paths.iter().all(|path| !path.exists()));
                // Parsed metadata remains a snapshot and does not retain uploaded storage.
                assert!(
                    parsed
                        .files
                        .iter()
                        .all(|file| file.path.as_os_str().is_empty())
                );
                client.close().await;
            })
            .await;
    }
}
