//! The internal edit queue facilitating native <-> webview communication.
//!
//! Originally, we used long-polling on the wry custom protocol to send edits to the webview.
//! Due to bugs in wry on android, we switched to a websocket connection that the webview connects to.
//! We use the sledgehammer crate to build batches of edits and send them through the websocket to
//! the webview.
//!
//! Using a websocket lets us send binary data to the webview quite efficiently and does encounter
//! many of the issues with regular request/response protocols. Note that the websocket max frame
//! size is quite large (9.22 exabytes), so we can have very large batches without issue.
//!
//! Using websockets does mean we need to handle security and content security policies ourselves.
//! The code here generates a random key that the webview must use to connect to the websocket.
//! We use the initialization script API to setup the websocket connection without leaking the key
//! to the webview itself in case there's untrusted content in the webview.

use dioxus_interpreter_js::MutationState;
use futures_channel::mpsc::UnboundedSender;
use futures_channel::oneshot;
use futures_util::{FutureExt, StreamExt};
use pollster::FutureExt as _;
use rand::{RngCore, SeedableRng};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::net::TcpListener;
use std::rc::Rc;
use std::sync::atomic::AtomicU32;
use std::{
    net::IpAddr,
    sync::{Arc, RwLock},
};

/// This handles communication between the requests that the webview makes and the interpreter.
#[derive(Clone)]
pub(crate) struct WryQueue {
    inner: Rc<RefCell<WryQueueInner>>,
}

impl WryQueue {
    pub(crate) fn with_mutation_state_mut<O: 'static>(
        &self,
        callback: impl FnOnce(&mut MutationState) -> O,
    ) -> O {
        let mut inner = self.inner.borrow_mut();
        callback(&mut inner.mutation_state)
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
    pub(crate) fn poll_edits_flushed(
        &self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<()> {
        if let Some(receiver) = self.inner.borrow_mut().edits_in_progress.as_mut() {
            receiver.poll_unpin(cx).map(|_| ())
        } else {
            std::task::Poll::Ready(())
        }
    }

    /// Get the websocket path that the webview should connect to in order to receive edits
    pub(crate) fn edits_path(&self) -> String {
        let WebviewWebsocketLocation {
            port,
            webview_id,
            key,
        } = &self.inner.borrow().location;
        let key_hex = encode_key_string(key);
        format!("ws://127.0.0.1:{port}/{webview_id}/{key_hex}")
    }
}

pub(crate) struct WryQueueInner {
    location: WebviewWebsocketLocation,
    websocket: EditWebsocket,
    // If this webview is currently waiting for an edit to be flushed. We don't run the virtual dom while this is true to avoid running effects before the dom has been updated
    edits_in_progress: Option<oneshot::Receiver<()>>,
    mutation_state: MutationState,
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

/// The websocket listener that the webview will connect to in order to receive edits and send requests. There
/// is only one websocket listener per application even if there are multiple windows so we don't use all the
/// open ports.
#[derive(Clone)]
pub(crate) struct EditWebsocket {
    /// A key that every websocket connection that originates from this application will use to identify itself.
    /// We use this to make sure no external applications can connect to our websocket and receive UI updates.
    key: [u8; KEY_SIZE],
    port: u16,
    max_webview_id: Arc<AtomicU32>,
    connections: Arc<RwLock<HashMap<u32, WebviewConnectionState>>>,
}

impl EditWebsocket {
    pub(crate) fn start() -> Self {
        let expected_key = create_secure_key();
        let connections = Arc::new(RwLock::new(HashMap::new()));
        let connections_ = connections.clone();
        let server = TcpListener::bind((IpAddr::from([127, 0, 0, 1]), 0))
            .expect("Failed to bind local TCP listener for edit socket");
        let port = server.local_addr().unwrap().port();

        std::thread::spawn(move || Self::accept_loop(server, port, expected_key, connections_));

        Self {
            connections,
            port,
            max_webview_id: Default::default(),
            key: expected_key,
        }
    }

    /// Accepts incoming websocket connections and handles them in a loop.
    ///
    /// New sockets are accepted and then put in to a new thread to handle the connection.
    /// This is implemented using traditional sync code to allow us to be independent of the async runtime.
    fn accept_loop(
        server: TcpListener,
        port: u16,
        expected_key: [u8; KEY_SIZE],
        connections: Arc<RwLock<HashMap<u32, WebviewConnectionState>>>,
    ) {
        use tungstenite::handshake::server::{Request, Response};

        let hex_encoded_key = encode_key_string(&expected_key);
        while let Ok((stream, _)) = server.accept() {
            let mut location = None;

            let on_request = |req: &Request, res| {
                // Try to parse the webview id and key from the path
                let path = req.uri().path();

                // The path should have two parts `/webview_id/key`
                let mut segments = path.trim_matches('/').split('/');
                let webview_id = segments
                    .next()
                    .and_then(|s| s.parse::<u32>().ok())
                    .ok_or_else(|| {
                        Response::builder()
                            .status(400)
                            .body(Some("Bad Request: Invalid webview ID".to_string()))
                            .unwrap()
                    })?;
                let key = segments.next().ok_or_else(|| {
                    Response::builder()
                        .status(400)
                        .body(Some("Bad Request: Missing key".to_string()))
                        .unwrap()
                })?;

                // Make sure the key matches the expected key.
                // VERY IMPORTANT: We cannot use normal string comparison here because it reveals information
                // about the key based on timing information. Instead we use a constant time comparison method.
                let key_matches: bool =
                    subtle::ConstantTimeEq::ct_eq(hex_encoded_key.as_ref(), key.as_bytes()).into();
                if !key_matches {
                    return Err(Response::builder()
                        .status(403)
                        .body(Some("Forbidden: Invalid key".to_string()))
                        .unwrap());
                }

                location = Some(WebviewWebsocketLocation {
                    port,
                    webview_id,
                    key: expected_key,
                });

                Ok(res)
            };

            // Accept the websocket connection while reading the path and setting the location
            let mut websocket = match tungstenite::accept_hdr(stream, on_request) {
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
            let (edits_outgoing, mut edits_incoming_rx) =
                futures_channel::mpsc::unbounded::<MsgPair>();

            // Spawn a task to handle the websocket connection
            std::thread::spawn(move || {
                // Wait until there are edits ready to send
                while let Some(msg) = edits_incoming_rx.next().block_on() {
                    // Send the edits to the webview
                    if let Err(e) = websocket.send(tungstenite::Message::Binary(msg.edits.into())) {
                        tracing::error!("Error sending edits to webview: {}", e);
                        break;
                    }

                    // Wait for the webview to apply the edits
                    while let Ok(msg) = websocket.read() {
                        match msg {
                            // We expect the webview to send a binary message when it has applied the edits
                            // This is a signal that we can continue processing
                            tungstenite::Message::Binary(_) => break,
                            tungstenite::Message::Close(_) => return,
                            _ => {}
                        }
                    }

                    // Notify that the edits have been applied
                    if msg.response.send(()).is_err() {
                        tracing::error!("Error sending edits applied notification");
                    }
                }
            });

            let mut connections = connections.write().unwrap();
            match connections.remove(&location.webview_id) {
                // If there are pending edits, send them to the new connection
                Some(WebviewConnectionState::Pending { mut pending }) => {
                    while let Some(pair) = pending.pop_front() {
                        _ = edits_outgoing.unbounded_send(pair);
                    }
                }

                // If the webview was already connected, never send edits from the old connection to
                // the new connection. This should never happen
                Some(WebviewConnectionState::Connected { .. }) => {
                    tracing::error!(
                        "Webview {} was already connected. Rejecting new connection.",
                        location.webview_id
                    );
                    continue;
                }

                None => {}
            }

            connections.insert(
                location.webview_id,
                WebviewConnectionState::Connected { edits_outgoing },
            );
        }
    }

    pub(crate) fn create_queue(&self) -> WryQueue {
        WryQueue {
            inner: Rc::new(RefCell::new(WryQueueInner {
                location: WebviewWebsocketLocation {
                    webview_id: self
                        .max_webview_id
                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
                    port: self.port,
                    key: self.key,
                },
                websocket: self.clone(),
                edits_in_progress: None,
                mutation_state: MutationState::default(),
            })),
        }
    }

    fn send_edits(&mut self, webview: u32, edits: Vec<u8>) -> oneshot::Receiver<()> {
        let mut connections_mut = self.connections.write().unwrap();
        let connection =
            connections_mut
                .entry(webview)
                .or_insert_with(|| WebviewConnectionState::Pending {
                    pending: VecDeque::new(),
                });

        match connection {
            WebviewConnectionState::Pending { pending: queue } => {
                let (response_sender, response_receiver) = oneshot::channel();
                queue.push_back(MsgPair {
                    edits,
                    response: response_sender,
                });
                response_receiver
            }
            WebviewConnectionState::Connected { edits_outgoing } => {
                let (response_sender, response_receiver) = oneshot::channel();
                _ = edits_outgoing.unbounded_send(MsgPair {
                    edits,
                    response: response_sender,
                });
                response_receiver
            }
        }
    }
}

/// The state of a webview websocket connection. This may be pending while the webview is booting.
/// If it is, we queue up edits until the webview is ready to receive them.
enum WebviewConnectionState {
    Pending {
        pending: VecDeque<MsgPair>,
    },
    Connected {
        edits_outgoing: UnboundedSender<MsgPair>,
    },
}

struct MsgPair {
    edits: Vec<u8>,
    response: oneshot::Sender<()>,
}

const KEY_SIZE: usize = 256;
type EncodedKey = [u8; KEY_SIZE];

/// Base64 encode the key to a string to be used in the websocket URL.
fn encode_key_string(key: &EncodedKey) -> String {
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE, key)
}

/// Create a secure key for the websocket connection.
/// Returns the key as a byte array and a hex-encoded string representation of the key.
fn create_secure_key() -> EncodedKey {
    // Helper function to assert that the RNG is a CryptoRng - make sure we use a secure RNG
    fn assert_crypto_random<R: rand::CryptoRng>(val: R) -> R {
        val
    }

    let mut secure_rng = assert_crypto_random(rand::rngs::StdRng::from_entropy());
    let mut expected_key: EncodedKey = [0u8; KEY_SIZE];
    secure_rng.fill_bytes(&mut expected_key);
    expected_key
}

#[test]
fn test_key_encoding_length() {
    let mut rand = rand::rngs::StdRng::from_entropy();
    for _ in 0..100 {
        let mut key: EncodedKey = [0u8; KEY_SIZE];
        rand.fill_bytes(&mut key);
        let encoded = encode_key_string(&key);
        // The encoded key length should be the same regardless of the value of the key
        assert_eq!(encoded.len(), 344);
    }
}
