use super::*;
use axum_core::extract::Request;
use dioxus_fullstack_core::RequestError;
use dioxus_html::FileData;

#[cfg(feature = "server")]
use std::path::Path;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

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
    pub async fn from_path(file: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        Self::from_path_buf(file.as_ref()).await
    }

    #[cfg(feature = "server")]
    async fn from_path_buf(file: &Path) -> Result<Self, std::io::Error> {
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
    #[allow(unreachable_code)]
    fn into_request(
        self,
        #[allow(unused_mut)] mut builder: ClientRequest,
    ) -> impl Future<Output = ClientResult> + 'static {
        async move {
            let Some(file_data) = self.data else {
                return Err(RequestError::Request(
                    "FileStream has no data to send".into(),
                ));
            };

            #[cfg(feature = "web")]
            if cfg!(target_arch = "wasm32") {
                use js_sys::escape;
                use wasm_bindgen::JsCast;

                let as_file = file_data.inner().downcast_ref::<web_sys::File>().unwrap();
                let as_blob = as_file.dyn_ref::<web_sys::Blob>().unwrap();
                let content_type = as_blob.type_();
                let content_length = as_blob.size().to_string();
                let name = as_file.name();

                // Set both Content-Length and X-Content-Size for compatibility with server extraction.
                // In browsers, content-length is often overwritten, so we set X-Content-Size as well
                // for better compatibility with dioxus-based clients.
                return builder
                    .header("Content-Type", content_type)?
                    .header("Content-Length", content_length.clone())?
                    .header("X-Content-Size", content_length)?
                    .header(
                        "Content-Disposition",
                        format!("attachment; filename=\"{}\"", escape(&name)),
                    )?
                    .send_js_value(as_blob.clone().into())
                    .await;
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                use std::ascii::escape_default;

                use futures::TryStreamExt;

                let content_type = self
                    .content_type
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                let content_length = self.size.map(|s| s.to_string());
                let name = self.name;
                let stream = file_data.byte_stream().map_err(|_| StreamingError::Failed);

                // Ascii escape the filename to avoid issues with special characters.
                let mut chars = vec![];
                for byte in name.chars() {
                    chars.extend(escape_default(byte as u8));
                }
                let filename = String::from_utf8(chars).map_err(|_| {
                    RequestError::Request(
                        "Failed to escape filename for Content-Disposition".into(),
                    )
                });

                if let Some(length) = content_length {
                    builder = builder.header("Content-Length", length)?;
                }

                if let Ok(filename) = filename {
                    builder = builder.header(
                        "Content-Disposition",
                        format!("attachment; filename=\"{}\"", filename),
                    )?;
                }

                return builder
                    .header("Content-Type", content_type)?
                    .send_body_stream(stream)
                    .await;
            }

            unimplemented!("FileStream::into_request is only implemented for web targets");
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
                    content
                        .filename_full()
                        .unwrap_or_else(|| "file".to_string())
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

            Ok(FileStream {
                data: None,
                name: filename,
                content_type,
                size,
                client_body: None,
                #[cfg(feature = "server")]
                server_body: Some(req.into_body().into_data_stream()),
            })
        }
    }
}

impl FromResponse for FileStream {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move {
            // Check status code first - don't try to stream error responses as files
            if !res.status().is_success() {
                let status_code = res.status().as_u16();
                let canonical_reason = res
                    .status()
                    .canonical_reason()
                    .unwrap_or("Unknown error")
                    .to_string();
                let bytes = res.bytes().await.unwrap_or_default();
                let message = String::from_utf8(bytes.to_vec()).unwrap_or(canonical_reason);

                return Err(ServerFnError::ServerError {
                    message,
                    code: status_code,
                    details: None,
                });
            }

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

            Ok(Self {
                data: None,
                name,
                size,
                content_type,
                client_body: Some(Box::pin(res.bytes_stream())),
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
            client_body: None,
            #[cfg(feature = "server")]
            server_body: None,
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
