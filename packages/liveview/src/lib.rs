#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

mod adapters;
#[allow(unused_imports)]
pub use adapters::*;

mod element;
pub mod pool;
mod query;
mod upload;
use dioxus_interpreter_js::NATIVE_JS;
use futures_util::{SinkExt, StreamExt};
pub use pool::*;
mod config;
mod document;
mod events;
mod file_data;
mod file_transfer;
mod history;
pub use config::*;

/// The default cap on declared bytes in unread, incoming, and retained files per connection.
pub const DEFAULT_UPLOAD_STORAGE_LIMIT: u64 = 1024 * 1024 * 1024;

/// The default cap on unread, incoming, and retained files per LiveView connection.
pub const DEFAULT_UPLOAD_FILE_LIMIT: usize = 1024;

/// How long a file transfer requested by a read may wait for its HTTP request by default.
pub const DEFAULT_UPLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5 * 60);

#[cfg(feature = "axum")]
pub mod launch;

pub trait WebsocketTx: SinkExt<String, Error = LiveViewError> {}
impl<T> WebsocketTx for T where T: SinkExt<String, Error = LiveViewError> {}

pub trait WebsocketRx: StreamExt<Item = Result<String, LiveViewError>> {}
impl<T> WebsocketRx for T where T: StreamExt<Item = Result<String, LiveViewError>> {}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LiveViewError {
    #[error("Sending to client error")]
    SendingFailed,
}

fn handle_edits_code() -> String {
    use dioxus_interpreter_js::unified_bindings::SLEDGEHAMMER_JS;

    let mut interpreter = format!(
        r#"
    // Bring the sledgehammer code
    {SLEDGEHAMMER_JS}

    // And then extend it with our native bindings
    {NATIVE_JS}
    "#
    )
    .replace("export", "");
    while let Some(import_start) = interpreter.find("import") {
        let import_end = interpreter[import_start..]
            .find([';', '\n'])
            .map(|i| i + import_start)
            .unwrap_or_else(|| interpreter.len());
        interpreter.replace_range(import_start..import_end, "");
    }
    let main_js = include_str!("./main.js");
    let js = format!("{interpreter}\n{main_js}");
    js
}

/// This script that gets injected into your app connects this page to the websocket endpoint
///
/// Once the endpoint is connected, it will send the initial state of the app, and then start
/// processing user events and returning edits to the liveview instance.
///
/// You can pass a relative path prefixed with "/", or enter a full URL including the protocol
/// (`ws:` or `wss:`) as an argument.
///
/// If you enter a relative path, the web client automatically prefixes the host address in
/// `window.location` when creating a web socket to LiveView.
///
/// ```rust
/// use dioxus_liveview::interpreter_glue;
///
/// // Creates websocket connection to same host as current page
/// interpreter_glue("/api/liveview");
///
/// // Creates websocket connection to specified url
/// interpreter_glue("ws://localhost:8080/api/liveview");
/// ```
pub fn interpreter_glue(url_or_path: &str) -> String {
    // If the url starts with a `/`, generate glue which reuses current host
    let get_ws_url = if url_or_path.starts_with('/') {
        r#"
  let loc = window.location;
  let new_url = "";
  if (loc.protocol === "https:") {{
      new_url = "wss:";
  }} else {{
      new_url = "ws:";
  }}
  new_url += "//" + loc.host + path;
  return new_url;
      "#
    } else {
        "return path;"
    };

    let handle_edits = handle_edits_code();

    format!(
        r#"
<script>
    function __dioxusGetWsUrl(path) {{
      {get_ws_url}
    }}

    var WS_ADDR = __dioxusGetWsUrl("{url_or_path}");
    {handle_edits}
</script>
    "#
    )
}
