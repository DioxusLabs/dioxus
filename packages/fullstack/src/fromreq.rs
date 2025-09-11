use std::prelude::rust_2024::Future;

use axum::{
    extract::{FromRequest, Request, State},
    Json,
};
pub use impls::*;

use crate::{DioxusServerState, ServerFnRejection};

#[derive(Default)]
pub struct ExtractState {
    request: Request,
    state: State<DioxusServerState>,
    names: (&'static str, &'static str, &'static str),
}

pub struct DeSer<T, BodyTy = (), Body = Json<BodyTy>> {
    _t: std::marker::PhantomData<T>,
    _body: std::marker::PhantomData<BodyTy>,
    _encoding: std::marker::PhantomData<Body>,
}

impl<T, Encoding> DeSer<T, Encoding> {
    pub fn new() -> Self {
        DeSer {
            _t: std::marker::PhantomData,
            _body: std::marker::PhantomData,
            _encoding: std::marker::PhantomData,
        }
    }
}

/// An on-the-fly struct for deserializing a variable number of types as a map
pub struct DeTys<T> {
    names: &'static [&'static str],
    _phantom: std::marker::PhantomData<T>,
}

impl<T, S> FromRequest<S> for DeTys<T> {
    #[doc = " If the extractor fails it\'ll use this \"rejection\" type. A rejection is"]
    #[doc = " a kind of error that can be converted into a response."]
    type Rejection = ServerFnRejection;

    #[doc = " Perform the extraction."]
    fn from_request(
        req: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move { todo!() }
    }
}

#[rustfmt::skip]
mod impls {
use super::*;

    /*
    Handle the regular axum-like handlers with tiered overloading with a single trait.
    */
    pub trait ExtractRequest {
        type Output;
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static;
    }

    use axum::extract::FromRequest as Freq;
    use axum::extract::FromRequestParts as Prts;
    use serde::de::DeserializeOwned as DeO_____;

    // Zero-arg case
    impl ExtractRequest for &&&&&&&&&&DeSer<()> {
        type Output = ();
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static {
            async move { Ok(()) }
        }
    }

    // One-arg case
    impl<A> ExtractRequest for &&&&&&&&&&DeSer<(A,)> where A: Freq<()> {
        type Output = (A,);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A> ExtractRequest for  &&&&&&&&&DeSer<(A,)> where A: Prts<()> {
        type Output = (A,);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A> ExtractRequest for   &&&&&&&&DeSer<(A,)> where A: DeO_____ {
        type Output = (A,);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }



    // Two-arg case
    impl<A, B> ExtractRequest for &&&&&&&&&&DeSer<(A, B)> where A: Prts<()>, B: Freq<()> {
        type Output = (A, B);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B> ExtractRequest for  &&&&&&&&&DeSer<(A, B)> where A: Prts<()>, B: Prts<()> {
        type Output = (A, B);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B> ExtractRequest for   &&&&&&&&DeSer<(A, B)> where A: Prts<()>, B: DeO_____ {
        type Output = (A, B);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B> ExtractRequest for    &&&&&&&DeSer<(A, B)> where A: DeO_____, B: DeO_____ {
        type Output = (A, B);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }


    // the three-arg case
    impl<A, B, C> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C)> where A: Prts<()>, B: Prts<()>, C: Freq<()>, {
        type Output = (A, B, C);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C)> where A: Prts<()>, B: Prts<()>, C: Prts<()> {
        type Output = (A, B, C);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C> ExtractRequest for   &&&&&&&&DeSer<(A, B, C)> where A: Prts<()>, B: Prts<()>, C: DeO_____ {
        type Output = (A, B, C);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C> ExtractRequest for   &&&&&&&DeSer<(A, B, C)> where A: Prts<()>, B: DeO_____, C: DeO_____ {
        type Output = (A, B, C);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C> ExtractRequest for    &&&&&&DeSer<(A, B, C)> where A: DeO_____, B: DeO_____, C: DeO_____ {
        type Output = (A, B, C);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }



    // the four-arg case
    impl<A, B, C, D> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Freq<()> {
        type Output = (A, B, C, D);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()> {
        type Output = (A, B, C, D);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: DeO_____ {
        type Output = (A, B, C, D);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D)> where A: Prts<()>, B: Prts<()>, C: DeO_____, D: DeO_____ {
        type Output = (A, B, C, D);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D> ExtractRequest for     &&&&&&DeSer<(A, B, C, D)> where A: Prts<()>, B: DeO_____, C: DeO_____, D: DeO_____ {
        type Output = (A, B, C, D);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D> ExtractRequest for      &&&&&DeSer<(A, B, C, D)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____ {
        type Output = (A, B, C, D);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }



    // the five-arg case
    impl<A, B, C, D, E> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Freq<()> {
        type Output = (A, B, C, D, E);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()> {
        type Output = (A, B, C, D, E);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: DeO_____ {
        type Output = (A, B, C, D, E);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: DeO_____, E: DeO_____ {
        type Output = (A, B, C, D, E);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: Prts<()>, C: DeO_____, D: DeO_____, E: DeO_____ {
        type Output = (A, B, C, D, E);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E)> where A: Prts<()>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____ {
        type Output = (A, B, C, D, E);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E> ExtractRequest for       &&&&DeSer<(A, B, C, D, E)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____ {
        type Output = (A, B, C, D, E);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }

    // the six-arg case
    impl<A, B, C, D, E, F> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Freq<()> {
        type Output = (A, B, C, D, E, F);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()> {
        type Output = (A, B, C, D, E, F);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: DeO_____, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: DeO_____, E: DeO_____, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<()>, B: Prts<()>, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F> ExtractRequest for       &&&&DeSer<(A, B, C, D, E, F)> where A: Prts<()>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F> ExtractRequest for        &&&DeSer<(A, B, C, D, E, F)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }



    // the seven-arg case
    impl<A, B, C, D, E, F, G> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: Freq<()> {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: Prts<()> {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for       &&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<()>, B: Prts<()>, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for        &&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<()>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for         &&DeSer<(A, B, C, D, E, F, G)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }



    // the eight-arg case
    impl<A, B, C, D, E, F, G, H> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: Prts<()>, H: Freq<()> {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: Prts<()>, H: Prts<()> {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: Prts<()>, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: Prts<()>, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: Prts<()>, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: Prts<()>, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for       &&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: Prts<()>, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for        &&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: Prts<()>, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for         &&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<()>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for          &DeSer<(A, B, C, D, E, F, G, H)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        async fn extract(&self, ctx: ExtractState) -> Result<Self::Output, ServerFnRejection> { todo!() }
    }
}
