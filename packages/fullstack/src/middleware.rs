use crate::error::ServerFnError;
use bytes::Bytes;
use std::{future::Future, pin::Pin};

/// An abstraction over a middleware layer, which can be used to add additional
/// middleware layer to a [`Service`].
pub trait Layer<Req, Res>: Send + Sync + 'static {
    /// Adds this layer to the inner service.
    fn layer(&self, inner: BoxedService<Req, Res>) -> BoxedService<Req, Res>;
}

/// A type-erased service, which takes an HTTP request and returns a response.
pub struct BoxedService<Req, Res> {
    /// A function that converts a [`ServerFnError`] into a string.
    pub ser: fn(ServerFnError) -> Bytes,

    /// The inner service.
    pub service: Box<dyn Service<Req, Res> + Send>,
}

impl<Req, Res> BoxedService<Req, Res> {
    /// Constructs a type-erased service from this service.
    pub fn new(
        ser: fn(ServerFnError) -> Bytes,
        service: impl Service<Req, Res> + Send + 'static,
    ) -> Self {
        Self {
            ser,
            service: Box::new(service),
        }
    }

    /// Converts a request into a response by running the inner service.
    pub fn run(&mut self, req: Req) -> Pin<Box<dyn Future<Output = Res> + Send>> {
        self.service.run(req, self.ser)
    }
}

/// A service converts an HTTP request into a response.
pub trait Service<Request, Response> {
    /// Converts a request into a response.
    fn run(
        &mut self,
        req: Request,
        ser: fn(ServerFnError) -> Bytes,
    ) -> Pin<Box<dyn Future<Output = Response> + Send>>;
}
