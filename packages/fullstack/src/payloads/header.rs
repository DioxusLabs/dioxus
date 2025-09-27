use std::{marker::PhantomData, prelude::rust_2024::Future};

use axum::response::{IntoResponse, IntoResponseParts, ResponseParts};
use dioxus_fullstack_core::{RequestError, ServerFnError};
use headers::Header;
use http::{header::InvalidHeaderValue, HeaderValue};

use crate::{ClientRequest, ClientResponse, FromResponse, FromResponseParts, IntoRequest};

pub use headers::Cookie;
pub use headers::SetCookie;

#[derive(Clone, Debug)]
pub struct SetHeader<Data> {
    data: Option<Data>,
}

impl<T: Header> SetHeader<T> {
    pub fn new(
        value: impl TryInto<HeaderValue, Error = InvalidHeaderValue>,
    ) -> Result<Self, headers::Error> {
        let values = value.try_into().map_err(|_| headers::Error::invalid())?;

        let res = T::decode(&mut std::iter::once(&values))?;

        Ok(Self { data: Some(res) })
    }

    // pub fn new<I>(value: impl IntoIterator<Item = I>) -> Result<Self, headers::Error>
    //     where
    //         I: TryInto<HeaderValue, Error = InvalidHeaderValue>,
    //     {
    //         let values: Vec<HeaderValue> = value
    //             .into_iter()
    //             .map(|v| v.try_into())
    //             .collect::<Result<Vec<_>, _>>()
    //             .map_err(|_| headers::Error::invalid())?;
}

impl<T: Header> IntoResponseParts for SetHeader<T> {
    type Error = ();

    fn into_response_parts(self, res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        todo!()
    }
}

impl<T: Header> FromResponseParts for SetHeader<T> {
    fn from_response_parts(parts: &mut axum::http::response::Parts) -> Result<Self, ServerFnError> {
        let Some(header) = parts.headers.remove(T::name()) else {
            return Ok(SetHeader { data: None });
        };

        let data = T::decode(&mut std::iter::once(&header))
            .map_err(|_| ServerFnError::Deserialization("Failed to decode header".into()))?;

        Ok(SetHeader { data: Some(data) })
    }
}

impl<T: Header> IntoResponse for SetHeader<T> {
    fn into_response(self) -> axum::response::Response {
        let mut values = vec![];
        self.data.unwrap().encode(&mut values);

        let mut response = axum::response::Response::builder();

        for value in values {
            response = response.header(T::name(), value);
        }

        response.body(axum_core::body::Body::empty()).unwrap()
    }
}
