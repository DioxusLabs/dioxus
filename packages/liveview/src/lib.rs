pub mod adapters;
pub mod hooks;
pub mod pool;

use std::borrow::Cow;

use tokio::sync::OnceCell;

pub use adapters::*;
pub use hooks::*;
pub use pool::*;

use dioxus_interpreter_js::INTERPRETER_JS;
static MAIN_JS: &str = include_str!("./main.js");
static MAIN_JS_MINIFIED: OnceCell<String> = OnceCell::const_new();

/// TODO: Docs. Tip: Use the adapter-specific type alias (e.g.
/// `AxumLiveViewError`) for a type without generics
#[derive(Debug, thiserror::Error)]
pub enum LiveViewError<
    SendErr: Send + std::fmt::Debug + 'static,
    RecvErr: Send + std::fmt::Debug + 'static,
> {
    /// Failed to receive a message
    #[error("Failed to receive a message. Context: {0:?}")]
    ReceivingMsgFailed(RecvErr),

    /// The WebSocket stream closed unexpectedly (i.e. without receiving a `Close` message)
    #[error("The WebSocket stream closed unexpectedly (i.e. without receiving a `Close` message). Context: {context:?}")]
    StreamClosedUnexpectedly { context: Cow<'static, str> },

    /// Received unexpected message
    #[error("Received unexpected message. Context: {context:?}, message: {msg:?}")]
    UnexpectedMsg {
        msg: WebSocketMsg,
        context: Cow<'static, str>,
    },

    /// Failed to send a message
    #[error("Failed to send a message")]
    SendingMsgFailed(SendErr),

    /// The event loop handling the WebSocket or the application panicked
    #[error("The event loop handling the WebSocket or the application panicked")]
    Panicked(Box<dyn std::any::Any + Send + 'static>),
}

// XXX: Having the `SendErr`/`RecvErr` generics makes the internal API quite a
// bit more verbose and complex (the "Custom Adapter" API also, slightly), but
// the regular public API isn't much worse (e.g., users just use `axum_socket`
// and `AxumLiveViewError`, and the handler passed to `use_disconnect_handler`
// needs its argument annotated via `AxumDisconnectReason`). I believe,
// exposing the full underlying errors (instead of only a `stringified` version
// of them) is the right thing to do. It was quite a PITA to add this in
// retrospect (it was one of the last things I did), and it would be great if
// you would double-check that everything makes sense.

/// TODO: Docs. represents the reason why WebSocket session was disconnected.
/// `Closure` => received `WebSocketMsg::Close` (which might or might not
/// represent an error), `Error` => something failed. Tip: Use a adapter-
/// specific type alias (e.g. `AxumDisconnectReason`) for a type without
/// generics
// XXX: Currently, we do not treat any disconnect caused by a
// `WebSocketMsg::Close` message as an error. Mostly because I'm not sure which
// WebSocket close codes to classify as failures. Should we try to do this
// classification? It seems reasonable to let the user handle this scenario.
#[derive(Debug)]
pub enum DisconnectReason<
    SendErr: Send + std::fmt::Debug + 'static,
    RecvErr: Send + std::fmt::Debug + 'static,
> {
    Closure(Option<crate::CloseFrame>),
    Error(crate::LiveViewError<SendErr, RecvErr>),
}

/// Returns the source code of a JavaScript script element to embed into a
/// page to connect it to the LiveView server via the provided WebSocket
/// endpoint.
///
/// Once the client is connected to the server, the server will send the
/// initial application state to the client, start processing user events
/// received from it and send edits back.
///
/// Also see [InterpreterGlueBuilder] for a way to configure aspects of the
/// generated source code.
pub fn interpreter_glue<'a>(websocket_endpoint: impl Into<Cow<'a, str>>) -> String {
    InterpreterGlueBuilder::new(websocket_endpoint)
        .with_script_tag(true)
        .build()
    // XXX: I believe this should not default to wrapping the code in a script
    // tag (this can be added easily where the code is used). I've just added
    // it for backwards compatibility. I normally probably wouldn't even add
    // the `with_script_tag` method to the builder itself.
}

/// A builder to generate the JavaScript source code to embed into a page to
/// connect it to the LiveView server via the provided WebSocket endpoint. Also
/// see [interpreter_glue].
pub struct InterpreterGlueBuilder<'a> {
    websocket_endpoint: Cow<'a, str>,
    log: bool,
    reconnect: bool,
    reconnection_delays: Vec<(u64, u64)>,
    with_script_tag: bool,
    minify: bool,
}

impl<'a> InterpreterGlueBuilder<'a> {
    /// Create a new builder with the provided WebSocket endpoint.
    pub fn new(websocket_endpoint: impl Into<Cow<'a, str>>) -> Self {
        let this = Self {
            websocket_endpoint: websocket_endpoint.into(),
            log: cfg!(debug_assertions),
            reconnect: true,
            reconnection_delays: Vec::new(),
            with_script_tag: false,
            minify: !cfg!(debug_assertions),
        };

        Self::reconnection_delays(this, 500, &[(20 * 1000, 1000), (5 * 60 * 1000, 3000)])

        // ^ Start with a reconnection delay of 500 millisecond. After 20
        // seconds, switch to a delay of one second. After 5 minutes, switch to
        // three seconds.
    }

    /// Whether to log information to the browser console. Defaults to `true`
    /// during DEBUG mode, otherwise to `false`.
    pub fn log(mut self, value: bool) -> Self {
        self.log = value;
        self
    }

    /// Whether to automatically reconnect, when the connection to the server is
    /// lost. Defaults to `true`.
    pub fn reconnect(mut self, value: bool) -> Self {
        self.reconnect = value;
        self
    }

    /// Set the initial reconnection delay to `delay` milliseconds.
    /// `delay_changes` can optionally contain tuples of the format `(duration,
    /// new_delay)`, meaning, that the previous delay will be changed to
    /// `new_delay` after `duration`. All `u64` values represent milliseconds.
    ///
    /// Example 1: Start with a reconnection delay of 500 milliseconds. After
    /// 20 seconds, switch to a delay of one second. After 5 minutes, switch to
    /// three seconds (which are the default settings):
    ///
    /// ```rust
    /// builder.reconnection_delays(500, &[(20 * 1000, 1000), (5 * 60 * 1000, 3000)]);
    /// ```
    ///
    /// Example 2: Use a constant delay of one second:
    ///
    /// ```rust
    /// builder.reconnection_delays(1000, &[]);
    /// ```
    pub fn reconnection_delays(mut self, mut delay: u64, delay_changes: &[(u64, u64)]) -> Self {
        // Transform the arguments of this function to `Vec<($duration, $delay)>`.
        // $delay will be used between attempts to reconnect until $duration ms
        // are over, after which the next $delay will be used. The last entry has
        // `0` as $duration, because the JavaScript code will ignore the last
        // $duration (i.e. keep the last $delay).

        let number_of_changes = delay_changes.len();

        let delays = if number_of_changes == 0 {
            vec![(0, delay)]
        } else {
            let mut delays = Vec::with_capacity(number_of_changes + 1);
            let last_index = number_of_changes - 1;
            let changes = delay_changes.iter().enumerate();

            for (index, (duration_of_previous_delay, next_delay)) in changes {
                delays.push((*duration_of_previous_delay, delay));
                delay = *next_delay;
                if index == last_index {
                    delays.push((0, delay));
                }
            }

            delays
        };

        self.reconnection_delays = delays;

        self
    }

    /// Whether to wrap the generated JavaScript code with a script tag.
    /// Defaults to `false`.
    pub fn with_script_tag(mut self, value: bool) -> Self {
        self.with_script_tag = value;
        self
    }

    /// Whether to remove lines that can be removed without changing the
    /// functionality of the code. If set to `true`, removes lines that only
    /// consist of whitespace, comments and log statements. If `log` is set
    /// to `true`, log statements will not be removed. Defaults to `false` in
    /// DEBUG mode, otherwise to `true`.
    ///
    /// Note: For full minification, an external tool has to be used. This
    /// option only handles a few low hanging fruits.
    pub fn minify(mut self, value: bool) -> Self {
        self.minify = value;
        self
    }

    pub fn build(self) -> String {
        let Self {
            websocket_endpoint,
            log,
            reconnect,
            reconnection_delays,
            with_script_tag,
            minify,
        } = self;

        // TODO: Cache the last value of this via `OnceCell`?
        let reconnection_delays = serde_json::to_string(&reconnection_delays)
            .expect("Serializing `Vec<(u64, u64)>` should never fail");

        let (script_open, script_close) = if with_script_tag {
            ("<script>", "</script>")
        } else {
            ("", "")
        };

        // We only "minify" ´main.js` at the moment (`INTERPRETER_JS` doesn't
        // have many lines to remove anyway):
        let main_js = if minify {
            if let Some(minified) = MAIN_JS_MINIFIED.get() {
                minified
            } else {
                let lines = MAIN_JS.lines().filter(|line| {
                    let line = line.trim_start();
                    !(line.is_empty()
                        || line.starts_with("//")
                        || (!log && line.starts_with("log(")))
                });
                let mut main_js = String::with_capacity(MAIN_JS.len());
                for line in lines {
                    main_js.push_str(line);
                    main_js.push('\n');
                }
                main_js.shrink_to_fit();
                MAIN_JS_MINIFIED.set(main_js).expect("Can't fail");
                MAIN_JS_MINIFIED.get().expect("Can't fail")
            }
        } else {
            MAIN_JS
        };

        format!(
            r#"{script_open}
var DIOXUS_WS_ADDR = "{websocket_endpoint}";
var DIOXUS_RECONNECT = {reconnect};
var DIOXUS_RECONNECT_DELAYS = {reconnection_delays};
var DIOXUS_LOG = {log};
{INTERPRETER_JS};
{main_js};
main();
{script_close}"#
        )
    }
}
