use std::{
    any::{type_name, TypeId},
    prelude::rust_2024::Future,
};

use dioxus_fullstack_core::ServerFnError;
use serde::de::DeserializeOwned;

pub trait FromResponse<M>: Sized {
    fn from_response(
        res: reqwest::Response,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send;
}

pub struct DefaultEncoding;
impl<T> FromResponse<DefaultEncoding> for T
where
    T: DeserializeOwned + 'static,
{
    fn from_response(
        res: reqwest::Response,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        send_wrapper::SendWrapper::new(async move {
            let bytes = res.bytes().await.unwrap();
            let as_str = String::from_utf8_lossy(&bytes);
            tracing::info!(
                "Response bytes: {:?} for type {:?} ({})",
                as_str,
                TypeId::of::<T>(),
                type_name::<T>()
            );

            // let resas_astr = Ok(as_str);
            let res = serde_json::from_slice::<T>(&bytes);
            // let res = res.json::<T>().await;
            match res {
                Err(err) => Err(ServerFnError::Deserialization(err.to_string())),
                Ok(res) => Ok(res),
            }
        })
    }
}

pub trait IntoRequest<M> {
    type Input;
    type Output;
    fn into_request(input: Self::Input) -> Result<Self::Output, ServerFnError>;
}

pub use req_from::*;
pub use req_to::*;

pub mod req_to {
    use std::prelude::rust_2024::Future;

    use axum_core::extract::{FromRequest, Request};
    use http::HeaderMap;
    pub use impls::*;

    use crate::{DioxusServerState, ServerFnRejection};

    pub struct EncodeState {
        pub client: reqwest::RequestBuilder,
    }

    unsafe impl Send for EncodeState {}
    unsafe impl Sync for EncodeState {}

    pub struct ClientRequest<In, Out, M = (), BodyTy = (), Body = BodyTy> {
        _marker: std::marker::PhantomData<M>,
        _out: std::marker::PhantomData<Out>,
        _t: std::marker::PhantomData<In>,
        _body: std::marker::PhantomData<BodyTy>,
        _encoding: std::marker::PhantomData<Body>,
    }

    unsafe impl<A, B, C> Send for ClientRequest<A, B, C> {}
    unsafe impl<A, B, C> Sync for ClientRequest<A, B, C> {}

    fn assert_is_send(_: impl Send) {}
    fn check_it() {
        // assert_is_send(DeSer::<(HeaderMap, Json<String>), Json<String>>::new());
        // assert_is_send( &&&&&&&&DeSer<(A,)>);
    }

    impl<T, Out, Encoding> ClientRequest<T, Out, Encoding> {
        pub fn new() -> Self {
            ClientRequest {
                _marker: std::marker::PhantomData,
                _out: std::marker::PhantomData,
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

    pub struct EncodedBody {
        pub data: bytes::Bytes,
        pub content_type: &'static str,
    }

    #[allow(clippy::manual_async_fn)]
    #[rustfmt::skip]
    mod impls {
        use crate::{FromResponse, ServerFnError};

        use super::*;

        /*
        Handle the regular axum-like handlers with tiered overloading with a single trait.
        */
        pub trait EncodeRequest {
            type Input;
            type Output;
            fn fetch(&self, ctx: EncodeState, data: Self::Input) -> impl Future<Output = Self::Output> + Send + 'static;
        }

        use axum_core::extract::FromRequest as Freq;
        use axum_core::extract::FromRequestParts as Prts;
        use serde::ser::Serialize as DeO_____;


        // fallback case for *all invalid*
        // todo...
        impl<In, Out> EncodeRequest for ClientRequest<In, Out> {
            type Input = In;
            type Output = Out;
            fn fetch(&self, _ctx: EncodeState, _data: Self::Input) -> impl Future<Output = Out> + Send + 'static {
                async move { panic!("Could not encode request") }
            }
        }

        // Zero-arg case
        impl<O: FromResponse<M>, E, M> EncodeRequest for &&&&&&&&&&ClientRequest<(), Result<O, E>, M> where E: From<ServerFnError> {
            type Input = ();
            type Output = Result<O, E>;
            fn fetch(&self, ctx: EncodeState, data: Self::Input) -> impl Future<Output = Self::Output> + Send + 'static {
                send_wrapper::SendWrapper::new(async move {
                    let res = ctx.client.body(String::new()).send().await;
                    match res {
                        Ok(res) => O::from_response(res).await.map_err(|e| e.into()),
                        Err(err) => Err(ServerFnError::Request { message: err.to_string(), code: err.status().map(|s| s.as_u16()) }.into())
                    }
                })
            }
        }

        // One-arg case
        impl<A, O, E> EncodeRequest for &&&&&&&&&&ClientRequest<(A,), Result<O, E>> where A: Freq<DioxusServerState>, E: From<ServerFnError> {
            type Input = (A,);
            type Output = Result<O, E>;
            fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Self::Output> + Send + 'static {
                async move { todo!() }
            }
        }
        impl<A, O, E> EncodeRequest for  &&&&&&&&&ClientRequest<(A,), Result<O, E>> where A: Prts<DioxusServerState>, E: From<ServerFnError> {
            type Input = (A,);
            type Output = Result<O, E>;
            fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Self::Output> + Send + 'static {
                async move { todo!() }
            }
        }
        impl<A, O, E> EncodeRequest for   &&&&&&&&ClientRequest<(A,), Result<O, E>> where A: DeO_____, E: From<ServerFnError> {
            type Input = (A,);
            type Output = Result<O, E>;
            fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Self::Output> + Send + 'static {
                async move { todo!() }
            }
        }


        // Two-arg case
        impl<A, B, O, E> EncodeRequest for &&&&&&&&&&ClientRequest<(A, B), Result<O, E>> where A: Prts<DioxusServerState>, B: Freq<DioxusServerState>, E: From<ServerFnError> {
            type Input = (A, B);
            type Output = Result<O, E>;
            fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Self::Output> + Send + 'static {
                async move { todo!() }
            }
        }
        impl<A, B, O, E> EncodeRequest for  &&&&&&&&&ClientRequest<(A, B), Result<O, E>> where A: Prts<DioxusServerState>, B: Prts<DioxusServerState>, E: From<ServerFnError> {
            type Input = (A, B);
            type Output = Result<O, E>;
            fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Self::Output> + Send + 'static {
                async move { todo!() }
            }
        }
        impl<A, B, O, E> EncodeRequest for   &&&&&&&&ClientRequest<(A, B), Result<O, E>> where A: Prts<DioxusServerState>, B: DeO_____, E: From<ServerFnError> {
            type Input = (A, B);
            type Output = Result<O, E>;
            fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Self::Output> + Send + 'static {
                async move { todo!() }
            }
        }
        impl<A, B, O, E> EncodeRequest for    &&&&&&&ClientRequest<(A, B), Result<O, E>> where A: DeO_____, B: DeO_____, E: From<ServerFnError> {
            type Input = (A, B);
            type Output = Result<O, E>;
            fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Self::Output> + Send + 'static {
                async move { todo!() }
            }
        }


        // // the three-arg case
        // impl<A, B, C> EncodeRequest for &&&&&&&&&&ClientRequest<(A, B, C)> where A: Prts, B: Prts, C: Freq, {
        //     type Input = (A, B, C);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C> EncodeRequest for  &&&&&&&&&ClientRequest<(A, B, C)> where A: Prts, B: Prts, C: Prts {
        //     type Input = (A, B, C);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C> EncodeRequest for   &&&&&&&&ClientRequest<(A, B, C)> where A: Prts, B: Prts {
        //     type Input = (A, B, C);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C> EncodeRequest for   &&&&&&&ClientRequest<(A, B, C)> where A: Prts {
        //     type Input = (A, B, C);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C> EncodeRequest for    &&&&&&ClientRequest<(A, B, C)>  {
        //     type Input = (A, B, C);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }



        // // the four-arg case
        // impl<A, B, C, D> EncodeRequest for &&&&&&&&&&ClientRequest<(A, B, C, D)> where A: Prts, B: Prts, C: Prts, D: Freq {
        //     type Input = (A, B, C, D);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D> EncodeRequest for  &&&&&&&&&ClientRequest<(A, B, C, D)> where A: Prts, B: Prts, C: Prts, D: Prts {
        //     type Input = (A, B, C, D);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D> EncodeRequest for   &&&&&&&&ClientRequest<(A, B, C, D)> where A: Prts, B: Prts, C: Prts, D: DeO_____ {
        //     type Input = (A, B, C, D);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D> EncodeRequest for    &&&&&&&ClientRequest<(A, B, C, D)> where A: Prts, B: Prts, C: DeO_____, D: DeO_____ {
        //     type Input = (A, B, C, D);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D> EncodeRequest for     &&&&&&ClientRequest<(A, B, C, D)> where A: Prts, B: DeO_____, C: DeO_____, D: DeO_____ {
        //     type Input = (A, B, C, D);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D> EncodeRequest for      &&&&&ClientRequest<(A, B, C, D)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____ {
        //     type Input = (A, B, C, D);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }

        // // the five-arg case
        // impl<A, B, C, D, E> EncodeRequest for &&&&&&&&&&ClientRequest<(A, B, C, D, E)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Freq {
        //     type Input = (A, B, C, D, E);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E> EncodeRequest for  &&&&&&&&&ClientRequest<(A, B, C, D, E)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts {
        //     type Input = (A, B, C, D, E);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E> EncodeRequest for   &&&&&&&&ClientRequest<(A, B, C, D, E)> where A: Prts, B: Prts, C: Prts, D: Prts, E: DeO_____ {
        //     type Input = (A, B, C, D, E);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E> EncodeRequest for    &&&&&&&ClientRequest<(A, B, C, D, E)> where A: Prts, B: Prts, C: Prts, D: DeO_____, E: DeO_____ {
        //     type Input = (A, B, C, D, E);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E> EncodeRequest for     &&&&&&ClientRequest<(A, B, C, D, E)> where A: Prts, B: Prts, C: DeO_____, D: DeO_____, E: DeO_____ {
        //     type Input = (A, B, C, D, E);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E> EncodeRequest for      &&&&&ClientRequest<(A, B, C, D, E)> where A: Prts, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____ {
        //     type Input = (A, B, C, D, E);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E> EncodeRequest for       &&&&ClientRequest<(A, B, C, D, E)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____ {
        //     type Input = (A, B, C, D, E);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }

        // // the six-arg case
        // impl<A, B, C, D, E, F> EncodeRequest for &&&&&&&&&&ClientRequest<(A, B, C, D, E, F)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Freq {
        //     type Input = (A, B, C, D, E, F);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F> EncodeRequest for  &&&&&&&&&ClientRequest<(A, B, C, D, E, F)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Prts {
        //     type Input = (A, B, C, D, E, F);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F> EncodeRequest for   &&&&&&&&ClientRequest<(A, B, C, D, E, F)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: DeO_____ {
        //     type Input = (A, B, C, D, E, F);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F> EncodeRequest for    &&&&&&&ClientRequest<(A, B, C, D, E, F)> where A: Prts, B: Prts, C: Prts, D: Prts, E: DeO_____, F: DeO_____ {
        //     type Input = (A, B, C, D, E, F);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F> EncodeRequest for     &&&&&&ClientRequest<(A, B, C, D, E, F)> where A: Prts, B: Prts, C: Prts, D: DeO_____, E: DeO_____, F: DeO_____ {
        //     type Input = (A, B, C, D, E, F);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F> EncodeRequest for      &&&&&ClientRequest<(A, B, C, D, E, F)> where A: Prts, B: Prts, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
        //     type Input = (A, B, C, D, E, F);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F> EncodeRequest for       &&&&ClientRequest<(A, B, C, D, E, F)> where A: Prts, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
        //     type Input = (A, B, C, D, E, F);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F> EncodeRequest for        &&&ClientRequest<(A, B, C, D, E, F)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
        //     type Input = (A, B, C, D, E, F);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }



        // // the seven-arg case
        // impl<A, B, C, D, E, F, G> EncodeRequest for &&&&&&&&&&ClientRequest<(A, B, C, D, E, F, G)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Prts, G: Freq {
        //     type Input = (A, B, C, D, E, F, G);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G> EncodeRequest for  &&&&&&&&&ClientRequest<(A, B, C, D, E, F, G)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Prts, G: Prts {
        //     type Input = (A, B, C, D, E, F, G);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G> EncodeRequest for   &&&&&&&&ClientRequest<(A, B, C, D, E, F, G)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Prts, G: DeO_____ {
        //     type Input = (A, B, C, D, E, F, G);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G> EncodeRequest for    &&&&&&&ClientRequest<(A, B, C, D, E, F, G)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: DeO_____, G: DeO_____ {
        //     type Input = (A, B, C, D, E, F, G);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G> EncodeRequest for     &&&&&&ClientRequest<(A, B, C, D, E, F, G)> where A: Prts, B: Prts, C: Prts, D: Prts, E: DeO_____, F: DeO_____, G: DeO_____ {
        //     type Input = (A, B, C, D, E, F, G);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G> EncodeRequest for      &&&&&ClientRequest<(A, B, C, D, E, F, G)> where A: Prts, B: Prts, C: Prts, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        //     type Input = (A, B, C, D, E, F, G);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G> EncodeRequest for       &&&&ClientRequest<(A, B, C, D, E, F, G)> where A: Prts, B: Prts, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        //     type Input = (A, B, C, D, E, F, G);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G> EncodeRequest for        &&&ClientRequest<(A, B, C, D, E, F, G)> where A: Prts, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        //     type Input = (A, B, C, D, E, F, G);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G> EncodeRequest for         &&ClientRequest<(A, B, C, D, E, F, G)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
        //     type Input = (A, B, C, D, E, F, G);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }



        // // the eight-arg case
        // impl<A, B, C, D, E, F, G, H> EncodeRequest for &&&&&&&&&&ClientRequest<(A, B, C, D, E, F, G, H)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Prts, G: Prts, H: Freq {
        //     type Input = (A, B, C, D, E, F, G, H);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G, H> EncodeRequest for  &&&&&&&&&ClientRequest<(A, B, C, D, E, F, G, H)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Prts, G: Prts, H: Prts {
        //     type Input = (A, B, C, D, E, F, G, H);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G, H> EncodeRequest for   &&&&&&&&ClientRequest<(A, B, C, D, E, F, G, H)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Prts, G: Prts, H: DeO_____ {
        //     type Input = (A, B, C, D, E, F, G, H);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G, H> EncodeRequest for    &&&&&&&ClientRequest<(A, B, C, D, E, F, G, H)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: Prts, G: DeO_____, H: DeO_____ {
        //     type Input = (A, B, C, D, E, F, G, H);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G, H> EncodeRequest for     &&&&&&ClientRequest<(A, B, C, D, E, F, G, H)> where A: Prts, B: Prts, C: Prts, D: Prts, E: Prts, F: DeO_____, G: DeO_____, H: DeO_____ {
        //     type Input = (A, B, C, D, E, F, G, H);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G, H> EncodeRequest for      &&&&&ClientRequest<(A, B, C, D, E, F, G, H)> where A: Prts, B: Prts, C: Prts, D: Prts, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        //     type Input = (A, B, C, D, E, F, G, H);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G, H> EncodeRequest for       &&&&ClientRequest<(A, B, C, D, E, F, G, H)> where A: Prts, B: Prts, C: Prts, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        //     type Input = (A, B, C, D, E, F, G, H);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G, H> EncodeRequest for        &&&ClientRequest<(A, B, C, D, E, F, G, H)> where A: Prts, B: Prts, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        //     type Input = (A, B, C, D, E, F, G, H);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G, H> EncodeRequest for         &&ClientRequest<(A, B, C, D, E, F, G, H)> where A: Prts, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        //     type Input = (A, B, C, D, E, F, G, H);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
        // impl<A, B, C, D, E, F, G, H> EncodeRequest for          &ClientRequest<(A, B, C, D, E, F, G, H)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
        //     type Input = (A, B, C, D, E, F, G, H);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = O> + Send + 'static { async move { todo!() } }
        // }
    }
}

pub mod req_from {
    use std::prelude::rust_2024::Future;

    use axum_core::extract::{FromRequest, Request};
    use http::HeaderMap;
    pub use impls::*;

    use crate::ServerFnRejection;

    #[derive(Default)]
    pub struct ExtractState {
        request: Request,
        names: (&'static str, &'static str, &'static str),
    }

    unsafe impl Send for ExtractState {}
    unsafe impl Sync for ExtractState {}

    pub struct DeSer<T, BodyTy = (), B = ()> {
        _t: std::marker::PhantomData<T>,
        _body: std::marker::PhantomData<BodyTy>,
        _encoding: std::marker::PhantomData<B>,
    }

    unsafe impl<A, B, C> Send for DeSer<A, B, C> {}
    unsafe impl<A, B, C> Sync for DeSer<A, B, C> {}

    fn assert_is_send(_: impl Send) {}
    fn check_it() {
        // (&&&&&&&&&&&&&&&&&&&DeSer::<(HeaderMap, Json<String>), Json<String>>::new()
        //     .extract_request(request));
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
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static;
        }

        use axum_core::extract::FromRequest as Freq;
        use axum_core::extract::FromRequestParts as Prts;
        use dioxus_fullstack_core::DioxusServerState;
        use serde::de::DeserializeOwned as DeO_____;
        use DioxusServerState as Ds;

        // Zero-arg case
        impl ExtractRequest for &&&&&&&&&&DeSer<()> {
            type Output = ();
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static {
                async move { Ok(()) }
            }
        }

        // One-arg case
        impl<A> ExtractRequest for &&&&&&&&&&DeSer<(A,)> where A: Freq<Ds> {
            type Output = (A,);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A> ExtractRequest for  &&&&&&&&&DeSer<(A,)> where A: Prts<Ds> {
            type Output = (A,);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A> ExtractRequest for   &&&&&&&&DeSer<(A,)> where A: DeO_____ {
            type Output = (A,);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }


        // Two-arg case
        impl<A, B> ExtractRequest for &&&&&&&&&&DeSer<(A, B)> where A: Prts<Ds>, B: Freq<Ds> {
            type Output = (A, B);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B> ExtractRequest for  &&&&&&&&&DeSer<(A, B)> where A: Prts<Ds>, B: Prts<Ds> {
            type Output = (A, B);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B> ExtractRequest for   &&&&&&&&DeSer<(A, B)> where A: Prts<Ds>, B: DeO_____ {
            type Output = (A, B);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B> ExtractRequest for    &&&&&&&DeSer<(A, B)> where A: DeO_____, B: DeO_____ {
            type Output = (A, B);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }


        // the three-arg case
        impl<A, B, C> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C)> where A: Prts<Ds>, B: Prts<Ds>, C: Freq<Ds>, {
            type Output = (A, B, C);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds> {
            type Output = (A, B, C);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C> ExtractRequest for   &&&&&&&&DeSer<(A, B, C)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____ {
            type Output = (A, B, C);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C> ExtractRequest for   &&&&&&&DeSer<(A, B, C)> where A: Prts<Ds>, B: DeO_____, C: DeO_____ {
            type Output = (A, B, C);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C> ExtractRequest for    &&&&&&DeSer<(A, B, C)> where A: DeO_____, B: DeO_____, C: DeO_____ {
            type Output = (A, B, C);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }



        // the four-arg case
        impl<A, B, C, D> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Freq<Ds> {
            type Output = (A, B, C, D);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds> {
            type Output = (A, B, C, D);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: DeO_____ {
            type Output = (A, B, C, D);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____, D: DeO_____ {
            type Output = (A, B, C, D);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D> ExtractRequest for     &&&&&&DeSer<(A, B, C, D)> where A: Prts<Ds>, B: DeO_____, C: DeO_____, D: DeO_____ {
            type Output = (A, B, C, D);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D> ExtractRequest for      &&&&&DeSer<(A, B, C, D)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____ {
            type Output = (A, B, C, D);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }

        // the five-arg case
        impl<A, B, C, D, E> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Freq<Ds> {
            type Output = (A, B, C, D, E);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds> {
            type Output = (A, B, C, D, E);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: DeO_____ {
            type Output = (A, B, C, D, E);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: DeO_____, E: DeO_____ {
            type Output = (A, B, C, D, E);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____, D: DeO_____, E: DeO_____ {
            type Output = (A, B, C, D, E);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E)> where A: Prts<Ds>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____ {
            type Output = (A, B, C, D, E);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E> ExtractRequest for       &&&&DeSer<(A, B, C, D, E)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____ {
            type Output = (A, B, C, D, E);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }

        // the six-arg case
        impl<A, B, C, D, E, F> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Freq<Ds> {
            type Output = (A, B, C, D, E, F);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds> {
            type Output = (A, B, C, D, E, F);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: DeO_____ {
            type Output = (A, B, C, D, E, F);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: DeO_____, F: DeO_____ {
            type Output = (A, B, C, D, E, F);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: DeO_____, E: DeO_____, F: DeO_____ {
            type Output = (A, B, C, D, E, F);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
            type Output = (A, B, C, D, E, F);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F> ExtractRequest for       &&&&DeSer<(A, B, C, D, E, F)> where A: Prts<Ds>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
            type Output = (A, B, C, D, E, F);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F> ExtractRequest for        &&&DeSer<(A, B, C, D, E, F)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
            type Output = (A, B, C, D, E, F);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }



        // the seven-arg case
        impl<A, B, C, D, E, F, G> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: Freq<Ds> {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: Prts<Ds> {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: DeO_____ {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: DeO_____, G: DeO_____ {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: DeO_____, F: DeO_____, G: DeO_____ {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G> ExtractRequest for       &&&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G> ExtractRequest for        &&&DeSer<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G> ExtractRequest for         &&DeSer<(A, B, C, D, E, F, G)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }



        // the eight-arg case
        impl<A, B, C, D, E, F, G, H> ExtractRequest for &&&&&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: Prts<Ds>, H: Freq<Ds> {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for  &&&&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: Prts<Ds>, H: Prts<Ds> {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for   &&&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: Prts<Ds>, H: DeO_____ {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for    &&&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: DeO_____, H: DeO_____ {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for     &&&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: DeO_____, G: DeO_____, H: DeO_____ {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for      &&&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for       &&&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for        &&&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for         &&DeSer<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for          &DeSer<(A, B, C, D, E, F, G, H)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
    }
}
