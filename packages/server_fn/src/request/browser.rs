use super::ClientReq;
use crate::{
    client::get_server_url,
    error::{FromServerFnError, ServerFnErrorErr},
};
use bytes::Bytes;
use futures::{Stream, StreamExt};
pub use gloo_net::http::Request;
use http::Method;
use js_sys::{Reflect, Uint8Array};
use send_wrapper::SendWrapper;
use std::ops::{Deref, DerefMut};
use wasm_bindgen::JsValue;
use wasm_streams::ReadableStream;
use web_sys::{
    AbortController, AbortSignal, FormData, Headers, RequestInit,
    UrlSearchParams,
};

/// A `fetch` request made in the browser.
#[derive(Debug)]
pub struct BrowserRequest(pub(crate) SendWrapper<RequestInner>);

#[derive(Debug)]
pub(crate) struct RequestInner {
    pub(crate) request: Request,
    pub(crate) abort_ctrl: Option<AbortOnDrop>,
}

#[derive(Debug)]
pub(crate) struct AbortOnDrop(Option<AbortController>);

impl AbortOnDrop {
    /// Prevents the request from being aborted on drop.
    pub fn prevent_cancellation(&mut self) {
        self.0.take();
    }
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        if let Some(inner) = self.0.take() {
            inner.abort();
        }
    }
}

impl From<BrowserRequest> for Request {
    fn from(value: BrowserRequest) -> Self {
        value.0.take().request
    }
}

impl From<BrowserRequest> for web_sys::Request {
    fn from(value: BrowserRequest) -> Self {
        value.0.take().request.into()
    }
}

impl Deref for BrowserRequest {
    type Target = Request;

    fn deref(&self) -> &Self::Target {
        &self.0.deref().request
    }
}

impl DerefMut for BrowserRequest {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.deref_mut().request
    }
}

/// The `FormData` type available in the browser.
#[derive(Debug)]
pub struct BrowserFormData(pub(crate) SendWrapper<FormData>);

impl BrowserFormData {
    /// Returns the raw `web_sys::FormData` struct.
    pub fn take(self) -> FormData {
        self.0.take()
    }
}

impl From<FormData> for BrowserFormData {
    fn from(value: FormData) -> Self {
        Self(SendWrapper::new(value))
    }
}

fn abort_signal() -> (Option<AbortOnDrop>, Option<AbortSignal>) {
    let ctrl = AbortController::new().ok();
    let signal = ctrl.as_ref().map(|ctrl| ctrl.signal());
    (ctrl.map(|ctrl| AbortOnDrop(Some(ctrl))), signal)
}

impl<E> ClientReq<E> for BrowserRequest
where
    E: FromServerFnError,
{
    type FormData = BrowserFormData;

    fn try_new_req_query(
        path: &str,
        content_type: &str,
        accepts: &str,
        query: &str,
        method: http::Method,
    ) -> Result<Self, E> {
        let (abort_ctrl, abort_signal) = abort_signal();
        let server_url = get_server_url();
        let mut url = String::with_capacity(
            server_url.len() + path.len() + 1 + query.len(),
        );
        url.push_str(server_url);
        url.push_str(path);
        url.push('?');
        url.push_str(query);
        Ok(Self(SendWrapper::new(RequestInner {
            request: match method {
                Method::GET => Request::get(&url),
                Method::DELETE => Request::delete(&url),
                Method::POST => Request::post(&url),
                Method::PUT => Request::put(&url),
                Method::PATCH => Request::patch(&url),
                m => {
                    return Err(E::from_server_fn_error(
                        ServerFnErrorErr::UnsupportedRequestMethod(
                            m.to_string(),
                        ),
                    ))
                }
            }
            .header("Content-Type", content_type)
            .header("Accept", accepts)
            .abort_signal(abort_signal.as_ref())
            .build()
            .map_err(|e| {
                E::from_server_fn_error(ServerFnErrorErr::Request(
                    e.to_string(),
                ))
            })?,
            abort_ctrl,
        })))
    }

    fn try_new_req_text(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: String,
        method: Method,
    ) -> Result<Self, E> {
        let (abort_ctrl, abort_signal) = abort_signal();
        let server_url = get_server_url();
        let mut url = String::with_capacity(server_url.len() + path.len());
        url.push_str(server_url);
        url.push_str(path);
        Ok(Self(SendWrapper::new(RequestInner {
            request: match method {
                Method::POST => Request::post(&url),
                Method::PATCH => Request::patch(&url),
                Method::PUT => Request::put(&url),
                m => {
                    return Err(E::from_server_fn_error(
                        ServerFnErrorErr::UnsupportedRequestMethod(
                            m.to_string(),
                        ),
                    ))
                }
            }
            .header("Content-Type", content_type)
            .header("Accept", accepts)
            .abort_signal(abort_signal.as_ref())
            .body(body)
            .map_err(|e| {
                E::from_server_fn_error(ServerFnErrorErr::Request(
                    e.to_string(),
                ))
            })?,
            abort_ctrl,
        })))
    }

    fn try_new_req_bytes(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: Bytes,
        method: Method,
    ) -> Result<Self, E> {
        let (abort_ctrl, abort_signal) = abort_signal();
        let server_url = get_server_url();
        let mut url = String::with_capacity(server_url.len() + path.len());
        url.push_str(server_url);
        url.push_str(path);
        let body: &[u8] = &body;
        let body = Uint8Array::from(body).buffer();
        Ok(Self(SendWrapper::new(RequestInner {
            request: match method {
                Method::POST => Request::post(&url),
                Method::PATCH => Request::patch(&url),
                Method::PUT => Request::put(&url),
                m => {
                    return Err(E::from_server_fn_error(
                        ServerFnErrorErr::UnsupportedRequestMethod(
                            m.to_string(),
                        ),
                    ))
                }
            }
            .header("Content-Type", content_type)
            .header("Accept", accepts)
            .abort_signal(abort_signal.as_ref())
            .body(body)
            .map_err(|e| {
                E::from_server_fn_error(ServerFnErrorErr::Request(
                    e.to_string(),
                ))
            })?,
            abort_ctrl,
        })))
    }

    fn try_new_req_multipart(
        path: &str,
        accepts: &str,
        body: Self::FormData,
        method: Method,
    ) -> Result<Self, E> {
        let (abort_ctrl, abort_signal) = abort_signal();
        let server_url = get_server_url();
        let mut url = String::with_capacity(server_url.len() + path.len());
        url.push_str(server_url);
        url.push_str(path);
        Ok(Self(SendWrapper::new(RequestInner {
            request: match method {
                Method::POST => Request::post(&url),
                Method::PATCH => Request::patch(&url),
                Method::PUT => Request::put(&url),
                m => {
                    return Err(E::from_server_fn_error(
                        ServerFnErrorErr::UnsupportedRequestMethod(
                            m.to_string(),
                        ),
                    ))
                }
            }
            .header("Accept", accepts)
            .abort_signal(abort_signal.as_ref())
            .body(body.0.take())
            .map_err(|e| {
                E::from_server_fn_error(ServerFnErrorErr::Request(
                    e.to_string(),
                ))
            })?,
            abort_ctrl,
        })))
    }

    fn try_new_req_form_data(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: Self::FormData,
        method: Method,
    ) -> Result<Self, E> {
        let (abort_ctrl, abort_signal) = abort_signal();
        let form_data = body.0.take();
        let url_params =
            UrlSearchParams::new_with_str_sequence_sequence(&form_data)
                .map_err(|e| {
                    E::from_server_fn_error(ServerFnErrorErr::Serialization(
                        e.as_string().unwrap_or_else(|| {
                            "Could not serialize FormData to URLSearchParams"
                                .to_string()
                        }),
                    ))
                })?;
        Ok(Self(SendWrapper::new(RequestInner {
            request: match method {
                Method::POST => Request::post(path),
                Method::PUT => Request::put(path),
                Method::PATCH => Request::patch(path),
                m => {
                    return Err(E::from_server_fn_error(
                        ServerFnErrorErr::UnsupportedRequestMethod(
                            m.to_string(),
                        ),
                    ))
                }
            }
            .header("Content-Type", content_type)
            .header("Accept", accepts)
            .abort_signal(abort_signal.as_ref())
            .body(url_params)
            .map_err(|e| {
                E::from_server_fn_error(ServerFnErrorErr::Request(
                    e.to_string(),
                ))
            })?,
            abort_ctrl,
        })))
    }

    fn try_new_req_streaming(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: impl Stream<Item = Bytes> + 'static,
        method: Method,
    ) -> Result<Self, E> {
        // Only allow for methods with bodies
        match method {
            Method::POST | Method::PATCH | Method::PUT => {}
            m => {
                return Err(E::from_server_fn_error(
                    ServerFnErrorErr::UnsupportedRequestMethod(m.to_string()),
                ))
            }
        }
        // TODO abort signal
        let (request, abort_ctrl) =
            streaming_request(path, accepts, content_type, body, method)
                .map_err(|e| {
                    E::from_server_fn_error(ServerFnErrorErr::Request(format!(
                        "{e:?}"
                    )))
                })?;
        Ok(Self(SendWrapper::new(RequestInner {
            request,
            abort_ctrl,
        })))
    }
}

fn streaming_request(
    path: &str,
    accepts: &str,
    content_type: &str,
    body: impl Stream<Item = Bytes> + 'static,
    method: Method,
) -> Result<(Request, Option<AbortOnDrop>), JsValue> {
    let (abort_ctrl, abort_signal) = abort_signal();
    let stream = ReadableStream::from_stream(body.map(|bytes| {
        let data = Uint8Array::from(bytes.as_ref());
        let data = JsValue::from(data);
        Ok(data) as Result<JsValue, JsValue>
    }))
    .into_raw();

    let headers = Headers::new()?;
    headers.append("Content-Type", content_type)?;
    headers.append("Accept", accepts)?;

    let init = RequestInit::new();
    init.set_headers(&headers);
    init.set_method(method.as_str());
    init.set_signal(abort_signal.as_ref());
    init.set_body(&stream);

    // Chrome requires setting `duplex: "half"` on streaming requests
    Reflect::set(
        &init,
        &JsValue::from_str("duplex"),
        &JsValue::from_str("half"),
    )?;
    let req = web_sys::Request::new_with_str_and_init(path, &init)?;
    Ok((Request::from(req), abort_ctrl))
}
