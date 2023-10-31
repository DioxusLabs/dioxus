#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub mod adapters {
    #[cfg(feature = "warp")]
    pub mod warp_adapter;
    #[cfg(feature = "warp")]
    pub use warp_adapter::*;

    #[cfg(feature = "axum")]
    pub mod axum_adapter;
    #[cfg(feature = "axum")]
    pub use axum_adapter::*;

    #[cfg(feature = "salvo")]
    pub mod salvo_adapter;

    #[cfg(feature = "salvo")]
    pub use salvo_adapter::*;
}

pub use adapters::*;

mod element;
pub mod pool;
mod query;
use futures_util::{SinkExt, StreamExt};
pub use pool::*;
mod eval;

pub trait WebsocketTx: SinkExt<String, Error = LiveViewError> {}
impl<T> WebsocketTx for T where T: SinkExt<String, Error = LiveViewError> {}

pub trait WebsocketRx: StreamExt<Item = Result<String, LiveViewError>> {}
impl<T> WebsocketRx for T where T: StreamExt<Item = Result<String, LiveViewError>> {}

#[derive(Debug, thiserror::Error)]
pub enum LiveViewError {
    #[error("warp error")]
    SendingFailed,
}

use once_cell::sync::Lazy;

static INTERPRETER_JS: Lazy<String> = Lazy::new(|| {
    let interpreter = dioxus_interpreter_js::INTERPRETER_JS;
    let serialize_file_uploads = r#"if (
      target.tagName === "INPUT" &&
      (event.type === "change" || event.type === "input")
    ) {
      const type = target.getAttribute("type");
      if (type === "file") {
        async function read_files() {
          const files = target.files;
          const file_contents = {};

          for (let i = 0; i < files.length; i++) {
            const file = files[i];

            file_contents[file.name] = Array.from(
              new Uint8Array(await file.arrayBuffer())
            );
          }
          let file_engine = {
            files: file_contents,
          };
          contents.files = file_engine;

          if (realId === null) {
            return;
          }
          const message = serializeIpcMessage("user_event", {
            name: name,
            element: parseInt(realId),
            data: contents,
            bubbles,
          });
          window.ipc.postMessage(message);
        }
        read_files();
        return;
      }
    }"#;

    let interpreter = interpreter.replace("/*POST_EVENT_SERIALIZATION*/", serialize_file_uploads);
    interpreter.replace("import { setAttributeInner } from \"./common.js\";", "")
});

static COMMON_JS: Lazy<String> = Lazy::new(|| {
    let common = dioxus_interpreter_js::COMMON_JS;
    common.replace("export", "")
});

static MAIN_JS: &str = include_str!("./main.js");

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
/// ```
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

    let js = &*INTERPRETER_JS;
    let common = &*COMMON_JS;
    format!(
        r#"
<script>
    function __dioxusGetWsUrl(path) {{
      {get_ws_url}
    }}
    
    var WS_ADDR = __dioxusGetWsUrl("{url_or_path}");
    {js}
    {common}
    {MAIN_JS}
    main();
</script>
    "#
    )
}
