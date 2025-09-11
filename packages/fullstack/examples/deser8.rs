use std::prelude::rust_2024::Future;

use axum::{
    extract::{FromRequest, FromRequestParts},
    handler::Handler,
    Json,
};
use dioxus_fullstack::DioxusServerState;
use http::HeaderMap;
use serde::{de::DeserializeOwned, Deserialize};

fn main() {
    fn assert_handler<T, F: MyHandler<T>>(_: F) -> T {
        todo!()
    }

    async fn handler1() {}
    async fn handler2(t: HeaderMap) {}
    async fn handler3(t: HeaderMap, body: Json<String>) {}
    async fn handler4(t: HeaderMap, a: HeaderMap) {}
    async fn handler5(t: HeaderMap, a: i32) {}
    async fn handler6(t: HeaderMap, a: Both) {}

    let a = assert_handler(handler1);
    let a = assert_handler(handler2);
    let a = assert_handler(handler3);
    let a = assert_handler(handler4);
    let a = assert_handler(handler5);
    let a = assert_handler(handler6);
}

#[derive(Deserialize)]
struct Both;
impl FromRequestParts<()> for Both {
    type Rejection = ();

    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        _state: &(),
    ) -> Result<Self, Self::Rejection> {
        Ok(Both)
    }
}

type H4 = (HeaderMap, HeaderMap, Json<String>);
type H5 = (HeaderMap, HeaderMap, HeaderMap, Json<String>);

trait MyHandler<T> {}

struct ViaParts;
struct ViaRequest;
struct ViaJson;
impl<T, Fut> MyHandler<(ViaParts,)> for T
where
    T: FnMut() -> Fut,
    Fut: Future<Output = ()>,
{
}

impl<T, Fut, A> MyHandler<(ViaRequest, A)> for T
where
    T: FnMut(A) -> Fut,
    Fut: Future<Output = ()>,
    A: FromRequest<()>,
{
}

impl<T, Fut, A> MyHandler<(ViaParts, A)> for T
where
    T: FnMut(A) -> Fut,
    Fut: Future<Output = ()>,
    A: FromRequestParts<()>,
{
}

impl<T, Fut, A, B> MyHandler<(ViaRequest, A, B)> for T
where
    T: FnMut(A, B) -> Fut,
    Fut: Future<Output = ()>,
    A: FromRequestParts<()>,
    B: FromRequest<()>,
{
}
impl<T, Fut, A, B> MyHandler<(ViaParts, A, B)> for T
where
    T: FnMut(A, B) -> Fut,
    Fut: Future<Output = ()>,
    A: FromRequestParts<()>,
    B: FromRequestParts<()>,
{
}

impl<T, Fut, A, B> MyHandler<(ViaJson, A, B)> for T
where
    T: FnMut(A, B) -> Fut,
    Fut: Future<Output = ()>,
    A: FromRequestParts<()>,
    B: DeserializeOwned,
{
}

impl<T, Fut, A, B, C> MyHandler<(ViaRequest, A, B, C)> for T
where
    T: FnMut(A, B, C) -> Fut,
    Fut: Future<Output = ()>,
    A: FromRequestParts<()>,
    B: FromRequestParts<()>,
    C: FromRequest<()>,
{
}
