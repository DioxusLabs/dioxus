use std::future::Future;

pub use axum::extract::Form;
use axum::extract::{FromRequest, Request};
use dioxus_fullstack_core::RequestError;
use http::Method;
use serde::{de::DeserializeOwned, Serialize};

use crate::{ClientRequest, ClientResponse, IntoRequest};

impl<T> IntoRequest for Form<T>
where
    T: Serialize + 'static,
{
    fn into_request(
        self,
        req: ClientRequest,
    ) -> impl Future<Output = Result<ClientResponse, RequestError>> + 'static {
        send_wrapper::SendWrapper::new(async move {
            let Form(value) = self;
            let ClientRequest { client, method } = req;

            let is_get_or_head = method == Method::GET || method == Method::HEAD;

            if is_get_or_head {
                ClientRequest {
                    client: client.query(&value),
                    method,
                }
                .send()
                .await
            } else {
                let body = serde_urlencoded::to_string(&value)
                    .map_err(|err| RequestError::Body(err.to_string()))?;

                ClientRequest {
                    client: client
                        .header("Content-Type", "application/x-www-form-urlencoded")
                        .body(body),
                    method,
                }
                .send()
                .await
            }
        })
    }
}
