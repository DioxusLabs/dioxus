use super::ClientReq;
use crate::{
    client::get_server_url,
    error::{FromServerFnError, IntoAppError, ServerFnErrorErr},
};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use reqwest::{
    header::{ACCEPT, CONTENT_TYPE},
    Body,
};
pub use reqwest::{multipart::Form, Client, Method, Request, Url};
use std::sync::LazyLock;

pub(crate) static CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

impl<E> ClientReq<E> for Request
where
    E: FromServerFnError,
{
    type FormData = Form;

    fn try_new_req_query(
        path: &str,
        content_type: &str,
        accepts: &str,
        query: &str,
        method: Method,
    ) -> Result<Self, E> {
        let url = format!("{}{}", get_server_url(), path);
        let mut url = Url::try_from(url.as_str()).map_err(|e| {
            E::from_server_fn_error(ServerFnErrorErr::Request(e.to_string()))
        })?;
        url.set_query(Some(query));
        let req = match method {
            Method::GET => CLIENT.get(url),
            Method::DELETE => CLIENT.delete(url),
            Method::HEAD => CLIENT.head(url),
            Method::POST => CLIENT.post(url),
            Method::PATCH => CLIENT.patch(url),
            Method::PUT => CLIENT.put(url),
            m => {
                return Err(E::from_server_fn_error(
                    ServerFnErrorErr::UnsupportedRequestMethod(m.to_string()),
                ))
            }
        }
        .header(CONTENT_TYPE, content_type)
        .header(ACCEPT, accepts)
        .build()
        .map_err(|e| {
            E::from_server_fn_error(ServerFnErrorErr::Request(e.to_string()))
        })?;
        Ok(req)
    }

    fn try_new_req_text(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: String,
        method: Method,
    ) -> Result<Self, E> {
        let url = format!("{}{}", get_server_url(), path);
        match method {
            Method::POST => CLIENT.post(url),
            Method::PUT => CLIENT.put(url),
            Method::PATCH => CLIENT.patch(url),
            m => {
                return Err(E::from_server_fn_error(
                    ServerFnErrorErr::UnsupportedRequestMethod(m.to_string()),
                ))
            }
        }
        .header(CONTENT_TYPE, content_type)
        .header(ACCEPT, accepts)
        .body(body)
        .build()
        .map_err(|e| ServerFnErrorErr::Request(e.to_string()).into_app_error())
    }

    fn try_new_req_bytes(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: Bytes,
        method: Method,
    ) -> Result<Self, E> {
        let url = format!("{}{}", get_server_url(), path);
        match method {
            Method::POST => CLIENT.post(url),
            Method::PATCH => CLIENT.patch(url),
            Method::PUT => CLIENT.put(url),
            m => {
                return Err(E::from_server_fn_error(
                    ServerFnErrorErr::UnsupportedRequestMethod(m.to_string()),
                ))
            }
        }
        .header(CONTENT_TYPE, content_type)
        .header(ACCEPT, accepts)
        .body(body)
        .build()
        .map_err(|e| ServerFnErrorErr::Request(e.to_string()).into_app_error())
    }

    fn try_new_req_multipart(
        path: &str,
        accepts: &str,
        body: Self::FormData,
        method: Method,
    ) -> Result<Self, E> {
        match method {
            Method::POST => CLIENT.post(path),
            Method::PUT => CLIENT.put(path),
            Method::PATCH => CLIENT.patch(path),
            m => {
                return Err(E::from_server_fn_error(
                    ServerFnErrorErr::UnsupportedRequestMethod(m.to_string()),
                ))
            }
        }
        .header(ACCEPT, accepts)
        .multipart(body)
        .build()
        .map_err(|e| ServerFnErrorErr::Request(e.to_string()).into_app_error())
    }

    fn try_new_req_form_data(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: Self::FormData,
        method: Method,
    ) -> Result<Self, E> {
        match method {
            Method::POST => CLIENT.post(path),
            Method::PATCH => CLIENT.patch(path),
            Method::PUT => CLIENT.put(path),
            m => {
                return Err(E::from_server_fn_error(
                    ServerFnErrorErr::UnsupportedRequestMethod(m.to_string()),
                ))
            }
        }
        .header(CONTENT_TYPE, content_type)
        .header(ACCEPT, accepts)
        .multipart(body)
        .build()
        .map_err(|e| ServerFnErrorErr::Request(e.to_string()).into_app_error())
    }

    fn try_new_req_streaming(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: impl Stream<Item = Bytes> + Send + 'static,
        method: Method,
    ) -> Result<Self, E> {
        let url = format!("{}{}", get_server_url(), path);
        let body = Body::wrap_stream(
            body.map(|chunk| Ok(chunk) as Result<Bytes, ServerFnErrorErr>),
        );
        match method {
            Method::POST => CLIENT.post(url),
            Method::PUT => CLIENT.put(url),
            Method::PATCH => CLIENT.patch(url),
            m => {
                return Err(E::from_server_fn_error(
                    ServerFnErrorErr::UnsupportedRequestMethod(m.to_string()),
                ))
            }
        }
        .header(CONTENT_TYPE, content_type)
        .header(ACCEPT, accepts)
        .body(body)
        .build()
        .map_err(|e| ServerFnErrorErr::Request(e.to_string()).into_app_error())
    }
}
