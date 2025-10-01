use super::*;
use axum_core::extract::Request;
use dioxus_html::FileData;
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
#[derive(Debug)]
pub struct FileUpload {
    data: Option<FileData>,
    name: String,
    size: Option<u64>,
    content_type: Option<String>,
    #[cfg(feature = "server")]
    body: Option<axum_core::body::BodyDataStream>, // outgoing_stream: Option<http_body_util::BodyDataStream<Request<Body>>>,
                                                   // content_type: Option<String>,
                                                   // filename: Option<String>,
}

unsafe impl Send for FileUpload {}
unsafe impl Sync for FileUpload {}

impl FileUpload {
    pub fn file_name(&self) -> &str {
        &self.name
    }

    pub fn size(&self) -> Option<u64> {
        self.size
    }

    pub fn content_type(&self, content_type: &str) -> Option<&str> {
        self.content_type.as_deref()
    }

    #[cfg(feature = "server")]
    pub fn body_mut(&mut self) -> &mut axum_core::body::BodyDataStream {
        self.body.as_mut().unwrap()
    }
}

impl Stream for FileUpload {
    type Item = Result<Bytes, dioxus_core::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        #[cfg(feature = "server")]
        {
            let this = self.get_mut();
            return Pin::new(this.body_mut())
                .poll_next(cx)
                .map_err(|e| e.into());
        }

        #[cfg(not(feature = "server"))]
        {
            todo!()
            // let this = self.get_mut();
            // if let Some(data) = this.data.take() {
            //     let stream = data.byte_stream();
            //     this.data = None; // Ensure we only take the data once
            //     Pin::new(&mut stream.into_stream())
            //         .poll_next(cx)
            //         .map_err(|e| e.into())
            // } else {
            //     Poll::Ready(None)
            // }
        }
    }
}

impl IntoRequest for FileUpload {
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
                    .header("Content-Type", content_type)
                    .header("Content-Length", content_length.clone())
                    .header("X-Content-Size", content_length)
                    .header(
                        "Content-Disposition",
                        format!(
                            "attachment; filename=\"{}\"",
                            escape(&file.name().to_string())
                        ),
                    );

                return builder.send_js_value(as_blob.clone().into()).await;
            }

            todo!()
        }
    }
}

impl<S> FromRequest<S> for FileUpload {
    type Rejection = ServerFnRejection;

    fn from_request(
        req: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            tracing::info!("Extracting FileUpload from request: {:?}", req);

            let disposition = req.headers().get("Content-Disposition");
            let filename = match disposition.map(|s| s.to_str()) {
                Some(Ok(dis)) => {
                    let content = content_disposition::parse_content_disposition(dis);
                    let names = content.filename();
                    match names {
                        Some((name, Some(filename))) => filename.to_string(),
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

            Ok(FileUpload {
                data: None,
                name: filename,
                content_type,
                size,
                #[cfg(feature = "server")]
                body: Some(stream),
            })
        }
    }
}

impl FromResponse for FileUpload {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move { todo!() }
    }
}

impl From<FileData> for FileUpload {
    fn from(value: FileData) -> Self {
        Self {
            name: value.name().to_string(),
            content_type: value.content_type().map(|s| s.to_string()),
            size: Some(value.size() as u64),
            data: Some(value),
            #[cfg(feature = "server")]
            body: None,
        }
    }
}
