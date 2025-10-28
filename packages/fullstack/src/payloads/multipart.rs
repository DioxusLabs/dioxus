#![allow(unreachable_code)]

use crate::{ClientRequest, ClientResponse, IntoRequest};
use axum::{
    extract::{FromRequest, Request},
    response::{IntoResponse, Response},
};
use dioxus_fullstack_core::RequestError;
use dioxus_html::{FormData, FormEvent};
use std::{prelude::rust_2024::Future, rc::Rc};

#[cfg(feature = "server")]
use axum::extract::multipart::{Field, MultipartError};

/// A streaming multipart form data handler.
///
/// This type makes it easy to send and receive multipart form data in a streaming fashion by directly
/// leveraging the corresponding `dioxus_html::FormData` and `axum::extract::Multipart` types.
///
/// On the client, you can create a `MultipartFormData` instance by using `.into()` on a `FormData` instance.
/// This is typically done by using the `FormEvent`'s `.data()` method.
///
/// On the server, you can extract a `MultipartFormData` instance by using it as an extractor in your handler function.
/// This gives you access to axum's `Multipart` extractor, which allows you to handle the various fields
/// and files in the multipart form data.
///
/// ## Axum Usage
///
/// Extractor that parses `multipart/form-data` requests (commonly used with file uploads).
///
/// ⚠️ Since extracting multipart form data from the request requires consuming the body, the
/// `Multipart` extractor must be *last* if there are multiple extractors in a handler.
/// See ["the order of extractors"][order-of-extractors]
///
/// [order-of-extractors]: mod@crate::extract#the-order-of-extractors
///
/// # Large Files
///
/// For security reasons, by default, `Multipart` limits the request body size to 2MB.
/// See [`DefaultBodyLimit`][default-body-limit] for how to configure this limit.
///
/// [default-body-limit]: crate::extract::DefaultBodyLimit
pub struct MultipartFormData<T = ()> {
    #[cfg(feature = "server")]
    form: Option<axum::extract::Multipart>,

    _client: Option<Rc<FormData>>,

    _phantom: std::marker::PhantomData<T>,
}

impl MultipartFormData {
    #[cfg(feature = "server")]
    pub async fn next_field(&mut self) -> Result<Option<Field<'_>>, MultipartError> {
        if let Some(form) = &mut self.form {
            form.next_field().await
        } else {
            Ok(None)
        }
    }
}

impl<S> IntoRequest for MultipartFormData<S> {
    fn into_request(
        self,
        _req: ClientRequest,
    ) -> impl Future<Output = Result<ClientResponse, RequestError>> + 'static {
        async move {
            // On the web, it's just easier to convert the form data into a blob and then send that
            // blob as the body of the request. This handles setting the correct headers, wiring
            // up file uploads as streams, and encoding the request.
            #[cfg(feature = "web")]
            if cfg!(target_arch = "wasm32") {
                let data = self._client.clone().ok_or_else(|| {
                    RequestError::Builder("Failed to get FormData from event".into())
                })?;

                fn get_form_data(data: Rc<FormData>) -> Option<wasm_bindgen::JsValue> {
                    use wasm_bindgen::JsCast;
                    let event: &web_sys::Event = data.downcast()?;
                    let target = event.target()?;
                    let form: &web_sys::HtmlFormElement = target.dyn_ref()?;
                    let data: web_sys::FormData = web_sys::FormData::new_with_form(form).ok()?;
                    Some(data.into())
                }

                let js_form_data = get_form_data(data).ok_or_else(|| {
                    RequestError::Builder("Failed to get FormData from event".into())
                })?;

                return _req.send_js_value(js_form_data).await;
            }

            // On non-web platforms, we actually need to read the values out of the FormData
            // and construct a multipart form body manually.
            #[cfg(not(target_arch = "wasm32"))]
            {
                let data = self._client.clone().ok_or_else(|| {
                    RequestError::Builder("Failed to get FormData from event".into())
                })?;

                return _req.send_multipart(&data).await;
            }

            unimplemented!("Non web wasm32 clients are not supported yet")
        }
    }
}
impl<S: Send + Sync + 'static, D> FromRequest<S> for MultipartFormData<D> {
    type Rejection = Response;

    #[doc = " Perform the extraction."]
    fn from_request(
        req: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        #[cfg(feature = "server")]
        return async move {
            let form = axum::extract::multipart::Multipart::from_request(req, state)
                .await
                .map_err(|err| err.into_response())?;

            Ok(MultipartFormData {
                form: Some(form),
                _client: None,
                _phantom: std::marker::PhantomData,
            })
        };

        #[cfg(not(feature = "server"))]
        async {
            use dioxus_fullstack_core::HttpError;

            let _ = req;
            let _ = state;
            Err(HttpError::new(
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "MultipartFormData extractor is not supported on non-server builds",
            )
            .into_response())
        }
    }
}

impl<T> From<Rc<FormData>> for MultipartFormData<T> {
    fn from(_value: Rc<FormData>) -> Self {
        MultipartFormData {
            #[cfg(feature = "server")]
            form: None,
            _client: Some(_value),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> From<FormEvent> for MultipartFormData<T> {
    fn from(event: FormEvent) -> Self {
        let data = event.data();
        MultipartFormData {
            #[cfg(feature = "server")]
            form: None,
            _client: Some(data),
            _phantom: std::marker::PhantomData,
        }
    }
}

unsafe impl Send for MultipartFormData {}
unsafe impl Sync for MultipartFormData {}
