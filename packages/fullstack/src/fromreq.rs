use std::prelude::rust_2024::Future;

use axum::{
    extract::{FromRequest, Request, State},
    Json,
};
use http::HeaderMap;
pub use impls::*;

use crate::{DioxusServerState, ServerFnRejection};

#[derive(Default)]
pub struct ExtractState {
    request: Request,
    state: State<DioxusServerState>,
    names: (&'static str, &'static str, &'static str),
}

unsafe impl Send for ExtractState {}
unsafe impl Sync for ExtractState {}

pub struct DeSer<T, BodyTy = (), Body = Json<BodyTy>> {
    _t: std::marker::PhantomData<T>,
    _body: std::marker::PhantomData<BodyTy>,
    _encoding: std::marker::PhantomData<Body>,
}

unsafe impl<A, B, C> Send for DeSer<A, B, C> {}
unsafe impl<A, B, C> Sync for DeSer<A, B, C> {}

fn assert_is_send(_: impl Send) {}
fn check_it() {
    // assert_is_send(DeSer::<(HeaderMap, Json<String>), Json<String>>::new());
    // assert_is_send( &&&&&&&&DeSer<(A,)>);
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

#[allow(clippy::manual_async_fn)]
#[rustfmt::skip]
mod impls {
use super::*;

    /*
    Handle the regular axum-like handlers with tiered overloading with a single trait.
    */
    pub trait ExtractRequest<S = DioxusServerState> {
        type Output;
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static;
    }

    use axum::extract::FromRequest as Freq;
    use axum::extract::FromRequestParts as Prts;
    use serde::de::DeserializeOwned as DeO_____;
    use super::DioxusServerState as Ds;

    // Zero-arg case
    impl ExtractRequest for &&&&&&&&&&DeSer<()> {
        type Output = ();
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static {
            async move { Ok(()) }
        }
    }

    // One-arg case
    impl<A> ExtractRequest for &&&&&&&&&&DeSer<(A,)> where A: Freq<Ds> {
        type Output = (A,);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A> ExtractRequest for  &&&&&&&&&DeSer<(A,)> where A: Prts<Ds> {
        type Output = (A,);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A> ExtractRequest for   &&&&&&&&DeSer<(A,)> where A: DeO_____ {
        type Output = (A,);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }


    // Two-arg case
    impl<A, B> ExtractRequest for &&&&&&&&&&DeSer<(A, B)> where A: Prts<Ds>, B: Freq<Ds> {
        type Output = (A, B);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B> ExtractRequest for  &&&&&&&&&DeSer<(A, B)> where A: Prts<Ds>, B: Prts<Ds> {
        type Output = (A, B);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B> ExtractRequest for   &&&&&&&&DeSer<(A, B)> where A: Prts<Ds>, B: DeO_____ {
        type Output = (A, B);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B> ExtractRequest for    &&&&&&&DeSer<(A, B)> where A: DeO_____, B: DeO_____ {
        type Output = (A, B);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }


    // the three-arg case
    impl<A, B, C> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C)> where A: Prts<Ds>, B: Prts<Ds>, C: Freq<Ds>, {
        type Output = (A, B, C);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds> {
        type Output = (A, B, C);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C> ExtractRequest for   &&&&&&&&DeSer<(A, B, C)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____ {
        type Output = (A, B, C);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C> ExtractRequest for   &&&&&&&DeSer<(A, B, C)> where A: Prts<Ds>, B: DeO_____, C: DeO_____ {
        type Output = (A, B, C);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C> ExtractRequest for    &&&&&&DeSer<(A, B, C)> where A: DeO_____, B: DeO_____, C: DeO_____ {
        type Output = (A, B, C);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }



    // the four-arg case
    impl<A, B, C, D> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Freq<Ds> {
        type Output = (A, B, C, D);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds> {
        type Output = (A, B, C, D);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: DeO_____ {
        type Output = (A, B, C, D);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____, D: DeO_____ {
        type Output = (A, B, C, D);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D> ExtractRequest for     &&&&&&DeSer<(A, B, C, D)> where A: Prts<Ds>, B: DeO_____, C: DeO_____, D: DeO_____ {
        type Output = (A, B, C, D);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D> ExtractRequest for      &&&&&DeSer<(A, B, C, D)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____ {
        type Output = (A, B, C, D);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }



    // the five-arg case
    impl<A, B, C, D, E> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Freq<Ds> {
        type Output = (A, B, C, D, E);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds> {
        type Output = (A, B, C, D, E);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: DeO_____ {
        type Output = (A, B, C, D, E);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: DeO_____, E: DeO_____ {
        type Output = (A, B, C, D, E);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____, D: DeO_____, E: DeO_____ {
        type Output = (A, B, C, D, E);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E)> where A: Prts<Ds>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____ {
        type Output = (A, B, C, D, E);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E> ExtractRequest for       &&&&DeSer<(A, B, C, D, E)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____ {
        type Output = (A, B, C, D, E);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }

    // the six-arg case
    impl<A, B, C, D, E, F> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Freq<Ds> {
        type Output = (A, B, C, D, E, F);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds> {
        type Output = (A, B, C, D, E, F);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: DeO_____, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: DeO_____, E: DeO_____, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F> ExtractRequest for       &&&&DeSer<(A, B, C, D, E, F)> where A: Prts<Ds>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F> ExtractRequest for        &&&DeSer<(A, B, C, D, E, F)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
        type Output = (A, B, C, D, E, F);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }



    // the seven-arg case
    impl<A, B, C, D, E, F, G> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: Freq<Ds> {
        type Output = (A, B, C, D, E, F, G);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: Prts<Ds> {
        type Output = (A, B, C, D, E, F, G);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for       &&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for        &&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G> ExtractRequest for         &&DeSer<(A, B, C, D, E, F, G)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Output = (A, B, C, D, E, F, G);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }



    // the eight-arg case
    impl<A, B, C, D, E, F, G, H> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: Prts<Ds>, H: Freq<Ds> {
        type Output = (A, B, C, D, E, F, G, H);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: Prts<Ds>, H: Prts<Ds> {
        type Output = (A, B, C, D, E, F, G, H);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: Prts<Ds>, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for       &&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for        &&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for         &&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> ExtractRequest for          &DeSer<(A, B, C, D, E, F, G, H)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Output = (A, B, C, D, E, F, G, H);
        fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
}
