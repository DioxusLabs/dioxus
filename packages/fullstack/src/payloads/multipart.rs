use std::{prelude::rust_2024::Future, rc::Rc};

use crate::{ClientRequest, ClientResponse, FromResponse, IntoRequest};
use axum::{
    extract::{FromRequest, Multipart, Request},
    response::IntoResponse,
};
use dioxus_fullstack_core::{RequestError, ServerFnError};
use dioxus_html::FormData;

pub struct MultipartFormData<T = ()> {
    client: Option<Rc<FormData>>,

    #[cfg(feature = "server")]
    form: Option<axum::extract::Multipart>,
    _phantom: std::marker::PhantomData<T>,
}

impl MultipartFormData {
    pub fn form(&mut self) -> dioxus_core::Result<&mut Multipart> {
        #[cfg(feature = "server")]
        {
            use anyhow::Context;

            self.form
                .as_mut()
                .context("Multipart form data has already been consumed?")
        }

        #[cfg(not(feature = "server"))]
        {
            todo!()
        }
    }
}

unsafe impl Send for MultipartFormData {}
unsafe impl Sync for MultipartFormData {}

impl<T> FromResponse for MultipartFormData<T> {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move { todo!() }
    }
}
impl<S> IntoRequest for MultipartFormData<S> {
    fn into_request(
        self,
        builder: ClientRequest,
    ) -> impl Future<Output = Result<ClientResponse, RequestError>> + 'static {
        async move {
            #[cfg(feature = "web")]
            {
                use wasm_bindgen::JsCast;

                let data = self.client.unwrap();
                let event: &web_sys::Event = data.downcast().unwrap();
                let target = event.target().unwrap();
                let form: &web_sys::HtmlFormElement = target.dyn_ref().unwrap();
                let data = web_sys::FormData::new_with_form(form).unwrap();
                builder.send_web_form(data).await
            }

            #[cfg(not(feature = "web"))]
            {
                todo!()
            }
        }
    }
}
impl<S: Send + Sync + 'static, D> FromRequest<S> for MultipartFormData<D> {
    type Rejection = axum::response::Response;

    #[doc = " Perform the extraction."]
    fn from_request(
        req: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let data = axum::extract::multipart::Multipart::from_request(req, state)
                .await
                .map_err(|err| {
                    tracing::error!("Failed to extract multipart form data: {:?}", err);
                    err.into_response()
                })?;

            Ok(MultipartFormData {
                #[cfg(feature = "server")]
                form: Some(data),
                client: None,
                _phantom: std::marker::PhantomData,
            })
        }
    }
}

impl<T> IntoResponse for MultipartFormData<T> {
    fn into_response(self) -> axum::response::Response {
        todo!()
    }
}

impl<T> From<Rc<FormData>> for MultipartFormData<T> {
    fn from(_value: Rc<FormData>) -> Self {
        MultipartFormData {
            #[cfg(feature = "server")]
            form: None,
            client: Some(_value),
            _phantom: std::marker::PhantomData,
        }
    }
}
