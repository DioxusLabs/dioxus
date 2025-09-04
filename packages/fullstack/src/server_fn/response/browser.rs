use super::ClientRes;
use crate::{
    error::{FromServerFnError, IntoAppError, ServerFnErrorErr},
    redirect::REDIRECT_HEADER,
};
use bytes::Bytes;
use futures::{Stream, StreamExt};
pub use gloo_net::http::Response;
use http::{HeaderMap, HeaderName, HeaderValue};
use js_sys::Uint8Array;
use send_wrapper::SendWrapper;
use std::{future::Future, str::FromStr};
use wasm_bindgen::JsCast;
use wasm_streams::ReadableStream;

/// The response to a `fetch` request made in the browser.
pub struct BrowserResponse(pub(crate) SendWrapper<Response>);

impl BrowserResponse {
    /// Generate the headers from the internal [`Response`] object.
    /// This is a workaround for the fact that the `Response` object does not
    /// have a [`HeaderMap`] directly. This function will iterate over the
    /// headers and convert them to a [`HeaderMap`].
    pub fn generate_headers(&self) -> HeaderMap {
        self.0
            .headers()
            .entries()
            .filter_map(|(key, value)| {
                let key = HeaderName::from_str(&key).ok()?;
                let value = HeaderValue::from_str(&value).ok()?;
                Some((key, value))
            })
            .collect()
    }
}

impl<E: FromServerFnError> ClientRes<E> for BrowserResponse {
    fn try_into_string(self) -> impl Future<Output = Result<String, E>> + Send {
        // the browser won't send this async work between threads (because it's single-threaded)
        // so we can safely wrap this
        SendWrapper::new(async move {
            self.0.text().await.map_err(|e| {
                ServerFnErrorErr::Deserialization(e.to_string())
                    .into_app_error()
            })
        })
    }

    fn try_into_bytes(self) -> impl Future<Output = Result<Bytes, E>> + Send {
        // the browser won't send this async work between threads (because it's single-threaded)
        // so we can safely wrap this
        SendWrapper::new(async move {
            self.0.binary().await.map(Bytes::from).map_err(|e| {
                ServerFnErrorErr::Deserialization(e.to_string())
                    .into_app_error()
            })
        })
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static, E>
    {
        let stream = ReadableStream::from_raw(self.0.body().unwrap())
            .into_stream()
            .map(|data| match data {
                Err(e) => {
                    web_sys::console::error_1(&e);
                    Err(E::from_server_fn_error(ServerFnErrorErr::Request(
                        format!("{e:?}"),
                    ))
                    .ser())
                }
                Ok(data) => {
                    let data = data.unchecked_into::<Uint8Array>();
                    let mut buf = Vec::new();
                    let length = data.length();
                    buf.resize(length as usize, 0);
                    data.copy_to(&mut buf);
                    Ok(Bytes::from(buf))
                }
            });
        Ok(SendWrapper::new(stream))
    }

    fn status(&self) -> u16 {
        self.0.status()
    }

    fn status_text(&self) -> String {
        self.0.status_text()
    }

    fn location(&self) -> String {
        self.0
            .headers()
            .get("Location")
            .unwrap_or_else(|| self.0.url())
    }

    fn has_redirect(&self) -> bool {
        self.0.headers().get(REDIRECT_HEADER).is_some()
    }
}
