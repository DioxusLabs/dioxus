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

    // let r = (&&&&&&&&&&&DeSer::<(HeaderMap, HeaderMap, Json<String>), _>::new())
    //     .extract(ExtractState::default())
    //     .await;

    // let r = (&&&&&&&&&&&DeSer::<(HeaderMap, HeaderMap, String), _>::new())
    //     .extract(ExtractState::default())
    //     .await;

    // let r = (&&&&&&&&&&&DeSer::<(HeaderMap, String, String), _>::new())
    //     .extract(ExtractState::default())
    //     .await;

    // let r = (&&&&&&&&&&&DeSer::<(String, String, String), _>::new())
    //     .extract(ExtractState::default())
    //     .await;

    // let r = (&&&&&&&&&&&DeSer::<(String, (), ()), _>::new())
    //     .extract(ExtractState::default())
    //     .await;

    // let r = (&&&&&&&&&&&DeSer::<(HeaderMap, (), Json<()>), _>::new())
    //     .extract(ExtractState::default())
    //     .await;

    // let r = (&&&&&&&&&&&DeSer::<(HeaderMap, HeaderMap, HeaderMap, Json<()>), _>::new())
    //     .extract(ExtractState::default())
    //     .await;

    // let r = (&&&&&&&&&&&DeSer::<(HeaderMap, HeaderMap, i32, i32), _>::new())
    //     .extract(ExtractState::default())
    //     .await;

    // let r = (&&&&&&&&&&&DeSer::<(HeaderMap, HeaderMap, i32, i32, i32), _>::new())
    //     .extract(ExtractState::default())
    //     .await;
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
    trait Unit{ }
    impl Unit for () {}

    use super::FromRequest as Freq;
    use super::FromRequestParts as Prts;
    use super::DeserializeOwned as DeO_____;

        // Zero-arg case
        impl ExtractRequest for &&&&&&&&&&DeSer<()> {
            async fn extract(&self, ctx: ExtractState) {}
        }

        // the eight-arg case
        impl<A, B, C, D, E, F, G, H> ExtractRequest for &&&&&&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: Prts<()>, H: Freq<()> {
            async fn extract(&self, ctx: ExtractState) {}
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: Freq<()>, H: Prts<()> {
            async fn extract(&self, ctx: ExtractState) {}
        }

        impl<A, B, C, D, E, F, G, H> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: Prts<()>, H: Prts<()> {
            async fn extract(&self, ctx: ExtractState) {}
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: Prts<()>, H: DeO_____ {
            async fn extract(&self, ctx: ExtractState) {}
        }

        impl<A, B, C, D, E, F, G, H> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: DeO_____, H: DeO_____ {
            async fn extract(&self, ctx: ExtractState) {}
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: DeO_____, G: DeO_____, H: DeO_____ {
            async fn extract(&self, ctx: ExtractState) {}
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
            async fn extract(&self, ctx: ExtractState) {}
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for       &&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
            async fn extract(&self, ctx: ExtractState) {}
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for        &&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
            async fn extract(&self, ctx: ExtractState) {}
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for         &&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
            async fn extract(&self, ctx: ExtractState) {}
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for          &DeSer<(A, B, C, D, E, F, G, H)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
            async fn extract(&self, ctx: ExtractState) {}
        }
}
