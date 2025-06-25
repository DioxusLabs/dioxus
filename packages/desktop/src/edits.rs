use dioxus_interpreter_js::MutationState;
use futures_channel::mpsc::UnboundedSender;
use futures_channel::oneshot;
use futures_util::{FutureExt, StreamExt};
use pollster::FutureExt as _;
use std::collections::{HashMap, VecDeque};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::AtomicU32;
use std::thread::spawn;
use std::{
    net::IpAddr,
    sync::{Arc, RwLock},
};
use tungstenite::handshake::server::Request;
use tungstenite::{accept_hdr, WebSocket};

/// Bind a listener to any port that is available on the given address.
fn get_available_port(address: IpAddr) -> Option<u16> {
    // Otherwise, try to bind to any port and return the first one we can
    TcpListener::bind((address, 0))
        .and_then(|listener| listener.local_addr().map(|f| f.port()))
        .ok()
}

/// A message to send to the webview to apply edits.
pub(crate) struct WebviewEditMessage {
    /// The websocket location that the webview is connected to
    pub(crate) location: WebviewWebsocketLocation,
    /// The serialized edits to apply to the webview
    pub(crate) edits: Vec<u8>,
}

/// The websocket listener that the webview will connect to in order to receive edits and send requests. There
/// is only one websocket listener per application even if there are multiple windows so we don't use all the
/// open ports.
#[derive(Clone)]
pub(crate) struct WryWebsocket {
    port: u16,
    max_webview_id: Arc<AtomicU32>,
    connections: Arc<RwLock<HashMap<WebviewWebsocketLocation, WebviewConnectionState>>>,
}

impl WryWebsocket {
    pub(crate) fn new() -> Self {
        let connections = Arc::new(RwLock::new(HashMap::new()));

        let ip = IpAddr::from([127, 0, 0, 1]);
        let port = get_available_port(ip).unwrap_or(9001);

        let server = TcpListener::bind((ip, port)).unwrap();
        spawn({
            let connections = connections.clone();
            move || {
                while let Ok((stream, _)) = server.accept() {
                    let mut location = None;
                    // Accept the websocket connection while reading the path
                    let websocket = accept_hdr(stream, |req: &Request, res| {
                        // Try to parse the webview id from the path
                        if let Some(webview_id) = req.uri().path().strip_prefix('/') {
                            if let Ok(webview_id) = webview_id.parse::<u32>() {
                                tracing::info!("Webview ID: {}", webview_id);
                                location = Some(WebviewWebsocketLocation::new(port, webview_id));
                            }
                        }
                        tracing::info!("WebSocket connection from: {}", req.uri());
                        Ok(res)
                    });
                    let websocket = match websocket {
                        Ok(ws) => ws,
                        Err(e) => {
                            tracing::error!("Error accepting websocket connection: {}", e);
                            continue;
                        }
                    };
                    let location = match location {
                        Some(loc) => loc,
                        None => {
                            tracing::error!("WebSocket connection without a valid webview ID");
                            continue;
                        }
                    };
                    // Handle the websocket connection in a separate thread
                    let mut connection = WebviewConnection::new(websocket);
                    let mut connections = connections.write().unwrap();
                    // If there are pending edits, send them to the new connection
                    if let Some(state) = connections.remove(&location) {
                        if let WebviewConnectionState::Pending(mut pending) = state {
                            while let Some((edit, response_sender)) = pending.pop_front() {
                                tracing::info!("flushing pending edits to new connection");
                                _ = connection.send_edits_with_response(edit, response_sender);
                            }
                        }
                    }
                    connections.insert(location, WebviewConnectionState::from(connection));
                }
            }
        });

        Self {
            connections,
            port,
            max_webview_id: Default::default(),
        }
    }

    pub(crate) fn create_queue(&self) -> WryQueue {
        let webview_id = self
            .max_webview_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let location = WebviewWebsocketLocation::new(self.port, webview_id);
        let websocket = self.clone();

        WryQueue {
            inner: Arc::new(RwLock::new(WryQueueInner {
                location,
                websocket,
                edits_in_progress: None,
                mutation_state: MutationState::default(),
            })),
        }
    }

    fn send_edits(&mut self, edits: WebviewEditMessage) -> oneshot::Receiver<()> {
        tracing::info!("Sending edits to webview: {:?}", edits.location.webview_id);
        let mut connections_mut = self.connections.write().unwrap();
        let connection = connections_mut.entry(edits.location).or_default();
        connection.send_edits(edits.edits)
    }
}

enum WebviewConnectionState {
    Pending(VecDeque<(Vec<u8>, oneshot::Sender<()>)>),
    Connected(WebviewConnection),
}

impl From<WebviewConnection> for WebviewConnectionState {
    fn from(connection: WebviewConnection) -> Self {
        WebviewConnectionState::Connected(connection)
    }
}

impl Default for WebviewConnectionState {
    fn default() -> Self {
        WebviewConnectionState::Pending(VecDeque::new())
    }
}

impl WebviewConnectionState {
    /// Send edits to the webview connection
    pub(crate) fn send_edits(&mut self, edits: Vec<u8>) -> oneshot::Receiver<()> {
        match self {
            WebviewConnectionState::Pending(pending) => {
                // If the connection is pending, add the edits to the pending list
                let (response_sender, response_receiver) = oneshot::channel();
                pending.push_back((edits, response_sender));
                response_receiver
            }
            WebviewConnectionState::Connected(connection) => connection.send_edits(edits),
        }
    }
}

struct WebviewConnection {
    edits_outgoing: UnboundedSender<(Vec<u8>, oneshot::Sender<()>)>,
}

impl WebviewConnection {
    pub(crate) fn new(socket: WebSocket<TcpStream>) -> Self {
        let (edits_outgoing, mut edits_incoming) =
            futures_channel::mpsc::unbounded::<(Vec<u8>, oneshot::Sender<()>)>();
        // Spawn a task to handle the websocket connection
        spawn({
            move || {
                let mut socket = socket;
                // Wait until there are edits ready to send
                while let Some((edits, response)) = edits_incoming.next().block_on() {
                    tracing::info!("sending edits to webview");
                    // Send the edits to the webview
                    if let Err(e) = socket.send(tungstenite::Message::Binary(edits.into())) {
                        tracing::error!("Error sending edits to webview: {}", e);
                        break;
                    }
                    tracing::info!("waiting for read");
                    // Wait for the webview to apply the edits
                    while let Ok(msg) = socket.read() {
                        tracing::info!("received message from webview: {:?}", msg);
                        match msg {
                            tungstenite::Message::Binary(_) => {
                                // We expect the webview to send a binary message when it has applied the edits
                                // This is a signal that we can continue processing
                                break;
                            }
                            tungstenite::Message::Close(_) => {
                                return;
                            }
                            _ => {}
                        }
                    }
                    // Notify that the edits have been applied
                    if response.send(()).is_err() {
                        tracing::error!("Error sending edits applied notification");
                    }
                }
            }
        });

        Self { edits_outgoing }
    }

    /// Send a message to the webview to apply edits
    pub(crate) fn send_edits(&mut self, edits: Vec<u8>) -> oneshot::Receiver<()> {
        let (response_sender, response_receiver) = oneshot::channel();
        self.send_edits_with_response(edits, response_sender);
        response_receiver
    }

    /// Send edits with a response channel
    pub(crate) fn send_edits_with_response(
        &mut self,
        edits: Vec<u8>,
        response_sender: oneshot::Sender<()>,
    ) {
        _ = self.edits_outgoing.unbounded_send((edits, response_sender));
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct WebviewWebsocketLocation {
    /// The port the websocket is on
    port: u16,
    /// The id of the webview that this websocket is connected to
    webview_id: u32,
}

impl WebviewWebsocketLocation {
    pub(crate) fn new(port: u16, webview_id: u32) -> Self {
        Self { port, webview_id }
    }

    /// Returns the websocket path for this webview
    pub(crate) fn path(&self) -> String {
        let Self { port, webview_id } = self;
        format!("ws://localhost:{port}/{webview_id}")
    }
}

/// This handles communication between the requests that the webview makes and the interpreter. The interpreter constantly makes long running requests to the webview to get any edits that should be made to the DOM almost like server side events.
/// It will hold onto the requests until the interpreter is ready to handle them and hold onto any pending edits until a new request is made.
#[derive(Clone)]
pub(crate) struct WryQueue {
    inner: Arc<RwLock<WryQueueInner>>,
}

pub(crate) struct WryQueueInner {
    location: WebviewWebsocketLocation,
    websocket: WryWebsocket,
    // If this webview is currently waiting for an edit to be flushed. We don't run the virtual dom while this is true to avoid running effects before the dom has been updated
    edits_in_progress: Option<oneshot::Receiver<()>>,
    mutation_state: MutationState,
}

impl WryQueue {
    pub fn with_mutation_state_mut<O: 'static>(
        &self,
        f: impl FnOnce(&mut MutationState) -> O,
    ) -> O {
        let mut inner = self.inner.write().unwrap();
        f(&mut inner.mutation_state)
    }

    /// Send a list of mutations to the webview
    pub(crate) fn send_edits(&self) {
        let mut myself = self.inner.write().unwrap();
        let serialized_edits = myself.mutation_state.export_memory();
        let edits = WebviewEditMessage {
            location: myself.location,
            edits: serialized_edits,
        };
        let receiver = myself.websocket.send_edits(edits);
        myself.edits_in_progress = Some(receiver);
    }

    /// Wait until all pending edits have been rendered in the webview
    pub fn poll_edits_flushed(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
        if let Some(receiver) = self.inner.write().unwrap().edits_in_progress.as_mut() {
            receiver.poll_unpin(cx).map(|_| ())
        } else {
            std::task::Poll::Ready(())
        }
    }

    /// Get the websocket path that the webview should connect to in order to receive edits
    pub fn edits_path(&self) -> String {
        self.inner.read().unwrap().location.path()
    }
}
