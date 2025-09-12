use std::{marker::PhantomData, prelude::rust_2024::Future};

use axum::{
    extract::{FromRequest, FromRequestParts},
    response::IntoResponse,
};
use dioxus_fullstack::DioxusServerState;
use http::request::Parts;
use serde::{de::DeserializeOwned, Deserialize};

fn main() {
    fn assert<T: CustomResponse<M>, M>() {}
    let r = assert::<Wrap<ThingBoth>, _>();
    let r = assert::<Wrap<ThingBoth>, _>();
}

#[derive(Deserialize)]
struct ThingBoth;
impl FromRequestParts<DioxusServerState> for ThingBoth {
    type Rejection = ();

    #[doc = " Perform the extraction."]
    fn from_request_parts(
        parts: &mut Parts,
        state: &DioxusServerState,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move { todo!() }
    }
}

trait CustomResponse<M> {}

struct Wrap<T>(PhantomData<T>);
struct CombinedMarker;

impl<T> CustomResponse<CombinedMarker> for Wrap<T> where
    T: FromRequestParts<DioxusServerState> + DeserializeOwned
{
}

struct ViaPartsMarker;
impl<T> CustomResponse<ViaPartsMarker> for Wrap<T> where T: FromRequestParts<DioxusServerState> {}

struct ViaDeserializeMarker;
impl<T> CustomResponse<ViaDeserializeMarker> for Wrap<T> where T: DeserializeOwned {}
