use std::future::Future;

use axum::extract::{FromRequest, Request};
// pub use axum::Form;
use dioxus_fullstack_core::RequestError;
use http::Method;
use serde::{de::DeserializeOwned, Serialize};

use crate::{ClientRequest, ClientResponse, IntoRequest};

pub struct Form<T>(pub T);

impl<T: DeserializeOwned, S: Send + Sync + 'static> FromRequest<S> for Form<T> {
    type Rejection = ();

    #[doc = " Perform the extraction."]
    fn from_request(
        _req: Request,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            axum::extract::Form::<T>::from_request(_req, _state)
                .await
                .map(|form| Form(form.0))
                .map_err(|_| ())
        }
    }
}

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
