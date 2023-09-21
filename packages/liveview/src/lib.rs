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
/// processing user events and returning edits to the liveview instance
pub fn interpreter_glue(url: &str) -> String {
    let js = &*INTERPRETER_JS;
    let common = &*COMMON_JS;
    format!(
        r#"
<script>
    var WS_ADDR = "{url}";
    {js}
    {common}
    {MAIN_JS}
    main();
</script>
    "#
    )
}

/// This function behaves similarly to [`interpreter_glue`], but interprets the provided URI as a
/// relative path, automatically prefixing the protocol and host endpoint.
///
/// For instance, if you call `interpreter_glue("ws://localhost:1234/ws/liveview")` and the root of
/// the liveview is being served at `localhost:1234`, you can replace it with
/// `interpreter_glue_relative_uri("/ws/liveview")`.
pub fn interpreter_glue_relative_uri(mut relative_uri: &str) -> String {
    if relative_uri.starts_with("/") {
        // NOTE: The `relative_uri` is automatically prefixed with `ws[s]://<host>/` during
        // client-side JavaScript execution, eliminating the need for a leading `/` as in `api/foo`.
        // However, since it's customary to use a `/` at the API level (e.g., `fetch("/api/foo")`),
        // we permit its inclusion in the `relative_uri` parameter for consistency.
        relative_uri = &relative_uri[1..];
    }

    let js = &*INTERPRETER_JS;
    let common = &*COMMON_JS;
    format!(
        r#"
<script>
function __dioxusGetWsUrl() {{
  let loc = window.location; 
  let new_uri = "";
  if (loc.protocol === "https:") {{
      new_uri = "wss:";
  }} else {{
      new_uri = "ws:";
  }}
  new_uri += "//" + loc.host;
  new_uri += "/{relative_uri}";
  return new_uri;
}}

var WS_ADDR = __dioxusGetWsUrl();
{js}
{common}
{MAIN_JS}
main();
</script>
    "#
    )
}
