use crate::{ClientResponse, FromResponse};
use dioxus_fullstack_core::ServerFnError;
use std::prelude::rust_2024::Future;

pub struct RangedBytes {
    #[cfg(feature = "server")]
    response: Option<axum::response::Response>,
}

impl FromResponse for RangedBytes {
    fn from_response(
        res: ClientResponse,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        async move {
            Ok(Self {
                #[cfg(feature = "server")]
                response: None,
            })
        }
    }
}

/// The main responder type. Implements [`IntoResponse`].
pub struct Ranged<B> {
    #[cfg(feature = "server")]
    range: Option<axum_extra::headers::Range>,
    body: B,
}

#[cfg(feature = "server")]
pub use server_impl::*;

#[cfg(feature = "server")]
mod server_impl {
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Response};
    use axum_extra::headers::{AcceptRanges, ContentLength, ContentRange, Range};
    pub use axum_extra::TypedHeader;
    use bytes::Bytes;
    use bytes::BytesMut;
    use futures::Stream;
    use http_body::{Body, Frame, SizeHint};
    use pin_project::pin_project;
    use std::io;
    use std::mem;
    use std::ops::Bound;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::{AsyncRead, AsyncSeek};
    use tokio::io::{AsyncSeekExt, ReadBuf};

    pub type RangeHeader = Option<TypedHeader<Range>>;

    use super::*;

    impl RangedBytes {
        /// Create a ranged response from a file path and an optional [`Range`] header.
        pub async fn from_file(
            path: impl AsRef<std::path::Path>,
            range: RangeHeader,
        ) -> Result<Self, io::Error> {
            let file = tokio::fs::File::open(path).await?;
            let known = KnownSize::file(file).await?;
            let ranged = Ranged::new(range.map(|range| range.0), known);
            let response = ranged.try_respond().unwrap().into_response();
            Ok(RangedBytes {
                response: Some(response),
            })
        }

        /// Create a ranged response from an in-memory buffer and an optional [`Range`] header.
        pub fn from_bytes(bytes: impl Into<Bytes>, range: RangeHeader) -> Self {
            let known = KnownSize::bytes(bytes);
            let ranged = Ranged::new(range.map(|range| range.0), known);
            let response = ranged.try_respond().unwrap().into_response();
            RangedBytes {
                response: Some(response),
            }
        }

        /// Create a ranged response from any type implementing [`AsyncRead`] and [`AsyncSeek`].
        pub fn from_async_read(
            body: impl AsyncRead + AsyncSeek + Send + 'static,
            byte_size: u64,
            range: RangeHeader,
        ) -> Self {
            let known = KnownSize::sized(body, byte_size);
            let ranged = Ranged::new(range.map(|range| range.0), known);
            let response = ranged.try_respond().unwrap().into_response();
            RangedBytes {
                response: Some(response),
            }
        }
    }

    impl IntoResponse for RangedBytes {
        fn into_response(self) -> axum::response::Response {
            self.response.unwrap()
        }
    }

    impl Ranged<Box<dyn RangeBody + Send + 'static>> {
        /// Construct a ranged response over any type implementing [`RangeBody`]
        /// and an optional [`Range`] header.
        pub fn boxed(range: Option<Range>, body: impl RangeBody + Send + 'static) -> Self {
            Ranged {
                range,
                body: Box::new(body),
            }
        }
    }

    impl<B: RangeBody + Send + 'static> Ranged<B> {
        /// Construct a ranged response over any type implementing [`RangeBody`]
        /// and an optional [`Range`] header.
        pub fn new(range: Option<Range>, body: B) -> Self {
            Ranged { range, body }
        }

        /// Responds to the request, returning headers and body as
        /// [`RangedResponse`]. Returns [`RangeNotSatisfiable`] error if requested
        /// range in header was not satisfiable.
        pub fn try_respond(self) -> Result<RangedResponse<B>, RangeNotSatisfiable> {
            let total_bytes = self.body.byte_size();

            // we don't support multiple byte ranges, only none or one
            // fortunately, only responding with one of the requested ranges and
            // no more seems to be compliant with the HTTP spec.
            let range = self
                .range
                .and_then(|range| range.satisfiable_ranges(total_bytes).nth(0));

            // pull seek positions out of range header
            let seek_start = match range {
                Some((Bound::Included(seek_start), _)) => seek_start,
                _ => 0,
            };

            let seek_end_excl = match range {
                // HTTP byte ranges are inclusive, so we translate to exclusive by adding 1:
                Some((_, Bound::Included(end))) => {
                    if end >= total_bytes {
                        total_bytes
                    } else {
                        end + 1
                    }
                }
                _ => total_bytes,
            };

            // check seek positions and return with 416 Range Not Satisfiable if invalid
            let seek_start_beyond_seek_end = seek_start > seek_end_excl;
            // we could use >= above but I think this reads more clearly:
            let zero_length_range = seek_start == seek_end_excl;

            if seek_start_beyond_seek_end || zero_length_range {
                let content_range = ContentRange::unsatisfied_bytes(total_bytes);
                return Err(RangeNotSatisfiable(content_range));
            }

            // if we're good, build the response
            let content_range = range.map(|_| {
                ContentRange::bytes(seek_start..seek_end_excl, total_bytes)
                    .expect("ContentRange::bytes cannot panic in this usage")
            });

            let content_length = ContentLength(seek_end_excl - seek_start);

            let stream = RangedStream::new(self.body, seek_start, content_length.0);

            Ok(RangedResponse {
                content_range,
                content_length,
                stream,
            })
        }
    }

    /// [`AsyncSeek`] narrowed to only allow seeking from start.
    pub trait AsyncSeekStart {
        /// Same semantics as [`AsyncSeek::start_seek`], always passing position as the `SeekFrom::Start` variant.
        fn start_seek(self: Pin<&mut Self>, position: u64) -> io::Result<()>;

        /// Same semantics as [`AsyncSeek::poll_complete`], returning `()` instead of the new stream position.
        fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>>;
    }

    impl<T: AsyncSeek> AsyncSeekStart for T {
        fn start_seek(self: Pin<&mut Self>, position: u64) -> io::Result<()> {
            AsyncSeek::start_seek(self, io::SeekFrom::Start(position))
        }

        fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            AsyncSeek::poll_complete(self, cx).map_ok(|_| ())
        }
    }

    /// An [`AsyncRead`] and [`AsyncSeekStart`] with a fixed known byte size.
    pub trait RangeBody: AsyncRead + AsyncSeekStart {
        /// The total size of the underlying file.
        ///
        /// This should not change for the lifetime of the object once queried.
        /// Behaviour is not guaranteed if it does change.
        fn byte_size(&self) -> u64;
    }

    impl<B: RangeBody + Send + 'static> IntoResponse for Ranged<B> {
        fn into_response(self) -> Response {
            self.try_respond().into_response()
        }
    }

    /// Error type indicating that the requested range was not satisfiable. Implements [`IntoResponse`].
    #[derive(Debug, Clone)]
    pub struct RangeNotSatisfiable(pub ContentRange);

    impl IntoResponse for RangeNotSatisfiable {
        fn into_response(self) -> Response {
            let status = StatusCode::RANGE_NOT_SATISFIABLE;
            let header = TypedHeader(self.0);
            (status, header, ()).into_response()
        }
    }

    /// Data type containing computed headers and body for a range response. Implements [`IntoResponse`].
    pub struct RangedResponse<B> {
        pub content_range: Option<ContentRange>,
        pub content_length: ContentLength,
        pub stream: RangedStream<B>,
    }

    impl<B: RangeBody + Send + 'static> IntoResponse for RangedResponse<B> {
        fn into_response(self) -> Response {
            let content_range = self.content_range.map(TypedHeader);
            let content_length = TypedHeader(self.content_length);
            let accept_ranges = TypedHeader(AcceptRanges::bytes());
            let stream = self.stream;

            let status = match content_range {
                Some(_) => StatusCode::PARTIAL_CONTENT,
                None => StatusCode::OK,
            };

            (status, content_range, content_length, accept_ranges, stream).into_response()
        }
    }

    /// Implements [`RangeBody`] for any [`AsyncRead`] and [`AsyncSeekStart`], constructed with a fixed byte size.
    #[pin_project]
    pub struct KnownSize<B: AsyncRead + AsyncSeekStart> {
        byte_size: u64,
        #[pin]
        body: B,
    }

    impl KnownSize<tokio::fs::File> {
        /// Calls [`tokio::fs::File::metadata`] to determine file size.
        pub async fn file(file: tokio::fs::File) -> io::Result<KnownSize<tokio::fs::File>> {
            let byte_size = file.metadata().await?.len();
            Ok(KnownSize {
                byte_size,
                body: file,
            })
        }
    }

    impl<B: AsyncRead + AsyncSeekStart> KnownSize<B> {
        /// Construct a [`KnownSize`] instance with a byte size supplied manually.
        pub fn sized(body: B, byte_size: u64) -> Self {
            KnownSize { byte_size, body }
        }
    }

    impl<B: AsyncRead + AsyncSeek + Unpin> KnownSize<B> {
        /// Uses `seek` to determine size by seeking to the end and getting stream position.
        pub async fn seek(mut body: B) -> io::Result<KnownSize<B>> {
            let byte_size = Pin::new(&mut body).seek(io::SeekFrom::End(0)).await?;
            Ok(KnownSize { byte_size, body })
        }
    }

    impl KnownSize<io::Cursor<Bytes>> {
        /// Uses the length of the vector as the byte size.
        pub fn bytes<B>(bytes: B) -> Self
        where
            B: Into<Bytes>,
        {
            let bytes = bytes.into();
            let byte_size = bytes.len() as u64;
            let cursor = io::Cursor::new(bytes);
            KnownSize {
                byte_size,
                body: cursor,
            }
        }
    }

    impl<B: AsyncRead + AsyncSeekStart> AsyncRead for KnownSize<B> {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let this = self.project();
            this.body.poll_read(cx, buf)
        }
    }

    impl<B: AsyncRead + AsyncSeekStart> AsyncSeekStart for KnownSize<B> {
        fn start_seek(self: Pin<&mut Self>, position: u64) -> io::Result<()> {
            let this = self.project();
            this.body.start_seek(position)
        }

        fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            let this = self.project();
            this.body.poll_complete(cx)
        }
    }

    impl<B: AsyncRead + AsyncSeekStart> RangeBody for KnownSize<B> {
        fn byte_size(&self) -> u64 {
            self.byte_size
        }
    }

    const IO_BUFFER_SIZE: usize = 64 * 1024;

    /// Response body stream. Implements [`Stream`], [`Body`], and [`IntoResponse`].
    #[pin_project]
    pub struct RangedStream<B> {
        state: StreamState,
        length: u64,
        #[pin]
        body: B,
    }

    impl<B: RangeBody + Send + 'static> RangedStream<B> {
        pub(crate) fn new(body: B, start: u64, length: u64) -> Self {
            RangedStream {
                state: StreamState::Seek { start },
                length,
                body,
            }
        }
    }

    #[derive(Debug)]
    enum StreamState {
        Seek { start: u64 },
        Seeking { remaining: u64 },
        Reading { buffer: BytesMut, remaining: u64 },
    }

    impl<B: RangeBody + Send + 'static> IntoResponse for RangedStream<B> {
        fn into_response(self) -> Response {
            Response::new(axum::body::Body::new(self))
        }
    }

    impl<B: RangeBody> Body for RangedStream<B> {
        type Data = Bytes;
        type Error = io::Error;

        fn size_hint(&self) -> SizeHint {
            SizeHint::with_exact(self.length)
        }

        fn poll_frame(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<io::Result<Frame<Bytes>>>> {
            self.poll_next(cx)
                .map(|item| item.map(|result| result.map(Frame::data)))
        }
    }

    impl<B: RangeBody> Stream for RangedStream<B> {
        type Item = io::Result<Bytes>;

        fn poll_next(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<io::Result<Bytes>>> {
            let mut this = self.project();

            if let StreamState::Seek { start } = *this.state {
                match this.body.as_mut().start_seek(start) {
                    Err(e) => return Poll::Ready(Some(Err(e))),
                    Ok(()) => {
                        let remaining = *this.length;
                        *this.state = StreamState::Seeking { remaining };
                    }
                }
            }

            if let StreamState::Seeking { remaining } = *this.state {
                match this.body.as_mut().poll_complete(cx) {
                    Poll::Pending => {
                        return Poll::Pending;
                    }
                    Poll::Ready(Err(e)) => {
                        return Poll::Ready(Some(Err(e)));
                    }
                    Poll::Ready(Ok(())) => {
                        let buffer = allocate_buffer();
                        *this.state = StreamState::Reading { buffer, remaining };
                    }
                }
            }

            if let StreamState::Reading { buffer, remaining } = this.state {
                let uninit = buffer.spare_capacity_mut();

                // calculate max number of bytes to read in this iteration, the
                // smaller of the buffer size and the number of bytes remaining
                let nbytes = std::cmp::min(
                    uninit.len(),
                    usize::try_from(*remaining).unwrap_or(usize::MAX),
                );

                let mut read_buf = ReadBuf::uninit(&mut uninit[0..nbytes]);

                match this.body.as_mut().poll_read(cx, &mut read_buf) {
                    Poll::Pending => {
                        return Poll::Pending;
                    }
                    Poll::Ready(Err(e)) => {
                        return Poll::Ready(Some(Err(e)));
                    }
                    Poll::Ready(Ok(())) => {
                        match read_buf.filled().len() {
                            0 => {
                                return Poll::Ready(None);
                            }
                            n => {
                                // SAFETY: poll_read has filled the buffer with `n`
                                // additional bytes. `buffer.len` should always be
                                // 0 here, but include it for rigorous correctness
                                unsafe {
                                    buffer.set_len(buffer.len() + n);
                                }

                                // replace state buffer and take this one to return
                                let chunk = mem::replace(buffer, allocate_buffer());

                                // subtract the number of bytes we just read from
                                // state.remaining, this usize->u64 conversion is
                                // guaranteed to always succeed, because n cannot be
                                // larger than remaining due to the cmp::min above
                                *remaining -= u64::try_from(n).unwrap();

                                // return this chunk
                                return Poll::Ready(Some(Ok(chunk.freeze())));
                            }
                        }
                    }
                }
            }

            unreachable!();
        }
    }

    fn allocate_buffer() -> BytesMut {
        BytesMut::with_capacity(IO_BUFFER_SIZE)
    }
}
