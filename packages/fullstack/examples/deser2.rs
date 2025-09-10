use axum::{
    extract::{FromRequest, FromRequestParts, Request},
    Json,
};
use dioxus_fullstack::DioxusServerState;
use http::HeaderMap;
use serde::{de::DeserializeOwned, Deserialize};

fn main() {
    let de = De::<(HeaderMap, HeaderMap, HeaderMap)>::new();
    let r = (&&&&de).extract();

    let de = De::<(HeaderMap, HeaderMap, String)>::new();
    let r = (&&&&de).extract();

    let de = De::<(HeaderMap, String, String)>::new();
    let r = (&&&&de).extract();

    let de = De::<(String, String, String)>::new();
    let r = (&&&&de).extract();

    let de = De::<(String, (), ())>::new();
    let r = (&&&&de).extract();

    // let de = De::<(HeaderMap, Json<()>, ())>::new();
    // let r = (&&&&de).extract();
}

struct De<T>(std::marker::PhantomData<T>);
impl<T> De<T> {
    fn new() -> Self {
        De(std::marker::PhantomData)
    }
}

trait ExtractP0<O> {
    fn extract(&self) -> O;
}

impl<A, B, C> ExtractP0<(A, B, C)> for &&&&De<(A, B, C)>
where
    A: FromRequestParts<DioxusServerState>,
    B: FromRequestParts<DioxusServerState>,
    C: FromRequestParts<DioxusServerState>,
{
    fn extract(&self) -> (A, B, C) {
        todo!()
    }
}
trait ExtractP1<O> {
    fn extract(&self) -> O;
}

impl<A, B, C> ExtractP1<(A, B, C)> for &&De<(A, B, C)>
where
    A: FromRequestParts<DioxusServerState>,
    B: FromRequestParts<DioxusServerState>,
    C: DeserializeOwned,
{
    fn extract(&self) -> (A, B, C) {
        todo!()
    }
}

trait ExtractP2<O> {
    fn extract(&self) -> O;
}

impl<A, B, C> ExtractP2<(A, B, C)> for &&De<(A, B, C)>
where
    A: FromRequestParts<DioxusServerState>,
    B: DeserializeOwned,
    C: DeserializeOwned,
{
    fn extract(&self) -> (A, B, C) {
        todo!()
    }
}

trait ExtractP3<O> {
    fn extract(&self) -> O;
}
impl<A, B, C> ExtractP3<(A, B, C)> for &De<(A, B, C)>
where
    A: DeserializeOwned,
    B: DeserializeOwned,
    C: DeserializeOwned,
{
    fn extract(&self) -> (A, B, C) {
        todo!()
    }
}
