use dioxus_interpreter_js::MutationState;
use futures_channel::mpsc::UnboundedSender;
use futures_channel::oneshot;
use futures_util::{FutureExt, StreamExt};
use pollster::FutureExt as _;
use rand::{RngCore, SeedableRng};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::atomic::AtomicU32;
use std::thread::spawn;
use std::{
    net::IpAddr,
    sync::{Arc, RwLock},
};
use tungstenite::handshake::server::{Request, Response};
use tungstenite::{accept_hdr, WebSocket};

/// Bind a listener to any port that is available on the given address.
fn get_available_port(address: IpAddr) -> Option<u16> {
    // Otherwise, try to bind to any port and return the first one we can
    TcpListener::bind((address, 0))
        .and_then(|listener| listener.local_addr().map(|f| f.port()))
        .ok()
}

const KEY_SIZE: usize = 256;

fn encode_key_string(key: &[u8; KEY_SIZE]) -> String {
    // base64 encode the key to a string
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE, key)
}

#[test]
fn test_key_encoding_length() {
    let mut rand = rand::rngs::StdRng::from_entropy();
    for _ in 0..100 {
        let mut key = [0u8; KEY_SIZE];
        rand.fill_bytes(&mut key);
        let encoded = encode_key_string(&key);
        // The encoded key length should be the same regardless of the value of the key
        assert_eq!(encoded.len(), 344);
    }
}

/// The websocket listener that the webview will connect to in order to receive edits and send requests. There
/// is only one websocket listener per application even if there are multiple windows so we don't use all the
/// open ports.
#[derive(Clone)]
pub(crate) struct EditWebsocket {
    port: u16,
    max_webview_id: Arc<AtomicU32>,
    connections: Arc<RwLock<HashMap<u32, WebviewConnectionState>>>,
    /// A key that every websocket connection that originates from this application will use to identify itself.
    /// We use this to make sure no external applications can connect to our websocket and receive UI updates.
    pub(crate) key: [u8; KEY_SIZE],
}

impl EditWebsocket {
    pub(crate) fn new() -> Self {
        let connections = Arc::new(RwLock::new(HashMap::new()));

        let ip = IpAddr::from([127, 0, 0, 1]);
        let port = get_available_port(ip).unwrap_or(9001);

        fn assert_crypto_random<R: rand::CryptoRng>(val: R) -> R {
            val
        }

        let mut secure_rng = assert_crypto_random(rand::rngs::StdRng::from_entropy());
        let mut expected_key = [0u8; KEY_SIZE];
        secure_rng.fill_bytes(&mut expected_key);
        let hex_encoded_key = encode_key_string(&expected_key);

        let server = TcpListener::bind((ip, port)).unwrap();
        spawn({
            let connections = connections.clone();
            let hex_encoded_key = hex_encoded_key.clone();
            move || {
                while let Ok((stream, _)) = server.accept() {
                    let mut location = None;
                    // Accept the websocket connection while reading the path
                    let websocket = accept_hdr(stream, |req: &Request, res| {
                        // Try to parse the webview id and key from the path
                        let path = req.uri().path();
                        // The path should have two parts `/webview_id/key`
                        let mut segments = path.trim_matches('/').split('/');
                        let webview_id = segments.next().and_then(|s| s.parse::<u32>().ok());
                        let Some(webview_id) = webview_id else {
                            return Err(Response::builder()
                                .status(400)
                                .body(Some("Bad Request: Invalid webview ID".to_string()))
                                .unwrap());
                        };
                        location = Some(WebviewWebsocketLocation::new(
                            port,
                            webview_id,
                            expected_key,
                        ));
                        let Some(key) = segments.next() else {
                            return Err(Response::builder()
                                .status(400)
                                .body(Some("Bad Request: Missing key".to_string()))
                                .unwrap());
                        };

                        // Make sure the key matches the expected key.
                        // VERY IMPORTANT: We cannot use normal string comparison here because it reveals information
                        // about the key based on timing information. Instead we use a constant time comparison method.
                        let key_matches: bool =
                            subtle::ConstantTimeEq::ct_eq(hex_encoded_key.as_ref(), key.as_bytes())
                                .into();

                        if !key_matches {
                            return Err(Response::builder()
                                .status(403)
                                .body(Some("Forbidden: Invalid key".to_string()))
                                .unwrap());
                        }

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
                    let existing_entry = connections.remove(&location.webview_id);
                    if let Some(existing_entry) = existing_entry {
                        if let WebviewConnectionState::Pending(mut pending) = existing_entry {
                            while let Some((edit, response_sender)) = pending.pop_front() {
                                connection.send_edits_with_response(edit, response_sender);
                            }
                        } else {
                            // If the webview was already connected, never send edits from the old connection to
                            // the new connection. This should never happen
                            tracing::error!(
                                "Webview {} was already connected. Rejecting new connection.",
                                location.webview_id
                            );
                            continue;
                        }
                    }
                    connections.insert(
                        location.webview_id,
                        WebviewConnectionState::from(connection),
                    );
                }
            }
        });

        Self {
            connections,
            port,
            max_webview_id: Default::default(),
            key: expected_key,
        }
    }

    pub(crate) fn create_queue(&self) -> WryQueue {
        let webview_id = self
            .max_webview_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let location = WebviewWebsocketLocation::new(self.port, webview_id, self.key);
        let websocket = self.clone();

        WryQueue {
            inner: Rc::new(RefCell::new(WryQueueInner {
                location,
                websocket,
                edits_in_progress: None,
                mutation_state: MutationState::default(),
            })),
        }
    }

    fn send_edits(&mut self, webview: u32, edits: Vec<u8>) -> oneshot::Receiver<()> {
        let mut connections_mut = self.connections.write().unwrap();
        let connection = connections_mut.entry(webview).or_default();
        connection.send_edits(edits)
    }
}

/// The state of a webview websocket connection. This may be pending while the webview is booting.
/// If it is, we queue up edits until the webview is ready to receive them.
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

/// An active connection to a webview that is used to send edits and receive responses.
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
                    // Send the edits to the webview
                    if let Err(e) = socket.send(tungstenite::Message::Binary(edits.into())) {
                        tracing::error!("Error sending edits to webview: {}", e);
                        break;
                    }
                    // Wait for the webview to apply the edits
                    while let Ok(msg) = socket.read() {
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

/// The location of a webview websocket connection. This is used to identify the webview and the port it is connected to.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct WebviewWebsocketLocation {
    /// The port the websocket is on
    port: u16,
    /// The id of the webview that this websocket is connected to
    webview_id: u32,
    /// The key that the webview will use to connect to the websocket
    key: [u8; KEY_SIZE],
}

impl WebviewWebsocketLocation {
    pub(crate) fn new(port: u16, webview_id: u32, key: [u8; KEY_SIZE]) -> Self {
        Self {
            port,
            webview_id,
            key,
        }
    }

    /// Returns the websocket path for this webview
    pub(crate) fn path(&self) -> String {
        let Self {
            port,
            webview_id,
            key,
        } = self;
        let key_hex = encode_key_string(key);
        format!("ws://127.0.0.1:{port}/{webview_id}/{key_hex}")
    }
}

/// This handles communication between the requests that the webview makes and the interpreter. The interpreter constantly makes long running requests to the webview to get any edits that should be made to the DOM almost like server side events.
/// It will hold onto the requests until the interpreter is ready to handle them and hold onto any pending edits until a new request is made.
#[derive(Clone)]
pub(crate) struct WryQueue {
    inner: Rc<RefCell<WryQueueInner>>,
}

pub(crate) struct WryQueueInner {
    location: WebviewWebsocketLocation,
    websocket: EditWebsocket,
    // If this webview is currently waiting for an edit to be flushed. We don't run the virtual dom while this is true to avoid running effects before the dom has been updated
    edits_in_progress: Option<oneshot::Receiver<()>>,
    mutation_state: MutationState,
}

impl WryQueue {
    pub fn with_mutation_state_mut<O: 'static>(
        &self,
        f: impl FnOnce(&mut MutationState) -> O,
    ) -> O {
        let mut inner = self.inner.borrow_mut();
        f(&mut inner.mutation_state)
    }

    /// Send a list of mutations to the webview
    pub(crate) fn send_edits(&self) {
        let mut myself = self.inner.borrow_mut();
        let webview_id = myself.location.webview_id;
        let serialized_edits = myself.mutation_state.export_memory();
        let receiver = myself.websocket.send_edits(webview_id, serialized_edits);
        myself.edits_in_progress = Some(receiver);
    }

    /// Wait until all pending edits have been rendered in the webview
    pub fn poll_edits_flushed(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
        if let Some(receiver) = self.inner.borrow_mut().edits_in_progress.as_mut() {
            receiver.poll_unpin(cx).map(|_| ())
        } else {
            std::task::Poll::Ready(())
        }
    }

    /// Get the websocket path that the webview should connect to in order to receive edits
    pub fn edits_path(&self) -> String {
        self.inner.borrow().location.path()
    }
}
