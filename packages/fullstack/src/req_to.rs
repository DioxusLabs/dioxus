use std::prelude::rust_2024::Future;

use axum::{
    extract::{FromRequest, Request, State},
    Json,
};
use http::HeaderMap;
pub use impls::*;

use crate::{DioxusServerState, ServerFnRejection};

#[derive(Default)]
pub struct EncodeState {
    request: Request,
    state: State<DioxusServerState>,
    names: (&'static str, &'static str, &'static str),
}

unsafe impl Send for EncodeState {}
unsafe impl Sync for EncodeState {}

pub struct ReqSer<T, BodyTy = (), Body = Json<BodyTy>> {
    _t: std::marker::PhantomData<T>,
    _body: std::marker::PhantomData<BodyTy>,
    _encoding: std::marker::PhantomData<Body>,
}

unsafe impl<A, B, C> Send for ReqSer<A, B, C> {}
unsafe impl<A, B, C> Sync for ReqSer<A, B, C> {}

fn assert_is_send(_: impl Send) {}
fn check_it() {
    // assert_is_send(DeSer::<(HeaderMap, Json<String>), Json<String>>::new());
    // assert_is_send( &&&&&&&&DeSer<(A,)>);
}

impl<T, Encoding> ReqSer<T, Encoding> {
    pub fn new() -> Self {
        ReqSer {
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
    pub trait EncodeRequest {
        type Input;
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static;
    }

    use axum::response::IntoResponse as Freq;
    use axum::response::IntoResponseParts as Prts;
    use serde::ser::Serialize as DeO_____;
    use super::DioxusServerState as Ds;

    // Zero-arg case
    impl EncodeRequest for &&&&&&&&&&ReqSer<()> {
        type Input = ();
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static {
            async move { todo!() }
        }
    }

    // One-arg case
    impl<A> EncodeRequest for &&&&&&&&&&ReqSer<(A,)> where A: Freq {
        type Input = (A,);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A> EncodeRequest for  &&&&&&&&&ReqSer<(A,)> where A: Prts {
        type Input = (A,);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A> EncodeRequest for   &&&&&&&&ReqSer<(A,)> where A: DeO_____ {
        type Input = (A,);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }


    // Two-arg case
    impl<A, B> EncodeRequest for &&&&&&&&&&ReqSer<(A, B)> where A: Prts, B: Freq {
        type Input = (A, B);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B> EncodeRequest for  &&&&&&&&&ReqSer<(A, B)> where A: Prts, B: Prts {
        type Input = (A, B);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B> EncodeRequest for   &&&&&&&&ReqSer<(A, B)> where A: Prts {
        type Input = (A, B);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B> EncodeRequest for    &&&&&&&ReqSer<(A, B)>  {
        type Input = (A, B);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }


    // the three-arg case
    impl<A, B, C> EncodeRequest for &&&&&&&&&&ReqSer<(A, B, C)> where A: Prts, B: Prts, C: Freq, {
        type Input = (A, B, C);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C> EncodeRequest for  &&&&&&&&&ReqSer<(A, B, C)> where A: Prts, B: Prts, C: Prts {
        type Input = (A, B, C);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C> EncodeRequest for   &&&&&&&&ReqSer<(A, B, C)> where A: Prts, B: Prts {
        type Input = (A, B, C);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C> EncodeRequest for   &&&&&&&ReqSer<(A, B, C)> where A: Prts {
        type Input = (A, B, C);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C> EncodeRequest for    &&&&&&ReqSer<(A, B, C)>  {
        type Input = (A, B, C);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }



    // the four-arg case
    impl<A, B, C, D> EncodeRequest for &&&&&&&&&&ReqSer<(A, B, C, D)> where A: Prts, B: Prts, C: Prts, D: Freq {
        type Input = (A, B, C, D);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D> EncodeRequest for  &&&&&&&&&ReqSer<(A, B, C, D)> where A: Prts, B: Prts, C: Prts, D: Prts {
        type Input = (A, B, C, D);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D> EncodeRequest for   &&&&&&&&ReqSer<(A, B, C, D)> where A: Prts, B: Prts, C: Prts, D: DeO_____ {
        type Input = (A, B, C, D);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D> EncodeRequest for    &&&&&&&ReqSer<(A, B, C, D)> where A: Prts, B: Prts, C: DeO_____, D: DeO_____ {
        type Input = (A, B, C, D);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D> EncodeRequest for     &&&&&&ReqSer<(A, B, C, D)> where A: Prts, B: DeO_____, C: DeO_____, D: DeO_____ {
        type Input = (A, B, C, D);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D> EncodeRequest for      &&&&&ReqSer<(A, B, C, D)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____ {
        type Input = (A, B, C, D);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }

    // the five-arg case
    impl<A, B, C, D, E> EncodeRequest for &&&&&&&&&&ReqSer<(A, B, C, D, E)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Freq {
        type Input = (A, B, C, D, E);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E> EncodeRequest for  &&&&&&&&&ReqSer<(A, B, C, D, E)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts {
        type Input = (A, B, C, D, E);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E> EncodeRequest for   &&&&&&&&ReqSer<(A, B, C, D, E)> where A: Prts, B: Prts, C: Prts, D: Prts, E: DeO_____ {
        type Input = (A, B, C, D, E);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E> EncodeRequest for    &&&&&&&ReqSer<(A, B, C, D, E)> where A: Prts, B: Prts, C: Prts, D: DeO_____, E: DeO_____ {
        type Input = (A, B, C, D, E);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E> EncodeRequest for     &&&&&&ReqSer<(A, B, C, D, E)> where A: Prts, B: Prts, C: DeO_____, D: DeO_____, E: DeO_____ {
        type Input = (A, B, C, D, E);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E> EncodeRequest for      &&&&&ReqSer<(A, B, C, D, E)> where A: Prts, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____ {
        type Input = (A, B, C, D, E);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E> EncodeRequest for       &&&&ReqSer<(A, B, C, D, E)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____ {
        type Input = (A, B, C, D, E);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }

    // the six-arg case
    impl<A, B, C, D, E, F> EncodeRequest for &&&&&&&&&&ReqSer<(A, B, C, D, E, F)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Freq {
        type Input = (A, B, C, D, E, F);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F> EncodeRequest for  &&&&&&&&&ReqSer<(A, B, C, D, E, F)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Prts {
        type Input = (A, B, C, D, E, F);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F> EncodeRequest for   &&&&&&&&ReqSer<(A, B, C, D, E, F)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: DeO_____ {
        type Input = (A, B, C, D, E, F);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F> EncodeRequest for    &&&&&&&ReqSer<(A, B, C, D, E, F)> where A: Prts, B: Prts, C: Prts, D: Prts, E: DeO_____, F: DeO_____ {
        type Input = (A, B, C, D, E, F);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F> EncodeRequest for     &&&&&&ReqSer<(A, B, C, D, E, F)> where A: Prts, B: Prts, C: Prts, D: DeO_____, E: DeO_____, F: DeO_____ {
        type Input = (A, B, C, D, E, F);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F> EncodeRequest for      &&&&&ReqSer<(A, B, C, D, E, F)> where A: Prts, B: Prts, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
        type Input = (A, B, C, D, E, F);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F> EncodeRequest for       &&&&ReqSer<(A, B, C, D, E, F)> where A: Prts, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
        type Input = (A, B, C, D, E, F);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F> EncodeRequest for        &&&ReqSer<(A, B, C, D, E, F)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
        type Input = (A, B, C, D, E, F);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }



    // the seven-arg case
    impl<A, B, C, D, E, F, G> EncodeRequest for &&&&&&&&&&ReqSer<(A, B, C, D, E, F, G)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Prts, G: Freq {
        type Input = (A, B, C, D, E, F, G);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G> EncodeRequest for  &&&&&&&&&ReqSer<(A, B, C, D, E, F, G)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Prts, G: Prts {
        type Input = (A, B, C, D, E, F, G);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G> EncodeRequest for   &&&&&&&&ReqSer<(A, B, C, D, E, F, G)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Prts, G: DeO_____ {
        type Input = (A, B, C, D, E, F, G);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G> EncodeRequest for    &&&&&&&ReqSer<(A, B, C, D, E, F, G)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: DeO_____, G: DeO_____ {
        type Input = (A, B, C, D, E, F, G);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G> EncodeRequest for     &&&&&&ReqSer<(A, B, C, D, E, F, G)> where A: Prts, B: Prts, C: Prts, D: Prts, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Input = (A, B, C, D, E, F, G);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G> EncodeRequest for      &&&&&ReqSer<(A, B, C, D, E, F, G)> where A: Prts, B: Prts, C: Prts, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Input = (A, B, C, D, E, F, G);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G> EncodeRequest for       &&&&ReqSer<(A, B, C, D, E, F, G)> where A: Prts, B: Prts, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Input = (A, B, C, D, E, F, G);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G> EncodeRequest for        &&&ReqSer<(A, B, C, D, E, F, G)> where A: Prts, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Input = (A, B, C, D, E, F, G);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G> EncodeRequest for         &&ReqSer<(A, B, C, D, E, F, G)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        type Input = (A, B, C, D, E, F, G);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }



    // the eight-arg case
    impl<A, B, C, D, E, F, G, H> EncodeRequest for &&&&&&&&&&ReqSer<(A, B, C, D, E, F, G, H)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Prts, G: Prts, H: Freq {
        type Input = (A, B, C, D, E, F, G, H);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> EncodeRequest for  &&&&&&&&&ReqSer<(A, B, C, D, E, F, G, H)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Prts, G: Prts, H: Prts {
        type Input = (A, B, C, D, E, F, G, H);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> EncodeRequest for   &&&&&&&&ReqSer<(A, B, C, D, E, F, G, H)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Prts, G: Prts, H: DeO_____ {
        type Input = (A, B, C, D, E, F, G, H);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> EncodeRequest for    &&&&&&&ReqSer<(A, B, C, D, E, F, G, H)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Prts, G: DeO_____, H: DeO_____ {
        type Input = (A, B, C, D, E, F, G, H);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> EncodeRequest for     &&&&&&ReqSer<(A, B, C, D, E, F, G, H)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Input = (A, B, C, D, E, F, G, H);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> EncodeRequest for      &&&&&ReqSer<(A, B, C, D, E, F, G, H)> where A: Prts, B: Prts, C: Prts, D: Prts, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Input = (A, B, C, D, E, F, G, H);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> EncodeRequest for       &&&&ReqSer<(A, B, C, D, E, F, G, H)> where A: Prts, B: Prts, C: Prts, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Input = (A, B, C, D, E, F, G, H);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> EncodeRequest for        &&&ReqSer<(A, B, C, D, E, F, G, H)> where A: Prts, B: Prts, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Input = (A, B, C, D, E, F, G, H);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> EncodeRequest for         &&ReqSer<(A, B, C, D, E, F, G, H)> where A: Prts, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Input = (A, B, C, D, E, F, G, H);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
    impl<A, B, C, D, E, F, G, H> EncodeRequest for          &ReqSer<(A, B, C, D, E, F, G, H)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        type Input = (A, B, C, D, E, F, G, H);
        fn encode<O>(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Result<O, ServerFnRejection>> + Send + 'static { async move { todo!() } }
    }
}
