#[cfg(feature = "axum")]
pub mod axum_adapter;

#[cfg(feature = "axum")]
pub use axum_adapter::*;

#[cfg(feature = "warp")]
pub mod warp_adapter;

#[cfg(feature = "warp")]
pub use warp_adapter::*;

#[cfg(feature = "salvo")]
pub mod salvo_adapter;

#[cfg(feature = "salvo")]
pub use salvo_adapter::*;

use crate::LiveViewError;
use futures_util::{SinkExt, StreamExt};

// TODO: Add proper docs
/// WebSocket message type that WebSocket adapters need to produce/consume.
///
/// No `Ping` and `Pong` because we don't use that, because browser don't seem
/// to always support it. Adapters need to filter out `Ping` and `Pong` message
/// that some clients might send (most Rust server/client libraries handle these
/// message transparently, so they should not end um in the `Stream`/`Sink`)
#[derive(Debug, PartialEq)]
pub enum WebSocketMsg {
    /// A text message
    Text(String),

    /// A binary message. Note: Binary message are currently not used/supported
    /// by this LiveView server (but might be later).
    // XXX: Should we even have this now? Will `Text` still be available when
    // binary messages will be used later? I've added it so adding it later
    // doesn't break the API. But if we anyway would completely switch to binary
    // messages later (and stop using `Text`), that would anyway break the API
    // (or we'll have a useless `Text` variant in the public API).
    Binary(Vec<u8>),

    /// Close message
    Close(Option<CloseFrame>),
}

#[derive(Debug, PartialEq)]
pub struct CloseFrame {
    pub code: u16,
    pub reason: std::borrow::Cow<'static, str>,
}

pub trait WebsocketTx<
    SendErr: Send + std::fmt::Debug + 'static,
    RecvErr: Send + std::fmt::Debug + 'static,
>: SinkExt<WebSocketMsg, Error = LiveViewError<SendErr, RecvErr>>
{
}

impl<T, SendErr, RecvErr> WebsocketTx<SendErr, RecvErr> for T
where
    SendErr: Send + std::fmt::Debug + 'static,
    RecvErr: Send + std::fmt::Debug + 'static,
    T: SinkExt<WebSocketMsg, Error = LiveViewError<SendErr, RecvErr>>,
{
}

pub trait WebsocketRx<
    SendErr: Send + std::fmt::Debug + 'static,
    RecvErr: Send + std::fmt::Debug + 'static,
>: StreamExt<Item = Result<WebSocketMsg, LiveViewError<SendErr, RecvErr>>>
{
}

impl<T, SendErr, RecvErr> WebsocketRx<SendErr, RecvErr> for T
where
    SendErr: Send + std::fmt::Debug + 'static,
    RecvErr: Send + std::fmt::Debug + 'static,
    T: StreamExt<Item = Result<WebSocketMsg, LiveViewError<SendErr, RecvErr>>>,
{
}

// TODO: Update docs:
/// A LiveViewSocket is a Sink and Stream of Strings that Dioxus uses to communicate with the client
///
/// Most websockets from most HTTP frameworks can be converted into a LiveViewSocket using the appropriate adapter.
///
/// You can also convert your own socket into a LiveViewSocket by implementing this trait. This trait is an auto trait,
/// meaning that as long as your type implements Stream and Sink, you can use it as a LiveViewSocket.
///
/// For example, the axum implementation is a really small transform:
///
/// ```rust, ignore
/// pub fn axum_socket(ws: WebSocket) -> impl LiveViewSocket {
///     ws.map(transform_rx)
///         .with(transform_tx)
///         .sink_map_err(|_| LiveViewError::SendingFailed)
/// }
///
/// fn transform_rx(message: Result<Message, axum::Error>) -> Result<String, LiveViewError> {
///     message
///         .map_err(|_| LiveViewError::SendingFailed)?
///         .into_text()
///         .map_err(|_| LiveViewError::SendingFailed)
/// }
///
/// async fn transform_tx(message: String) -> Result<Message, axum::Error> {
///     Ok(Message::Text(message))
/// }
/// ```
pub trait LiveViewSocket<
    SendErr: Send + std::fmt::Debug + 'static,
    RecvErr: Send + std::fmt::Debug + 'static,
>:
    SinkExt<WebSocketMsg, Error = LiveViewError<SendErr, RecvErr>>
    + StreamExt<Item = Result<WebSocketMsg, LiveViewError<SendErr, RecvErr>>>
    + Send
    + 'static
{
}

impl<S, SendErr, RecvErr> LiveViewSocket<SendErr, RecvErr> for S
where
    SendErr: Send + std::fmt::Debug + 'static,
    RecvErr: Send + std::fmt::Debug + 'static,
    S: SinkExt<WebSocketMsg, Error = LiveViewError<SendErr, RecvErr>>
        + StreamExt<Item = Result<WebSocketMsg, LiveViewError<SendErr, RecvErr>>>
        + Send
        + 'static,
{
}
