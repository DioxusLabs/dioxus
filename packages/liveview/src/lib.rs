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

static MINIFIED: &str = include_str!("./minified.js");

/// This script that gets injected into your app connects this page to the websocket endpoint
///
/// Once the endpoint is connected, it will send the initial state of the app, and then start
/// processing user events and returning edits to the liveview instance
pub fn interpreter_glue(url: &str) -> String {
    format!(
        r#"
<script>
    var WS_ADDR = "{url}";
    {MINIFIED}
</script>
    "#
    )
}
