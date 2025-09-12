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
    impl ExtractRequest for &&&&&&DeSer<()> {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A> ExtractRequest for &&&&&&DeSer<(A,)> where A: FromRequest<()> {
        async fn extract(&self, ctx: ExtractState) {}
    }

    impl<A> ExtractRequest for &&&&&DeSer<(A,)> where A: FromRequestParts<()> {
        async fn extract(&self, ctx: ExtractState) {}
    }

    impl<A, B> ExtractRequest for &&&&&&DeSer<(A, B)> where A: FromRequestParts<()>, B: FromRequest<()> {
        async fn extract(&self, ctx: ExtractState) {}
    }

    impl<A, B> ExtractRequest for &&&&&DeSer<(A, B)> where A: FromRequestParts<()>, B: FromRequestParts<()> {
        async fn extract(&self, ctx: ExtractState) {}
    }

    impl<A, B, C> ExtractRequest for &&&&&&DeSer<(A, B, C)> where A: FromRequestParts<()>, B: FromRequestParts<()>, C: FromRequest<()>, {
        async fn extract(&self, ctx: ExtractState) {}
    }

    impl<A, B, C> ExtractRequest for &&&&&DeSer<(A, B, C)> where A: FromRequestParts<()>, B: FromRequestParts<()>, C: FromRequestParts<()>, {
        async fn extract(&self, ctx: ExtractState) {}
    }

    /*
    Now handle the deserialie cases. They are tiered below the standard axum handlers.
    */

    // the one-arg case
    impl<A> ExtractRequest for &&&&DeSer<(A,)> where A: DeserializeOwned {
        async fn extract(&self, ctx: ExtractState) {}
    }

    // the two-arg case
    impl<A, B> ExtractRequest for &&&&DeSer<(A, B)> where A: FromRequestParts<()>, B: DeserializeOwned {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B> ExtractRequest for &&&DeSer<(A, B)> where A: DeserializeOwned, B: DeserializeOwned {
        async fn extract(&self, ctx: ExtractState) {}
    }

    // the three-arg case
    impl<A, B, C> ExtractRequest for &&&&DeSer<(A, B, C)> where A: FromRequestParts<()>, B: FromRequestParts<()>, C: DeserializeOwned {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C> ExtractRequest for &&&DeSer<(A, B, C)> where A: FromRequestParts<()>, B: DeserializeOwned, C: DeserializeOwned {
        async fn extract(&self, ctx: ExtractState) {}
    }
    impl<A, B, C> ExtractRequest for &&DeSer<(A, B, C)> where A: DeserializeOwned, B: DeserializeOwned, C: DeserializeOwned {
        async fn extract(&self, ctx: ExtractState) {}
    }
}

// #[rustfmt::skip] trait ExtractP0<O> {
//     async fn extract( &self, request: Request, state: &State<DioxusServerState>, names: (&'static str, &'static str, &'static str), ) -> Result<O, ServerFnRejection>;
// }

// #[rustfmt::skip] impl<A, B, C> ExtractP0<(A, B, C)> for &&&&&DeSer<(A, B, C), ()> where
//     A: FromRequestParts<DioxusServerState>, B: FromRequestParts<DioxusServerState>, C: FromRequestParts<DioxusServerState>,
// {
//     async fn extract(&self, request: Request, state: &State<DioxusServerState>, names: (&'static str, &'static str, &'static str), ) -> Result<(A, B, C), ServerFnRejection> {
//         let (mut parts, _) = request.into_parts();
//         Ok((
//             A::from_request_parts(&mut parts, state).await.map_err(|_| ServerFnRejection {})?,
//             B::from_request_parts(&mut parts, state).await.map_err(|_| ServerFnRejection {})?,
//             C::from_request_parts(&mut parts, state).await.map_err(|_| ServerFnRejection {})?,
//         ))
//     }
// }

// trait Unit {}
// impl Unit for () {}

// trait ExtractPB0<O> {
//     async fn extract(
//         &self,
//         request: Request,
//         state: &State<DioxusServerState>,
//         names: (&'static str, &'static str, &'static str),
//     ) -> Result<O, ServerFnRejection>;
// }

// impl<A, B, C> ExtractPB0<(A, B, C)> for &&&&DeSer<(A, B, C), ()>
// where
//     A: FromRequestParts<DioxusServerState>,
//     B: FromRequest<DioxusServerState>,
//     C: Unit,
// {
//     async fn extract(
//         &self,
//         request: Request,
//         state: &State<DioxusServerState>,
//         names: (&'static str, &'static str, &'static str),
//     ) -> Result<(A, B, C), ServerFnRejection> {
//         todo!()
//     }
// }

// trait ExtractP1<O> {
//     async fn extract(
//         &self,
//         request: Request,
//         state: &State<DioxusServerState>,
//         names: (&'static str, &'static str, &'static str),
//     ) -> Result<O, ServerFnRejection>;
// }

// impl<A, B, C> ExtractP1<(A, B, C)> for &&&DeSer<(A, B, C), (C,)>
// where
//     A: FromRequestParts<DioxusServerState>,
//     B: FromRequestParts<DioxusServerState>,
//     C: DeserializeOwned,
// {
//     async fn extract(
//         &self,
//         request: Request,
//         state: &State<DioxusServerState>,
//         names: (&'static str, &'static str, &'static str),
//     ) -> Result<(A, B, C), ServerFnRejection> {
//         let (mut parts, body) = request.into_parts();
//         let a = A::from_request_parts(&mut parts, state)
//             .await
//             .map_err(|_| ServerFnRejection {})?;

//         let b = B::from_request_parts(&mut parts, state)
//             .await
//             .map_err(|_| ServerFnRejection {})?;

//         let bytes = body.collect().await.unwrap().to_bytes();
//         let (_, _, c) = struct_to_named_tuple::<(), (), C>(bytes, ("", "", names.2));

//         Ok((a, b, c))
//     }
// }

// trait ExtractP2<O> {
//     async fn extract(
//         &self,
//         request: Request,
//         state: &State<DioxusServerState>,
//         names: (&'static str, &'static str, &'static str),
//     ) -> O;
// }

// impl<A, B, C> ExtractP2<(A, B, C)> for &&DeSer<(A, B, C), (B, C)>
// where
//     A: FromRequestParts<DioxusServerState>,
//     B: DeserializeOwned,
//     C: DeserializeOwned,
// {
//     async fn extract(
//         &self,
//         request: Request,
//         state: &State<DioxusServerState>,
//         names: (&'static str, &'static str, &'static str),
//     ) -> (A, B, C) {
//         todo!()
//     }
// }

// trait ExtractP3<O> {
//     async fn extract(
//         &self,
//         request: Request,
//         state: &State<DioxusServerState>,
//         names: (&'static str, &'static str, &'static str),
//     ) -> O;
// }
// impl<A, B, C> ExtractP3<(A, B, C)> for &DeSer<(A, B, C), (A, B, C)>
// where
//     A: DeserializeOwned,
//     B: DeserializeOwned,
//     C: DeserializeOwned,
// {
//     async fn extract(
//         &self,
//         request: Request,
//         state: &State<DioxusServerState>,
//         names: (&'static str, &'static str, &'static str),
//     ) -> (A, B, C) {
//         todo!()
//     }
// }

// /// Todo: make this more efficient with a custom visitor instead of using serde_json intermediate
// fn struct_to_named_tuple<A, B, C>(
//     body: Bytes,
//     names: (&'static str, &'static str, &'static str),
// ) -> (A, B, C) {
//     todo!()
// }
