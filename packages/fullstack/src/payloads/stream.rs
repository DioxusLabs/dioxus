#![allow(clippy::type_complexity)]

use crate::{
    CborEncoding, ClientRequest, ClientResponse, Encoding, FromResponse, IntoRequest, JsonEncoding,
    ServerFnError,
};
use axum::extract::{FromRequest, Request};
use axum_core::response::IntoResponse;
use bytes::{Buf as _, Bytes};
use dioxus_fullstack_core::{HttpError, RequestError};
use futures::{Stream, StreamExt};
#[cfg(feature = "server")]
use futures_channel::mpsc::UnboundedSender;
use headers::{ContentType, Header};
use send_wrapper::SendWrapper;
use serde::{de::DeserializeOwned, Serialize};
use std::{future::Future, marker::PhantomData, pin::Pin};

/// A stream of text data.
///
/// # Chunking
///
/// Note that strings sent by the server might not arrive in the same chunking as they were sent.
///
/// This is because the underlying transport layer (HTTP/2 or HTTP/3) may choose to split or combine
/// chunks for efficiency.
///
/// If you need to preserve individual string boundaries, consider using `ChunkedTextStream` or another
/// encoding that preserves chunk boundaries.
pub type TextStream = Streaming<String>;

/// A stream of binary data.
///
/// # Chunking
///
/// Note that bytes sent by the server might not arrive in the same chunking as they were sent.
/// This is because the underlying transport layer (HTTP/2 or HTTP/3) may choose to split or combine
/// chunks for efficiency.
///
/// If you need to preserve individual byte boundaries, consider using `ChunkedByteStream` or another
/// encoding that preserves chunk boundaries.
pub type ByteStream = Streaming<Bytes>;

/// A stream of JSON-encoded data.
///
/// # Chunking
///
/// Normally, it's not possible to stream JSON over HTTP because browsers are free to re-chunk
/// data as they see fit. However, this implementation manually frames each JSON as if it were an unmasked
/// websocket message.
///
/// If you need to send a stream of JSON data without framing, consider using TextStream instead and
/// manually handling JSON buffering.
pub type JsonStream<T> = Streaming<T, JsonEncoding>;

/// A stream of Cbor-encoded data.
///
/// # Chunking
///
/// Normally, it's not possible to stream JSON over HTTP because browsers are free to re-chunk
/// data as they see fit. However, this implementation manually frames each item as if it were an unmasked
/// websocket message.
pub type CborStream<T> = Streaming<T, CborEncoding>;

/// A stream of manually chunked binary data.
///
/// This encoding preserves chunk boundaries by framing each chunk with its length, using Websocket
/// Framing.
pub type ChunkedByteStream = Streaming<Bytes, CborEncoding>;

/// A stream of manually chunked text data.
///
/// This encoding preserves chunk boundaries by framing each chunk with its length, using Websocket
/// Framing.
pub type ChunkedTextStream = Streaming<String, CborEncoding>;

/// A streaming payload.
///
/// ## Frames and Chunking
///
/// The streaming payload sends and receives data in discrete chunks or "frames". The size is converted
/// to hex and sent before each chunk, followed by a CRLF, the chunk data, and another CRLF.
///
/// This mimics actual HTTP chunked transfer encoding, but allows us to define our own framing
/// protocol on top of it.
///
/// Arbitrary bytes can be encoded between these frames, but the frames do come with some overhead.
///
/// ## Browser Support for Streaming Input
///
/// Browser fetch requests do not currently support full request duplexing, which
/// means that that they do not begin handling responses until the full request has been sent.
///
/// This means that if you use a streaming input encoding, the input stream needs to
/// end before the output will begin.
///
/// Streaming requests are only allowed over HTTP2 or HTTP3.
///
/// Also note that not all browsers support streaming bodies to servers.
pub struct Streaming<T = String, E = ()> {
    stream: Pin<Box<dyn Stream<Item = Result<T, StreamingError>> + Send>>,
    encoding: PhantomData<E>,
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Hash)]
pub enum StreamingError {
    /// The streaming request was interrupted and could not be completed.
    #[error("The streaming request was interrupted")]
    Interrupted,

    /// The stream failed to decode a chunk - possibly due to invalid data or version mismatch.
    #[error("The stream failed to decode a chunk")]
    Decoding,

    /// The stream failed to connect or encountered an error.
    #[error("The streaming request failed")]
    Failed,
}

impl<T: 'static + Send, E> Streaming<T, E> {
    /// Creates a new stream from the given stream.
    pub fn new(value: impl Stream<Item = T> + Send + 'static) -> Self {
        // Box and pin the incoming stream and store as a trait object
        Self {
            stream: Box::pin(value.map(|item| Ok(item)))
                as Pin<Box<dyn Stream<Item = Result<T, StreamingError>> + Send>>,
            encoding: PhantomData,
        }
    }

    /// Spawns a new task that produces items for the stream.
    ///
    /// The callback is provided an `UnboundedSender` that can be used to send items to the stream.
    #[cfg(feature = "server")]
    pub fn spawn<F>(callback: impl FnOnce(UnboundedSender<T>) -> F + Send + 'static) -> Self
    where
        F: Future<Output = ()> + 'static,
        T: Send,
    {
        let (tx, rx) = futures_channel::mpsc::unbounded();

        crate::spawn_platform(move || callback(tx));

        Self::new(rx)
    }

    /// Returns the next item in the stream, or `None` if the stream has ended.
    pub async fn next(&mut self) -> Option<Result<T, StreamingError>> {
        self.stream.as_mut().next().await
    }

    /// Consumes the wrapper, returning the inner stream.
    pub fn into_inner(self) -> impl Stream<Item = Result<T, StreamingError>> + Send {
        self.stream
    }

    /// Creates a streaming payload from an existing stream of bytes.
    ///
    /// This uses the internal framing mechanism to decode the stream into items of type `T`.
    fn from_bytes(stream: impl Stream<Item = Result<T, StreamingError>> + Send + 'static) -> Self {
        Self {
            stream: Box::pin(stream),
            encoding: PhantomData,
        }
    }
}

impl<S, U> From<S> for TextStream
where
    S: Stream<Item = U> + Send + 'static,
    U: Into<String>,
{
    fn from(value: S) -> Self {
        Self::new(value.map(|data| data.into()))
    }
}

impl<S, E> From<S> for ByteStream
where
    S: Stream<Item = Result<Bytes, E>> + Send + 'static,
{
    fn from(value: S) -> Self {
        Self {
            stream: Box::pin(value.map(|data| data.map_err(|_| StreamingError::Failed))),
            encoding: PhantomData,
        }
    }
}

impl<T, S, U, E> From<S> for Streaming<T, E>
where
    S: Stream<Item = U> + Send + 'static,
    U: Into<T>,
    T: 'static + Send,
    E: Encoding,
{
    fn from(value: S) -> Self {
        Self::from_bytes(value.map(|data| Ok(data.into())))
    }
}

impl IntoResponse for Streaming<String> {
    fn into_response(self) -> axum_core::response::Response {
        axum::response::Response::builder()
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(axum::body::Body::from_stream(self.stream))
            .unwrap()
    }
}

impl IntoResponse for Streaming<Bytes> {
    fn into_response(self) -> axum_core::response::Response {
        axum::response::Response::builder()
            .header("Content-Type", "application/octet-stream")
            .body(axum::body::Body::from_stream(self.stream))
            .unwrap()
    }
}

impl<T: DeserializeOwned + Serialize + 'static, E: Encoding> IntoResponse for Streaming<T, E> {
    fn into_response(self) -> axum_core::response::Response {
        let res = self.stream.map(|r| match r {
            Ok(res) => match encode_stream_frame::<T, E>(res) {
                Some(bytes) => Ok(bytes),
                None => Err(StreamingError::Failed),
            },
            Err(_err) => Err(StreamingError::Failed),
        });

        axum::response::Response::builder()
            .header("Content-Type", E::stream_content_type())
            .body(axum::body::Body::from_stream(res))
            .unwrap()
    }
}

impl FromResponse for Streaming<String> {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        SendWrapper::new(async move {
            let client_stream = Box::pin(res.bytes_stream().map(|byte| match byte {
                Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                    Ok(string) => Ok(string),
                    Err(_) => Err(StreamingError::Decoding),
                },
                Err(_) => Err(StreamingError::Failed),
            }));

            Ok(Self {
                stream: client_stream,
                encoding: PhantomData,
            })
        })
    }
}

impl FromResponse for Streaming<Bytes> {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move {
            let client_stream = Box::pin(SendWrapper::new(res.bytes_stream().map(
                |byte| match byte {
                    Ok(bytes) => Ok(bytes),
                    Err(_) => Err(StreamingError::Failed),
                },
            )));

            Ok(Self {
                stream: client_stream,
                encoding: PhantomData,
            })
        }
    }
}

impl<T: DeserializeOwned + Serialize + 'static + Send, E: Encoding> FromResponse
    for Streaming<T, E>
{
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        SendWrapper::new(async move {
            Ok(Self {
                stream: byte_stream_to_client_stream::<E, _, _, _>(res.bytes_stream()),
                encoding: PhantomData,
            })
        })
    }
}

impl<S> FromRequest<S> for Streaming<String> {
    type Rejection = ServerFnError;

    fn from_request(
        req: Request,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let (parts, body) = req.into_parts();
            let content_type = parts
                .headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if !content_type.starts_with("text/plain") {
                HttpError::bad_request("Invalid content type")?;
            }

            let stream = body.into_data_stream();

            Ok(Self {
                stream: Box::pin(stream.map(|byte| match byte {
                    Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                        Ok(string) => Ok(string),
                        Err(_) => Err(StreamingError::Decoding),
                    },
                    Err(_) => Err(StreamingError::Failed),
                })),
                encoding: PhantomData,
            })
        }
    }
}

impl<S> FromRequest<S> for ByteStream {
    type Rejection = ServerFnError;

    fn from_request(
        req: Request,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let (parts, body) = req.into_parts();
            let content_type = parts
                .headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if !content_type.starts_with("application/octet-stream") {
                HttpError::bad_request("Invalid content type")?;
            }

            let stream = body.into_data_stream();

            Ok(Self {
                stream: Box::pin(stream.map(|byte| match byte {
                    Ok(bytes) => Ok(bytes),
                    Err(_) => Err(StreamingError::Failed),
                })),
                encoding: PhantomData,
            })
        }
    }
}

impl<T: DeserializeOwned + Serialize + 'static + Send, E: Encoding, S> FromRequest<S>
    for Streaming<T, E>
{
    type Rejection = ServerFnError;

    fn from_request(
        req: Request,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let (parts, body) = req.into_parts();
            let content_type = parts
                .headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if !content_type.starts_with(E::stream_content_type()) {
                HttpError::bad_request("Invalid content type")?;
            }

            let stream = body.into_data_stream();

            Ok(Self {
                stream: byte_stream_to_client_stream::<E, _, _, _>(stream),
                encoding: PhantomData,
            })
        }
    }
}

impl IntoRequest for Streaming<String> {
    fn into_request(
        self,
        builder: ClientRequest,
    ) -> impl Future<Output = Result<ClientResponse, RequestError>> + 'static {
        async move {
            builder
                .header("Content-Type", "text/plain; charset=utf-8")?
                .send_body_stream(self.stream.map(|e| e.map(Bytes::from)))
                .await
        }
    }
}

impl IntoRequest for ByteStream {
    fn into_request(
        self,
        builder: ClientRequest,
    ) -> impl Future<Output = Result<ClientResponse, RequestError>> + 'static {
        async move {
            builder
                .header(ContentType::name(), "application/octet-stream")?
                .send_body_stream(self.stream)
                .await
        }
    }
}

impl<T: DeserializeOwned + Serialize + 'static + Send, E: Encoding> IntoRequest
    for Streaming<T, E>
{
    fn into_request(
        self,
        builder: ClientRequest,
    ) -> impl Future<Output = Result<ClientResponse, RequestError>> + 'static {
        async move {
            builder
                .header("Content-Type", E::stream_content_type())?
                .send_body_stream(self.stream.map(|r| {
                    r.and_then(|item| {
                        encode_stream_frame::<T, E>(item).ok_or(StreamingError::Failed)
                    })
                }))
                .await
        }
    }
}

impl<T> std::fmt::Debug for Streaming<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Streaming").finish()
    }
}

impl<T, E: Encoding> std::fmt::Debug for Streaming<T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Streaming")
            .field("encoding", &std::any::type_name::<E>())
            .finish()
    }
}

/// This function encodes a single frame of a streaming payload using the specified encoding.
///
/// The resulting `Bytes` object is encoded as a websocket frame, so you can send it over a streaming
/// HTTP response or even a websocket connection.
///
/// Note that the packet is not masked, as it is assumed to be sent over a trusted connection.
pub fn encode_stream_frame<T: Serialize, E: Encoding>(data: T) -> Option<Bytes> {
    // We use full advantage of `BytesMut` here, writing a maximally full frame and then shrinking it
    // down to size at the end.
    //
    // Also note we don't do any masking over this data since it's not going over an untrusted
    // network like a websocket would.
    //
    // We allocate 10 extra bytes to account for framing overhead, which we'll shrink after
    let mut bytes = vec![0u8; 10];

    E::encode(data, &mut bytes)?;

    let len = (bytes.len() - 10) as u64;
    let opcode = 0x82; // FIN + binary opcode

    // Write the header directly into the allocated space.
    let offset = if len <= 125 {
        bytes[8] = opcode;
        bytes[9] = len as u8;
        8
    } else if len <= u16::MAX as u64 {
        bytes[6] = opcode;
        bytes[7] = 126;
        let len_bytes = (len as u16).to_be_bytes();
        bytes[8] = len_bytes[0];
        bytes[9] = len_bytes[1];
        6
    } else {
        bytes[0] = opcode;
        bytes[1] = 127;
        bytes[2..10].copy_from_slice(&len.to_be_bytes());
        0
    };

    // Shrink down to the actual used size - is zero copy!
    Some(Bytes::from(bytes).slice(offset..))
}

fn byte_stream_to_client_stream<E, T, S, E1>(
    stream: S,
) -> Pin<Box<dyn Stream<Item = Result<T, StreamingError>> + Send>>
where
    S: Stream<Item = Result<Bytes, E1>> + 'static + Send,
    E: Encoding,
    T: DeserializeOwned + 'static,
{
    Box::pin(stream.flat_map(|bytes| {
        enum DecodeIteratorState {
            Empty,
            Failed,
            Checked(Bytes),
            UnChecked(Bytes),
        }

        let mut state = match bytes {
            Ok(bytes) => DecodeIteratorState::UnChecked(bytes),
            Err(_) => DecodeIteratorState::Failed,
        };

        futures::stream::iter(std::iter::from_fn(move || {
            match std::mem::replace(&mut state, DecodeIteratorState::Empty) {
                DecodeIteratorState::Empty => None,
                DecodeIteratorState::Failed => Some(Err(StreamingError::Failed)),
                DecodeIteratorState::Checked(mut bytes) => {
                    let r = decode_stream_frame_multi::<T, E>(&mut bytes);
                    if r.is_some() {
                        state = DecodeIteratorState::Checked(bytes)
                    }
                    r
                }
                DecodeIteratorState::UnChecked(mut bytes) => {
                    let r = decode_stream_frame_multi::<T, E>(&mut bytes);
                    if r.is_some() {
                        state = DecodeIteratorState::Checked(bytes);
                        r
                    } else {
                        Some(Err(StreamingError::Decoding))
                    }
                }
            }
        }))
    }))
}

/// Decode a websocket-framed streaming payload produced by [`encode_stream_frame`].
///
/// This function returns `None` if the frame is invalid or cannot be decoded.
///
/// It cannot handle masked frames, as those are not produced by our encoding function.
pub fn decode_stream_frame<T, E>(mut frame: Bytes) -> Option<T>
where
    E: Encoding,
    T: DeserializeOwned,
{
    decode_stream_frame_multi::<T, E>(&mut frame).and_then(|r| r.ok())
}

/// Decode one value and advance the bytes pointer
///
/// If the frame is empty return None.
///
/// Otherwise, if the initial opcode is not the one expected for binary stream
/// or the frame is not large enough return error StreamingError::Decoding
fn decode_stream_frame_multi<T, E>(frame: &mut Bytes) -> Option<Result<T, StreamingError>>
where
    E: Encoding,
    T: DeserializeOwned,
{
    let (offset, payload_len) = match offset_payload_len(frame)? {
        Ok(r) => r,
        Err(e) => return Some(Err(e)),
    };

    let r = E::decode(frame.slice(offset..offset + payload_len));
    frame.advance(offset + payload_len);
    r.map(|r| Ok(r))
}

/// Compute (offset,len) for decoding data
fn offset_payload_len(frame: &Bytes) -> Option<Result<(usize, usize), StreamingError>> {
    let data = frame.as_ref();

    if data.is_empty() {
        return None;
    }

    if data.len() < 2 {
        return Some(Err(StreamingError::Decoding));
    }

    let first = data[0];
    let second = data[1];

    // Require FIN with binary opcode and no RSV bits
    let fin = first & 0x80 != 0;
    let opcode = first & 0x0F;
    let rsv = first & 0x70;
    if !fin || opcode != 0x02 || rsv != 0 {
        return Some(Err(StreamingError::Decoding));
    }

    // Mask bit must be zero for our framing
    if second & 0x80 != 0 {
        return Some(Err(StreamingError::Decoding));
    }

    let mut offset = 2usize;
    let mut payload_len = (second & 0x7F) as usize;

    if payload_len == 126 {
        if data.len() < offset + 2 {
            return Some(Err(StreamingError::Decoding));
        }

        payload_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
    } else if payload_len == 127 {
        if data.len() < offset + 8 {
            return Some(Err(StreamingError::Decoding));
        }

        let mut len_bytes = [0u8; 8];
        len_bytes.copy_from_slice(&data[offset..offset + 8]);
        let len_u64 = u64::from_be_bytes(len_bytes);

        if len_u64 > usize::MAX as u64 {
            return Some(Err(StreamingError::Decoding));
        }

        payload_len = len_u64 as usize;
        offset += 8;
    }

    if data.len() < offset + payload_len {
        return Some(Err(StreamingError::Decoding));
    }
    Some(Ok((offset, payload_len)))
}
