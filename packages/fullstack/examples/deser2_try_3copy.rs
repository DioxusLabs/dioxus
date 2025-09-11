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

    let r = (&&&&&&&DeSer::<(HeaderMap, HeaderMap, Json<String>), _>::new())
        .extract(ExtractState::default())
        .await;

    let r = (&&&&&&&DeSer::<(HeaderMap, HeaderMap, String), _>::new())
        .extract(ExtractState::default())
        .await;

    let r = (&&&&&&&DeSer::<(HeaderMap, String, String), _>::new())
        .extract(ExtractState::default())
        .await;

    let r = (&&&&&&DeSer::<(String, String, String), _>::new())
        .extract(ExtractState::default())
        .await;

    let r = (&&&&&&DeSer::<(String, (), ()), _>::new())
        .extract(ExtractState::default())
        .await;

    let r = (&&&&&&DeSer::<(HeaderMap, (), Json<()>), _>::new())
        .extract(ExtractState::default())
        .await;

    let r = (&&&&&&DeSer::<(HeaderMap, HeaderMap, HeaderMap, Json<()>), _>::new())
        .extract(ExtractState::default())
        .await;

    let r = (&&&&&&DeSer::<(HeaderMap, HeaderMap, i32, i32), _>::new())
        .extract(ExtractState::default())
        .await;

    let r = (&&&&&&DeSer::<(HeaderMap, HeaderMap, i32, i32, i32), _>::new())
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
        async fn extract(&self, ctx: ExtractState);
    }

    use super::FromRequest as Freq;
    use super::FromRequestParts as Prts;
    use super::DeserializeOwned as DeO_____;

    // Zero-arg case
    impl ExtractRequest for &&&&&&DeSer<()> {
        async fn extract(&self, ctx: ExtractState) {}
    }

    // One-arg case
    impl<A> ExtractRequest for &&&&&&DeSer<(A,)> where A: Freq<()> {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A> ExtractRequest for &&&&&DeSer<(A,)> where A: Prts<()> {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A> ExtractRequest for  &&&&DeSer<(A,)> where A: DeO_____ {
        async fn extract(&self, ctx: ExtractState) {}
    }

    // Two-arg case
    impl<A, B> ExtractRequest for &&&&&&DeSer<(A, B)> where A: Prts<()>, B: Freq<()> {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B> ExtractRequest for  &&&&&DeSer<(A, B)> where A: Prts<()>, B: Prts<()> {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B> ExtractRequest for   &&&&DeSer<(A, B)> where A: Prts<()>, B: DeO_____ {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B> ExtractRequest for    &&&DeSer<(A, B)> where A: DeO_____, B: DeO_____ {
        async fn extract(&self, ctx: ExtractState) {}
    }


    // the three-arg case
    impl<A, B, C> ExtractRequest for &&&&&&DeSer<(A, B, C)> where A: Prts<()>, B: Prts<()>, C: Freq<()>, {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C> ExtractRequest for  &&&&&DeSer<(A, B, C)> where A: Prts<()>, B: Prts<()>, C: Prts<()> {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C> ExtractRequest for   &&&&DeSer<(A, B, C)> where A: Prts<()>, B: Prts<()>, C: DeO_____ {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C> ExtractRequest for   &&&DeSer<(A, B, C)> where A: Prts<()>, B: DeO_____, C: DeO_____ {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C> ExtractRequest for    &&DeSer<(A, B, C)> where A: DeO_____, B: DeO_____, C: DeO_____ {
        async fn extract(&self, ctx: ExtractState) {}
    }

    // the four-arg case
    impl<A, B, C, D> ExtractRequest for &&&&&&DeSer<(A, B, C, D)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Freq<()> {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C, D> ExtractRequest for  &&&&&DeSer<(A, B, C, D)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()> {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C, D> ExtractRequest for   &&&&DeSer<(A, B, C, D)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: DeO_____ {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C, D> ExtractRequest for    &&&DeSer<(A, B, C, D)> where A: Prts<()>, B: Prts<()>, C: DeO_____, D: DeO_____ {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C, D> ExtractRequest for     &&DeSer<(A, B, C, D)> where A: Prts<()>, B: DeO_____, C: DeO_____, D: DeO_____ {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C, D> ExtractRequest for      &DeSer<(A, B, C, D)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____ {
        async fn extract(&self, ctx: ExtractState) {}
    }

    // the five-arg case
    impl<A, B, C, D, E> ExtractRequest for &&&&&&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Freq<()> {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C, D, E> ExtractRequest for  &&&&&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()> {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C, D, E> ExtractRequest for   &&&&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: DeO_____ {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C, D, E> ExtractRequest for    &&&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: DeO_____, E: DeO_____ {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C, D, E> ExtractRequest for     &&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: Prts<()>, C: DeO_____, D: DeO_____, E: DeO_____ {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C, D, E> ExtractRequest for      &DeSer<(A, B, C, D, E)> where A: Prts<()>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____ {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C, D, E> ExtractRequest for       DeSer<(A, B, C, D, E)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____ {
        async fn extract(&self, ctx: ExtractState) {}
    }
}
