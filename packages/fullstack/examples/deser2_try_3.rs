use axum::{
    body::Body,
    extract::{FromRequest, FromRequestParts, Request, State},
    Json,
};
use bytes::Bytes;
use dioxus_fullstack::{DioxusServerState, ServerFnRejection};
use futures::StreamExt;
use http::{request::Parts, HeaderMap};
use http_body_util::BodyExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// okay, we got overloads working. lets organize it and then write the macro?
#[allow(clippy::needless_borrow)]
#[tokio::main]
async fn main() {
    let state = State(DioxusServerState::default());

    let r = (&&&&&&&&&&&DeSer::<(HeaderMap, HeaderMap, Json<String>), _>::new())
        .extract(ExtractState::default())
        .await;

    let r = (&&&&&&&&&&&DeSer::<(HeaderMap, HeaderMap, String), _>::new())
        .extract(ExtractState::default())
        .await;

    let r = (&&&&&&&&&&&DeSer::<(HeaderMap, String, String), _>::new())
        .extract(ExtractState::default())
        .await;

    let r = (&&&&&&&&&&&DeSer::<(String, String, String), _>::new())
        .extract(ExtractState::default())
        .await;

    let r = (&&&&&&&&&&&DeSer::<(String, (), ()), _>::new())
        .extract(ExtractState::default())
        .await;

    let r = (&&&&&&&&&&&DeSer::<(HeaderMap, (), Json<()>), _>::new())
        .extract(ExtractState::default())
        .await;

    let r = (&&&&&&&&&&&DeSer::<(HeaderMap, HeaderMap, HeaderMap, Json<()>), _>::new())
        .extract(ExtractState::default())
        .await;

    let r = (&&&&&&&&&&&DeSer::<(HeaderMap, HeaderMap, i32, i32), _>::new())
        .extract(ExtractState::default())
        .await;

    let r = (&&&&&&&&&&&DeSer::<(HeaderMap, HeaderMap, i32, i32, i32), _>::new())
        .extract(ExtractState::default())
        .await;
}

struct DeSer<T, BodyTy = (), Body = Json<BodyTy>> {
    _t: std::marker::PhantomData<T>,
    _body: std::marker::PhantomData<BodyTy>,
    _encoding: std::marker::PhantomData<Body>,
}

impl<T, Encoding> DeSer<T, Encoding> {
    fn new() -> Self {
        DeSer {
            _t: std::marker::PhantomData,
            _body: std::marker::PhantomData,
            _encoding: std::marker::PhantomData,
        }
    }
}

#[derive(Default)]
struct ExtractState {
    request: Request,
    state: State<DioxusServerState>,
    names: (&'static str, &'static str, &'static str),
}

use impls::*;
#[rustfmt::skip]
mod impls {
use super::*;

    /*
    Handle the regular axum-like handlers with tiered overloading with a single trait.
    */
    pub trait ExtractRequest {
        type Output;
        async fn extract(&self, ctx: ExtractState) -> Self::Output;
    }

    use super::FromRequest as Freq;
    use super::FromRequestParts as Prts;
    use super::DeserializeOwned as DeO_____;

    // Zero-arg case
    impl ExtractRequest for &&&&&&&&&&DeSer<()> {
        type Output = ();
        async fn extract(&self, ctx: ExtractState) -> Self::Output {}
    }

    // One-arg case
    impl<A> ExtractRequest for &&&&&&&&&&DeSer<(A,)> where A: Freq<()> {
        type Output = (A,);
        async fn extract(&self, ctx: ExtractState)-> Self::Output { todo!() }
    }
    impl<A> ExtractRequest for &&&&&&&&&DeSer<(A,)> where A: Prts<()> {
        type Output = (A,);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A> ExtractRequest for  &&&&&&&&DeSer<(A,)> where A: DeO_____ {
        type Output = (A,);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }

    // Two-arg case
    impl<A, B> ExtractRequest for &&&&&&&&&&DeSer<(A, B)> where A: Prts<()>, B: Freq<()> {
        type Output = (A, B);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B> ExtractRequest for  &&&&&&&&&DeSer<(A, B)> where A: Prts<()>, B: Prts<()> {
        type Output = (A, B);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B> ExtractRequest for   &&&&&&&&DeSer<(A, B)> where A: Prts<()>, B: DeO_____ {
        type Output = (A, B);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B> ExtractRequest for    &&&&&&&DeSer<(A, B)> where A: DeO_____, B: DeO_____ {
        type Output = (A, B);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }


    // the three-arg case
    impl<A, B, C> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C)> where A: Prts<()>, B: Prts<()>, C: Freq<()>, {
        type Output = (A, B, C);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C)> where A: Prts<()>, B: Prts<()>, C: Prts<()> {
        type Output = (A, B, C);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C> ExtractRequest for   &&&&&&&&DeSer<(A, B, C)> where A: Prts<()>, B: Prts<()>, C: DeO_____ {
        type Output = (A, B, C);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C> ExtractRequest for   &&&&&&&DeSer<(A, B, C)> where A: Prts<()>, B: DeO_____, C: DeO_____ {
        type Output = (A, B, C);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C> ExtractRequest for    &&&&&&DeSer<(A, B, C)> where A: DeO_____, B: DeO_____, C: DeO_____ {
        type Output = (A, B, C);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }

    // the four-arg case
    impl<A, B, C, D> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Freq<()> {
        type Output = (A, B, C, D);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()> {
        type Output = (A, B, C, D);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: DeO_____ {
        type Output = (A, B, C, D);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D)> where A: Prts<()>, B: Prts<()>, C: DeO_____, D: DeO_____ {
        type Output = (A, B, C, D);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D> ExtractRequest for     &&&&&&DeSer<(A, B, C, D)> where A: Prts<()>, B: DeO_____, C: DeO_____, D: DeO_____ {
        type Output = (A, B, C, D);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D> ExtractRequest for      &&&&&DeSer<(A, B, C, D)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____ {
        type Output = (A, B, C, D);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }

    // the five-arg case
    impl<A, B, C, D, E> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Freq<()> {
        type Output = (A, B, C, D, E);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()> {
        type Output = (A, B, C, D, E);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: DeO_____ {
        type Output = (A, B, C, D, E);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: DeO_____, E: DeO_____ {
        type Output = (A, B, C, D, E);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: Prts<()>, C: DeO_____, D: DeO_____, E: DeO_____ {
        type Output = (A, B, C, D, E);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____ {
        type Output = (A, B, C, D, E);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E> ExtractRequest for       &&&&DeSer<(A, B, C, D, E)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____ {
        type Output = (A, B, C, D, E);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }

    // the six-arg case
    impl<A, B, C, D, E, F> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Freq<()> {
        type Output = (A, B, C, D, E, F);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()> {
        type Output = (A, B, C, D, E, F);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: DeO_____, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: DeO_____, E: DeO_____, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<()>, B: Prts<()>, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F> ExtractRequest for       &&&&DeSer<(A, B, C, D, E, F)> where A: Prts<()>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F> ExtractRequest for        &&&DeSer<(A, B, C, D, E, F)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }

    // the seven-arg case
    impl<A, B, C, D, E, F, G> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: Freq<()> {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: Prts<()> {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for       &&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<()>, B: Prts<()>, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for        &&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<()>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for         &&DeSer<(A, B, C, D, E, F, G)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }

    // the eight-arg case
    impl<A, B, C, D, E, F, G, H> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: Prts<()>, H: Freq<()> {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: Prts<()>, H: Prts<()> {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: Prts<()>, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for       &&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for        &&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for         &&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for          &DeSer<(A, B, C, D, E, F, G, H)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Self::Output { todo!() }
    }
}
