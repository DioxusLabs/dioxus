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

pub mod pool;
use futures_util::{SinkExt, StreamExt};
pub use pool::*;

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

    interpreter.replace("/*POST_EVENT_SERIALIZATION*/", serialize_file_uploads)
});

static MAIN_JS: &str = include_str!("./main.js");

/// This script that gets injected into your app connects this page to the websocket endpoint
///
/// Once the endpoint is connected, it will send the initial state of the app, and then start
/// processing user events and returning edits to the liveview instance
pub fn interpreter_glue(url: &str) -> String {
    let js = &*INTERPRETER_JS;
    format!(
        r#"
<script>
    var WS_ADDR = "{url}";
    {js}
    {MAIN_JS}
    main();
</script>
    "#
    )
}
