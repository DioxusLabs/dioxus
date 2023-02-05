//
// Axum-based example that demeonstrates how to implement session managment
// with Dioxus-LiveView.
//
// Clients get an session ID in the form of an encrypted and signed cookie
// assigned, and if the conection between client and server is lost, the client
// periodically tries to reconnect.
//
// State is persisted in a simple SQLite-based key/value store, saved on
// disconnects and restored after the client reconnects (using the assigned
// session ID).
//
// You can run the example via `cargo run --example sessions -F axum -- /tmp/data.db`.
//

use axum_extra::extract::cookie::PrivateCookieJar;
use dioxus::prelude::*;
use dioxus_liveview::{
    get_client_status, use_disconnect_client_actions, use_disconnect_handler, ClientStatus,
    DisconnectClientAction,
};
use std::sync::atomic;
use tokio::sync::OnceCell;

// Helper to disconnect clients:
static DISCONNECT: OnceCell<atomic::AtomicBool> = OnceCell::const_new();

// To store data, we'll use a simple key/value store based on SQLite/rusqlite:
static DB: OnceCell<KeyValueStore> = OnceCell::const_new();

/// Helper to get the database in an ergonomic way
fn db() -> &'static KeyValueStore {
    DB.get().expect("DB cell should be initiated")
}

#[tokio::main]
async fn main() {
    if std::env::var_os("RUST_LOG").is_none() {
        // Default to showing logs for `sessions.rs` and `dioxus_liveview`:
        // TODO: Set to `debug` when the example is finished:
        std::env::set_var("RUST_LOG", "warn,sessions=trace,dioxus_liveview=trace");
    }

    pretty_env_logger::init();

    // Getting the path where the database will be stored:
    let Some(db_path) = std::env::args().skip(1).next() else {
        eprintln!("Error: This example requires a path to a file to store data in");
        eprintln!("Usage: cargo run --example sessions -F axum -- $PATH_TO_DATA_FILE");
        std::process::exit(1);
    };

    let db = KeyValueStore::new(&db_path);

    // To encrypt and sign secure session cookies, we need to generate and
    // store a cryptographic master key. For simplicity, we'll store this key
    // in our key/value store. In production the key should be stored in a
    // secure way (as any other secretes).

    let key = if let Some(key) = db.get::<Vec<u8>>("key") {
        axum_extra::extract::cookie::Key::from(&key)
    } else {
        let key = axum_extra::extract::cookie::Key::generate();
        db.set("key", key.master());
        key
    };

    // Start the axum app:

    DB.set(db)
        .map_err(|_| ()) // `db` does not implement Debug
        .expect("Failed to initialize DB cell");

    DISCONNECT
        .set(atomic::AtomicBool::new(false))
        .expect("Failed to initialize DISCONNECT cell");

    let addr: std::net::SocketAddr = ([127, 0, 0, 1], 3030).into();
    let liveview_pool = dioxus_liveview::LiveViewPool::new();
    // TODO: Switch to `interpreter_glue` when example is finished:
    let liveview_glue_code =
        dioxus_liveview::InterpreterGlueBuilder::new(format!("ws://{addr}/ws"))
            .with_script_tag(true)
            // .reconnect(false)
            // .reconnection_delays(1000, &[])
            // .log(false)
            // .minify(true)
            .build();
    let state = AppState { key, liveview_pool };
    let html = axum::response::Html(format!(
        r#"
        <!DOCTYPE html>
        <html>
            <head><title>Dioxus LiveView with session managment</title></head>
            <body>
                <div id="main"></div>
                {liveview_glue_code}
            </body>
        </html>
        "#
    ));

    let router = axum::Router::new()
        .route("/", axum::routing::get(move || async move { html }))
        .route("/ws", axum::routing::get(websocket_endpoint))
        .with_state(state);

    log::info!("Listening on http://{}", addr);

    axum::Server::bind(&addr.to_string().parse().unwrap())
        .serve(router.into_make_service())
        .await
        .unwrap();
}

#[derive(Clone)]
struct AppState {
    key: axum_extra::extract::cookie::Key,
    liveview_pool: dioxus_liveview::LiveViewPool,
}

impl axum::extract::FromRef<AppState> for axum_extra::extract::cookie::Key {
    // Required so `SignedCookieJar` (see `websocket_endpoint`, below this) can
    // access the cryptographic master key. If you want to wrap `AppState` in
    // an `Arc`, see:
    // https://docs.rs/axum-extra/latest/axum_extra/extract/cookie/struct.PrivateCookieJar.html
    fn from_ref(state: &AppState) -> Self {
        state.key.clone()
    }
}

async fn websocket_endpoint(
    ws: axum::extract::ws::WebSocketUpgrade,
    cookies: PrivateCookieJar,
    state: axum::extract::State<AppState>,
) -> axum::response::Response {
    let (id, client_status, updated_cookies) = get_or_create_session_id(cookies);

    let response = ws.on_upgrade(move |socket| async move {
        let ws = dioxus_liveview::axum_socket(socket);

        // Adding a way to disconnect a client.
        // Normally, you'd just pass `ws` to
        // `view.launch_with_props_and_client_status`.
        let ws = enable_client_disconnections(ws);

        // Making the session ID available to the app, so state can be
        // associated with it:
        let props = AppProps { id };

        let result = state
            .liveview_pool
            .launch_with_props_and_client_status(ws, app, props, client_status)
            .await;

        match result {
            dioxus_liveview::DisconnectReason::Closure(frame) => {
                log::info!("WebSocket closed with close frame: {frame:?}")
                // Note: `Closure` represents a regular WebSocket closure, and
                // may or may not represent an error. See the following link
                // for more information about common close codes:
                // https://developer.mozilla.org/en-US/docs/Web/API/CloseEvent/code
            }
            dioxus_liveview::DisconnectReason::Error(e) => {
                log::error!("Error while handling client connection: {e:?}")
            }
        }
    });

    if let Some(updated_jar) = updated_cookies {
        // New session => set session cookie
        use axum::response::IntoResponse;
        return (updated_jar, response).into_response();
    }

    response
}

/// Get or create a session ID via an encrypted and signed cookie. Returns a
/// tuple with the client ID, the client status (which specifies whether the
/// client connects or reconnects), and – if a new session was started – an
/// updated `PrivateCookieJar`.
fn get_or_create_session_id(
    cookies: PrivateCookieJar,
) -> (uuid::Uuid, ClientStatus, Option<PrivateCookieJar>) {
    if let Some(cookie) = cookies.get("id") {
        let id = uuid::Uuid::try_parse(cookie.value()).expect("Failed to parse UUID");
        // This should never fail. Cookies are signed, so we are the only one
        // who can create them, so the UUID should always be valid.
        (id, ClientStatus::Reconnects, None)
    } else {
        let id = uuid::Uuid::new_v4();
        // Note: UUIDs are not suitable for access control (and therefore for
        // secure session IDs). Because our cookies are encrypted and signed,
        // using UUIDs is secure.
        let cookie = axum_extra::extract::cookie::Cookie::build("id", id.to_string())
            // TODO: Add note why this is important
            .same_site(axum_extra::extract::cookie::SameSite::Strict)
            // Mark the cookie as HTTP-only (i.e. JavaScript doesn't get access
            // to the cookie), to reduce the impact of potential security
            // vulnerbilities like Cross-Site-Scripting (XSS):
            .http_only(true);

        // The secure field, by which cookies are only sent during secure
        // connections (i.e. HTTPS), should be set to true in production.
        // Because this example doesn't implements a HTTPS server, we disable
        // the next line in DEBUG mode:
        #[cfg(debug_assertions)]
        let cookie = cookie.secure(true);

        // TODO: Is anything else important for secure session cookies? Go
        // through the OWASP Session Managment Cheat Sheet to make sure we
        // didn't forget anything:
        // https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html

        // TODO: Add note about secutity implications of unsecure sessions, and
        // recommend the Session Managment Cheat Sheet by OWASP.

        let updated_cookies = cookies.add(cookie.finish());

        (id, ClientStatus::Connects, Some(updated_cookies))
    }
}

// XXX: We probably should offer versions of `websocket_endpoint` and
// `get_or_create_session_id` as part of the adapter implementations. I
// probably will not add that now (this PR was already a ton of work), but
// maybe later

// Creating the actual Dioxus app:

#[derive(PartialEq, dioxus::core_macro::Props)]
struct AppProps {
    id: uuid::Uuid,
}

fn app(cx: Scope<AppProps>) -> Element {
    let mut num = use_state(cx, || 0);
    let status = get_client_status(cx);

    // After a connection is re-established, we check if we have any stored
    // state for the client:

    if status == ClientStatus::Reconnects {
        if let Some(n) = db().get(&cx.props.id.to_string()) {
            log::info!("Client reconnected. Restoring saved state...");
            num.set(n)
        } else {
            log::info!("Client reconnected. No saved state available.");
        }
    }

    // The client status is set via `launch_with_props_and_client_status` of
    // `LiveViewPool` (see `websocket_endpoint` above), and can be used to
    // execute actions, depending on the status of the client connection:

    match status {
        ClientStatus::Connects => {
            log::info!("Client connects for the first time.");
            // Intended as the initial status if a new session was started
            // (i.e. the client did not reconnect). This status can be used
            // for initial setup work. After the first render, the status will
            // always be `Initiated`
        }
        ClientStatus::Reconnects => {
            log::info!("Client reconnects.");
            // Similar to `Connects`, but should be provided if the client
            // reconnects with a session ID cookie. This status is useful for
            // restoring previously saved state (as we do above). Again, this
            // status will only ever be set during the first render of the app,
            // after a connection is established.
        }
        ClientStatus::Initiated => {
            log::info!("Client is interacting with the app.");
            // This is the state that will be used if no client
            // state is provided (i.e. if `LiveViewPool::launch` or
            // `LiveViewPool::launch_with_props` is used). After the first
            // render, the status will always be `Initiated`.
        }
    }

    // Next, we'll set up a callback that saves the current state of the
    // component when the client disconnects:

    use_disconnect_handler(cx, {
        let num = num.clone();
        let id = cx.props.id;
        move |reason: &dioxus_liveview::AxumDisconnectReason| {
            log::info!("Client disconnected. Reason: {reason:?}");
            log::info!("Saving the state");

            db().set(&id.to_string(), &*num.current());

            // Note: If the server crashes or exits, this callback will not be
            // called, so the state is lost. See `CrashResistantCounter` for a
            // crash resistant exampe

            // XXX: See my comment at the end of `pool.rs`, where I describe the
            // situation
        }
    });

    // After that, we'll define actions that will be executed client-side,
    // after the connection to the server is lost (these are sent to the client
    // when the connection is established, so can be executed after it was
    // lost):

    use_disconnect_client_actions(cx, || {
        use DisconnectClientAction::*;
        [
            // Disable `button` elements to prevent data loss while
            // disconnected:
            SetAttribute {
                // CSS selector that selects HTML elements:
                selector: "#main button".into(),

                // Attribute name:
                name: "disabled".into(),

                // Attribute value:
                value: "".into(),
            },
            // Display a warning at the top of the page:
            SetAttribute {
                selector: "#main #disconnectAlert".into(),
                name: "style".into(),
                value: "
                    padding: 20px;
                    background-color: red;
                    color: white;
                    margin-bottom: 15px;"
                    .into(),
            },
            // Call a global JavaScript function named `disconnected` (which we
            // define below, in a script tag):
            CallJsFn("disconnected".into()),
        ]
    });

    // XXX: I tried to figure out if it's somehow possible to reuse Dioxus
    // existing functionality for `DisconnectClientAction` (i.e. creating and
    // sending `Mutations` to the client after the connection is established,
    // and apply them after the connection is lost). This doesn't seem
    // feasible, however, because the state of the DOM most likely diverged
    // from the state for which `Mutations` was created. Let me know if this is
    // somehow possible. The simple tools we offer to handle disconnects seem
    // good enough, however. Even `SetAttribute` alone should be enough for
    // most scenerios, and via `CallJsFn`, any custom logic can be implemented
    // (also see my comment below `DisconnectClientAction` in `hooks.rs`).

    // Note: After the connection is re-established, the app will be completly
    // re-rendered, so changes that are made to the content of the app root via
    // `DisconnectClientAction`s don't have to be undone.

    // XXX: To not re-render the app after reconnecting, the VDOM or the
    // contained state would need to be (de-)serializable, for this to work in
    // a multi-server setup. This doesn't seems like the route we want to go,
    // however (too much complexity, for too little gain). But let me know, if
    // you have a different opinion.

    cx.render(rsx! {
        // An alert box that will be displayed after the server connection is lost:
        div {
            id: "disconnectAlert",
            display: "none",
            "Warning: The server connection was lost!"
        }

        p {
            "Counter: {num} "
            button { onclick: move |_| num += 1, "Increment" }
        }

        // A button to close the connection without stopping the server (uses
        // a hacky workaround, which triggers disconnects after every two
        // clicks):

        p {
            "Disconnect the client by clicking this button twice (clicking it once will not work): "

            button { onclick: move |_| {
                DISCONNECT.get().expect("Can't fail").store(true, atomic::Ordering::SeqCst);
            }, "Disconnect" }
        }

        CrashResistantCounter { id: cx.props.id}

        // Define a global JavaScript function that will be called when the
        // connection is lost. This also could be define elsewhere (e.g. in
        // the HTML base template). For complex functions, define them outside
        // `rsx!` or use `std::include_str`.

        script { "function disconnected() {{
            // Show an alert after other actions were applied to the DOM:
            requestIdleCallback(() => alert('Disconnected'));
        }}" }
    })
}

// Every component can have their own handlers and actions:

#[allow(non_snake_case)]
#[inline_props]
fn CrashResistantCounter(cx: Scope, id: uuid::Uuid) -> Element {
    let mut num = use_state(cx, || 0);
    let key = move || format!("CrashResistantCounter_{id}");

    if get_client_status(cx) == ClientStatus::Reconnects {
        if let Some(n) = db().get(&key()) {
            num.set(n)
        }
    }

    // XXX: For some reason, the counter below isn't incremented if the parent
    // increment button was clicked. After reconnecting, the button works
    // again. The click event is sent to the server (and logged to the terminal
    // at the `trace` level), but they are not applied (i.e. the saved and
    // displayed state doesn't change). Is there anything wrong with this code?
    // This might just be my lacking knowledge regarding Dioxus.

    cx.render(rsx! {
        // Important state that needs to survive crashes and server restarts
        // (e.g. form data) needs to be saved after every update (or in the
        // granularity that results in acceptable potential data loss):

        p {
            "CrashResistantCounter: {num} "
            button {
                onclick: move |_| {
                    num += 1;
                    db().set(&key(), &*num.current());
                },
                "Increment"
            }
        }

        p {
            "Note: The state of CrashResistantCounter survives across server "
            "restarts (inscluding crashes). Give it a try, by incrementing the "
            "counter and then restarting the server."
        }
    })
}

// Some helpers for the example:

struct KeyValueStore {
    db: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
}

impl KeyValueStore {
    fn new(path: &str) -> Self {
        let db = rusqlite::Connection::open(path).expect("Failed to open SQLite database");
        let sql = "CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value BLOB NOT NULL)";
        db.execute(sql, ())
            .expect("Failed to create key/value table");
        let db = std::sync::Arc::new(std::sync::Mutex::new(db));
        Self { db }
    }

    fn set<T>(&self, key: &str, value: &T)
    where
        T: ?Sized + serde::ser::Serialize,
    {
        let bytes = serde_json::to_vec(value).expect("Failed to serialize value");
        let sql = "INSERT OR REPLACE INTO kv (key, value) VALUES (?1, ?2)";
        self.db
            .lock()
            .expect("Poisoned Mutex")
            .execute(sql, (key, &bytes))
            .expect("Failed to store key/value pair");
    }

    fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        use rusqlite::OptionalExtension;
        self.db
            .lock()
            .expect("Poisoned Mutex")
            .query_row("SELECT value FROM kv WHERE key = $1", (key,), |row| {
                let v: Vec<u8> = row.get(0).expect("Failed to get value");
                let v = serde_json::from_slice(&v).expect("Failed to deserialize value");
                Ok(v)
            })
            .optional()
            .expect("Failed to get value")
    }
}

/// Turns the next message receive into a `Close` message, if `DISCONNECT` is
/// set to `true`. Two clicks on the 'Disconnect' button are required, because
/// the first click sets `DISCONNECT` to `true`, and the second click triggers
/// the next message, which is then converted into a `Close` message.
fn enable_client_disconnections<
    SendErr: Send + std::fmt::Debug + 'static,
    RecvErr: Send + std::fmt::Debug + 'static,
>(
    ws: impl dioxus_liveview::LiveViewSocket<SendErr, RecvErr>,
) -> impl dioxus_liveview::LiveViewSocket<SendErr, RecvErr> {
    ws.map(move |msg| {
        let disconnect = DISCONNECT.get().expect("Can't fail");
        if disconnect.load(atomic::Ordering::SeqCst) {
            disconnect.store(false, atomic::Ordering::SeqCst); // Only disconnect once
            return Ok(dioxus_liveview::WebSocketMsg::Close(None));
        }
        msg
    })
}
