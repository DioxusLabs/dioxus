use std::prelude::rust_2024::Future;

use bytes::Bytes;

use crate::ServerFnError;

/// A WebSocket connection that can send and receive messages of type `In` and `Out`.
pub struct WebSocket<In = Bytes, Out = Bytes> {
    _in: std::marker::PhantomData<In>,
    _out: std::marker::PhantomData<Out>,
}

/// Create a new WebSocket connection that uses the provided function to handle incoming messages
impl WebSocket {
    pub fn new<F: Future<Output = ()>>(f: impl Fn((), ()) -> F) -> Self {
        Self {
            _in: std::marker::PhantomData,
            _out: std::marker::PhantomData,
        }
    }

    pub async fn send(&mut self, _msg: Bytes) -> Result<(), ServerFnError> {
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<Bytes, ServerFnError> {
        Ok(Bytes::new())
    }
}
