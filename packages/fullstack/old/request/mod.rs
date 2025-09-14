use bytes::Bytes;
use futures::{FutureExt, Sink, Stream, StreamExt};
use http::{
    header::{ACCEPT, CONTENT_TYPE, REFERER},
    Method,
};
use std::{borrow::Cow, future::Future};

// /// Request types for Axum.
// // #[cfg(feature = "axum-no-default")]
// // #[cfg(feature = "axum-no-default")]
// #[cfg(feature = "server")]
// pub mod axum_impl;

// /// Request types for the browser.
// #[cfg(feature = "browser")]
// pub mod browser;
// #[cfg(feature = "generic")]
// pub mod generic;

// /// Request types for [`reqwest`].
// #[cfg(feature = "reqwest")]
// pub mod reqwest;

// Represents the request as received by the server.
// pub trait Req<Error, InputStreamError = Error, OutputStreamError = Error>
// where
//     Self: Sized,
// /// The response type for websockets.
// type WebsocketResponse: Send;

// The type used for URL-encoded form data in this client.
// type FormData;

use crate::{error::IntoAppError, FromServerFnError, ServerFnError};

#[allow(unused_variables)]
pub trait ServerFnRequestExt: Sized {
    /// Attempts to construct a new request with query parameters.
    fn try_new_req_query(
        path: &str,
        content_type: &str,
        accepts: &str,
        query: &str,
        method: Method,
    ) -> Result<Self, ServerFnError> {
        todo!()
    }

    /// Attempts to construct a new request with a text body.
    fn try_new_req_text(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: String,
        method: Method,
    ) -> Result<Self, ServerFnError> {
        todo!()
    }

    /// Attempts to construct a new request with a binary body.
    fn try_new_req_bytes(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: Bytes,
        method: Method,
    ) -> Result<Self, ServerFnError> {
        todo!()
    }

    /// Attempts to construct a new request with form data as the body.
    fn try_new_req_form_data<FormData>(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: FormData,
        method: Method,
    ) -> Result<Self, ServerFnError> {
        todo!()
    }

    /// Attempts to construct a new request with a multipart body.
    fn try_new_req_multipart<FormData>(
        path: &str,
        accepts: &str,
        body: FormData,
        method: Method,
    ) -> Result<Self, ServerFnError> {
        todo!()
    }

    /// Attempts to construct a new request with a streaming body.
    fn try_new_req_streaming(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: impl Stream<Item = Bytes> + Send + 'static,
        method: Method,
    ) -> Result<Self, ServerFnError> {
        todo!()
    }

    /// Attempts to construct a new `GET` request.
    fn try_new_get(
        path: &str,
        content_type: &str,
        accepts: &str,
        query: &str,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_query(path, content_type, accepts, query, Method::GET)
    }

    /// Attempts to construct a new `DELETE` request.
    /// **Note**: Browser support for `DELETE` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_delete(
        path: &str,
        content_type: &str,
        accepts: &str,
        query: &str,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_query(path, content_type, accepts, query, Method::DELETE)
    }

    /// Attempts to construct a new `POST` request with a text body.
    fn try_new_post(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: String,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_text(path, content_type, accepts, body, Method::POST)
    }

    /// Attempts to construct a new `PATCH` request with a text body.
    /// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_patch(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: String,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_text(path, content_type, accepts, body, Method::PATCH)
    }

    /// Attempts to construct a new `PUT` request with a text body.
    /// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_put(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: String,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_text(path, content_type, accepts, body, Method::PUT)
    }

    /// Attempts to construct a new `POST` request with a binary body.
    fn try_new_post_bytes(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: Bytes,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_bytes(path, content_type, accepts, body, Method::POST)
    }

    /// Attempts to construct a new `PATCH` request with a binary body.
    /// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_patch_bytes(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: Bytes,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_bytes(path, content_type, accepts, body, Method::PATCH)
    }

    /// Attempts to construct a new `PUT` request with a binary body.
    /// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_put_bytes(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: Bytes,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_bytes(path, content_type, accepts, body, Method::PUT)
    }

    /// Attempts to construct a new `POST` request with form data as the body.
    fn try_new_post_form_data<FormData>(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: FormData,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_form_data(path, accepts, content_type, body, Method::POST)
    }

    /// Attempts to construct a new `PATCH` request with form data as the body.
    /// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_patch_form_data<FormData>(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: FormData,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_form_data(path, accepts, content_type, body, Method::PATCH)
    }

    /// Attempts to construct a new `PUT` request with form data as the body.
    /// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_put_form_data<FormData>(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: FormData,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_form_data(path, accepts, content_type, body, Method::PUT)
    }

    /// Attempts to construct a new `POST` request with a multipart body.
    fn try_new_post_multipart<FormData>(
        path: &str,
        accepts: &str,
        body: FormData,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_multipart(path, accepts, body, Method::POST)
    }

    /// Attempts to construct a new `PATCH` request with a multipart body.
    /// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_patch_multipart<FormData>(
        path: &str,
        accepts: &str,
        body: FormData,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_multipart(path, accepts, body, Method::PATCH)
    }

    /// Attempts to construct a new `PUT` request with a multipart body.
    /// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_put_multipart<FormData>(
        path: &str,
        accepts: &str,
        body: FormData,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_multipart(path, accepts, body, Method::PUT)
    }

    /// Attempts to construct a new `POST` request with a streaming body.
    fn try_new_post_streaming(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: impl Stream<Item = Bytes> + Send + 'static,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_streaming(path, accepts, content_type, body, Method::POST)
    }

    /// Attempts to construct a new `PATCH` request with a streaming body.
    /// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_patch_streaming(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: impl Stream<Item = Bytes> + Send + 'static,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_streaming(path, accepts, content_type, body, Method::PATCH)
    }

    /// Attempts to construct a new `PUT` request with a streaming body.
    /// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
    /// Consider using a `POST` request if functionality without JS/WASM is required.
    fn try_new_put_streaming(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: impl Stream<Item = Bytes> + Send + 'static,
    ) -> Result<Self, ServerFnError> {
        Self::try_new_req_streaming(path, accepts, content_type, body, Method::PUT)
    }

    fn uri(&self) -> &http::Uri;
    fn headers(&self) -> &http::HeaderMap;
    fn into_parts(self) -> (http::request::Parts, axum::body::Body);
    fn into_body(self) -> axum::body::Body;

    fn as_query(&self) -> Option<&str> {
        self.uri().query()
    }

    fn to_content_type(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(CONTENT_TYPE)
            .map(|h| String::from_utf8_lossy(h.as_bytes()))
    }

    fn accepts(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(ACCEPT)
            .map(|h| String::from_utf8_lossy(h.as_bytes()))
    }

    fn referer(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(REFERER)
            .map(|h| String::from_utf8_lossy(h.as_bytes()))
    }

    fn try_into_bytes(
        self,
    ) -> impl std::future::Future<Output = Result<Bytes, ServerFnError>> + Send {
        let (_parts, body) = self.into_parts();

        async {
            use http_body_util::BodyExt;
            body.collect()
                .await
                .map(|c| c.to_bytes())
                .map_err(|e| ServerFnError::Deserialization(e.to_string()).into_app_error())
        }
    }

    fn try_into_string(
        self,
    ) -> impl std::future::Future<Output = Result<String, ServerFnError>> + Send {
        async {
            todo!()
            // let bytes = Req::<Error>::try_into_bytes(self).await?;
            // String::from_utf8(bytes.to_vec())
            //     .map_err(|e| ServerFnError::Deserialization(e.to_string()).into_app_error())
        }
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static, ServerFnError> {
        Ok(self.into_body().into_data_stream().map(|chunk| {
            chunk.map_err(|e| {
                ServerFnError::from_server_fn_error(ServerFnError::Deserialization(e.to_string()))
                    .ser()
            })
        }))
    }
}

#[cfg(feature = "server")]
async fn try_into_websocket(
    req: http::Request<axum::body::Body>,
) -> Result<
    (
        impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
        impl Sink<Bytes> + Send + 'static,
        http::Response<axum::body::Body>,
    ),
    ServerFnError,
> {
    use axum::extract::{ws::Message, FromRequest};
    use futures::FutureExt;

    type InputStreamError = ServerFnError;

    let upgrade = axum::extract::ws::WebSocketUpgrade::from_request(req, &())
        .await
        .map_err(|err| {
            use crate::FromServerFnError;

            ServerFnError::from_server_fn_error(ServerFnError::Request(err.to_string()))
        })?;

    let (mut outgoing_tx, outgoing_rx) =
        futures::channel::mpsc::channel::<Result<Bytes, Bytes>>(2048);
    let (incoming_tx, mut incoming_rx) = futures::channel::mpsc::channel::<Bytes>(2048);

    let response = upgrade
            .on_failed_upgrade({
                let mut outgoing_tx = outgoing_tx.clone();
                move |err: axum::Error| {
                    _ = outgoing_tx.start_send(Err(InputStreamError::from_server_fn_error(ServerFnError::Response(err.to_string())).ser()));
                }
            })
            .on_upgrade(|mut session| async move {
                loop {
                    futures::select! {
                        incoming = incoming_rx.next() => {
                            let Some(incoming) = incoming else {
                                break;
                            };
                            if let Err(err) = session.send(Message::Binary(incoming)).await {
                                _ = outgoing_tx.start_send(Err(InputStreamError::from_server_fn_error(ServerFnError::Request(err.to_string())).ser()));
                            }
                        },
                            outgoing = session.recv().fuse() => {
                            let Some(outgoing) = outgoing else {
                                break;
                            };
                            match outgoing {
                                Ok(Message::Binary(bytes)) => {
                                    _ = outgoing_tx
                                        .start_send(
                                            Ok(bytes),
                                        );
                                }
                                Ok(Message::Text(text)) => {
                                    _ = outgoing_tx.start_send(Ok(Bytes::from(text)));
                                }
                                Ok(Message::Ping(bytes)) => {
                                    if session.send(Message::Pong(bytes)).await.is_err() {
                                        break;
                                    }
                                }
                                Ok(_other) => {}
                                Err(e) => {
                                    _ = outgoing_tx.start_send(Err(InputStreamError::from_server_fn_error(ServerFnError::Response(e.to_string())).ser()));
                                }
                            }
                        }
                    }
                }
                _ = session.send(Message::Close(None)).await;
            });

    Ok((outgoing_rx, incoming_tx, response))
}
