use crate::{
    request::Req,
    response::{Res, TryRes},
};
use std::future::Future;

/// A server defines a pair of request/response types and the logic to spawn
/// an async task.
///
/// This trait is implemented for any server backend for server functions including
/// `axum` and `actix-web`. It should almost never be necessary to implement it
/// yourself, unless you’re trying to use an alternative HTTP server.
pub trait Server<Error, InputStreamError = Error, OutputStreamError = Error> {
    /// The type of the HTTP request when received by the server function on the server side.
    type Request: Req<
            Error,
            InputStreamError,
            OutputStreamError,
            WebsocketResponse = Self::Response,
        > + Send
        + 'static;

    /// The type of the HTTP response returned by the server function on the server side.
    type Response: Res + TryRes<Error> + Send + 'static;

    /// Spawn an async task on the server.
    fn spawn(
        future: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), Error>;
}
