pub mod adapters {
    #[cfg(feature = "warp")]
    pub mod warp_adapter;

    #[cfg(feature = "axum")]
    pub mod axum_adapter;

    #[cfg(feature = "salvo")]
    pub mod salvo_adapter;
}

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

use dioxus_interpreter_js::INTERPRETER_JS;
static MAIN_JS: &str = include_str!("./main.js");

pub fn interpreter_glue(url: &str) -> String {
    format!(
        r#"
<script>
    var WS_ADDR = "{url}";
    {INTERPRETER_JS}
    {MAIN_JS}
    main();
</script>
    "#
    )
}
