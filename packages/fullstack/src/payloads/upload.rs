use super::*;
use axum_core::extract::Request;
use dioxus_html::FileData;
#[cfg(feature = "server")]
use std::path::PathBuf;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use wasm_bindgen::JsCast;

/// A payload for uploading files using streams.
///
/// The `FileUpload` struct allows you to upload files by streaming their data. It can be constructed
/// from a stream of bytes and can be sent as part of an HTTP request. This is particularly useful for
/// handling large files without loading them entirely into memory.
///
/// On the web, this uses the `ReadableStream` API to stream file data.
pub struct FileStream {
    data: Option<FileData>,
    name: String,
    size: Option<u64>,
    content_type: Option<String>,
    #[cfg(feature = "server")]
    server_body: Option<axum_core::body::BodyDataStream>,

    // For downloaded files...
    #[allow(clippy::type_complexity)]
    client_body: Option<Pin<Box<dyn Stream<Item = Result<Bytes, StreamingError>> + Send>>>,
}

impl FileStream {
    /// Get the name of the file.
    pub fn file_name(&self) -> &str {
        &self.name
    }

    /// Get the size of the file, if known.
    pub fn size(&self) -> Option<u64> {
        self.size
    }

    /// Get the content type of the file, if available.
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// Return the underlying body stream, assuming the `FileStream` was created by a server request.
    #[cfg(feature = "server")]
    pub fn body_mut(&mut self) -> Option<&mut axum_core::body::BodyDataStream> {
        self.server_body.as_mut()
    }

    /// Create a new `FileStream` from a file path. This is only available on the server.
    #[cfg(feature = "server")]
    pub async fn from_file(file: PathBuf) -> Result<Self, std::io::Error> {
        let metadata = file.metadata()?;
        let contents = tokio::fs::File::open(&file).await?;
        let mime = dioxus_asset_resolver::native::get_mime_from_ext(
            file.extension().and_then(|s| s.to_str()),
        );
        let size = metadata.len();
        let name = file
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("file")
            .to_string();

        // Convert the tokio file into an async byte stream
        let reader_stream = tokio_util::io::ReaderStream::new(contents);

        // Attempt to construct a BodyDataStream from the reader stream.
        // Many axum_core versions provide a `from_stream` or similar constructor.
        let body = axum_core::body::Body::from_stream(reader_stream).into_data_stream();

        Ok(Self {
            data: None,
            name,
            size: Some(size),
            content_type: Some(mime.to_string()),
            #[cfg(feature = "server")]
            server_body: Some(body),
            client_body: None,
        })
    }

    /// Create a new `FileStream` from raw components.
    ///
    /// This is meant to be used on the server where a file might not even exist but you still want
    /// to stream it to the client as a download.
    #[cfg(feature = "server")]
    pub fn from_raw(
        name: String,
        size: Option<u64>,
        content_type: String,
        body: axum_core::body::BodyDataStream,
    ) -> Self {
        Self {
            data: None,
            name,
            size,
            content_type: Some(content_type),
            #[cfg(feature = "server")]
            server_body: Some(body),
            client_body: None,
        }
    }
}

impl IntoRequest for FileStream {
    fn into_request(self, builder: ClientRequest) -> impl Future<Output = ClientResult> + 'static {
        async move {
            #[cfg(feature = "web")]
            {
                use js_sys::escape;

                let file = self.data.unwrap();
                let as_file = file.inner().downcast_ref::<web_sys::File>().unwrap();
                let as_blob = as_file.dyn_ref::<web_sys::Blob>().unwrap();
                let content_type = as_blob.type_();
                let content_length = as_blob.size().to_string();

                // Set both Content-Length and X-Content-Size for compatibility with server extraction.
                // In browsers, content-length is often overwritten, so we set X-Content-Size as well
                // for better compatibility with dioxus-based clients.
                let builder = builder
                    .header("Content-Type", content_type)?
                    .header("Content-Length", content_length.clone())?
                    .header("X-Content-Size", content_length)?
                    .header(
                        "Content-Disposition",
                        format!(
                            "attachment; filename=\"{}\"",
                            escape(&file.name().to_string())
                        ),
                    )?;

                return builder.send_js_value(as_blob.clone().into()).await;
            }

            todo!()
        }
    }
}

impl<S> FromRequest<S> for FileStream {
    type Rejection = ServerFnError;

    fn from_request(
        req: Request,
        _: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            tracing::info!("Extracting FileUpload from request: {:?}", req);

            let disposition = req.headers().get("Content-Disposition");
            let filename = match disposition.map(|s| s.to_str()) {
                Some(Ok(dis)) => {
                    let content = content_disposition::parse_content_disposition(dis);
                    let names = content.filename();
                    match names {
                        Some((_name, Some(filename))) => filename.to_string(),
                        Some((name, None)) => name.to_string(),
                        None => "file".to_string(),
                    }
                }
                _ => "file".to_string(),
            };

            // Content-length is unreliable, so we use `X-Content-Size` as an indicator.
            // For stream requests with known bodies, the browser will still set Content-Length to 0, unfortunately.
            let size = req
                .headers()
                .get("X-Content-Size")
                .and_then(|s| s.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());

            let content_type = req
                .headers()
                .get("Content-Type")
                .and_then(|s| s.to_str().ok())
                .map(|s| s.to_string());

            let stream = req.into_body().into_data_stream();

            Ok(FileStream {
                data: None,
                name: filename,
                content_type,
                size,
                #[cfg(feature = "server")]
                server_body: Some(stream),
                client_body: None,
            })
        }
    }
}

impl FromResponse for FileStream {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move {
            // Extract filename from Content-Disposition header if present.
            let name = res
                .headers()
                .get("Content-Disposition")
                .and_then(|h| h.to_str().ok())
                .and_then(|dis| {
                    let cd = content_disposition::parse_content_disposition(dis);
                    cd.filename().map(|(name, _)| name.to_string())
                })
                .unwrap_or_else(|| "file".to_string());

            // Extract content type header
            let content_type = res
                .headers()
                .get("Content-Type")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());

            // Prefer the response's known content length but fall back to X-Content-Size header.
            let size = res.content_length().or_else(|| {
                res.headers()
                    .get("X-Content-Size")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
            });

            let res = res.bytes_stream();

            Ok(Self {
                data: None,
                name,
                size,
                content_type,
                client_body: Some(Box::pin(res)),
                #[cfg(feature = "server")]
                server_body: None,
            })
        }
    }
}

#[cfg(feature = "server")]
impl IntoResponse for FileStream {
    fn into_response(self) -> axum::response::Response {
        use axum::body::Body;

        let Some(body) = self.server_body else {
            use dioxus_fullstack_core::HttpError;
            return HttpError::new(http::StatusCode::BAD_REQUEST, "FileStream has no body")
                .into_response();
        };

        let mut res = axum::response::Response::new(Body::from_stream(body));

        // Set relevant headers if available
        if let Some(content_type) = &self.content_type {
            res.headers_mut()
                .insert("Content-Type", content_type.parse().unwrap());
        }
        if let Some(size) = self.size {
            res.headers_mut()
                .insert("Content-Length", size.to_string().parse().unwrap());
        }
        res.headers_mut().insert(
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", self.name)
                .parse()
                .unwrap(),
        );

        res
    }
}

impl From<FileData> for FileStream {
    fn from(value: FileData) -> Self {
        Self {
            name: value.name().to_string(),
            content_type: value.content_type().map(|s| s.to_string()),
            size: Some(value.size()),
            data: Some(value),

            #[cfg(feature = "server")]
            server_body: None,

            client_body: None,
        }
    }
}

impl Stream for FileStream {
    type Item = Result<Bytes, StreamingError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // For server-side builds, poll the server_body stream if it exists.
        #[cfg(feature = "server")]
        if let Some(body) = self.server_body.as_mut() {
            return Pin::new(body)
                .poll_next(cx)
                .map_err(|_| StreamingError::Failed);
        }

        // For client-side builds, poll the client_body stream if it exists.
        if let Some(body) = self.client_body.as_mut() {
            return body.as_mut().poll_next(cx);
        }

        // Otherwise, the stream is exhausted.
        Poll::Ready(None)
    }
}

impl std::fmt::Debug for FileStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileStream")
            .field("name", &self.name)
            .field("size", &self.size)
            .field("content_type", &self.content_type)
            .finish()
    }
}
