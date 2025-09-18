//! ServerFn request decoders and encoders.
//!
//! The Dioxus Server Function implementation brings a lot of *magic* to the types of endpoints we can handle.
//! Our ultimate goal is to handle *all* endpoints, even axum endpoints, with the macro.
//!
//! Unfortunately, some axum traits like `FromRequest` overlap with some of the default magic we want
//! to provide, like allowing DeserializedOwned groups.
//!
//! Our ultimate goal - to accept all axum handlers - is feasible but not implemented.
//!
//! Broadly, we support the following categories of handlers arguments:
//! - Handlers with a single argument that implements `FromRequest` + `IntoRequest`
//! - Handlers with multiple arguments that implement *all* `DeserializeOwned` (and thus can be deserialized from a JSON body)
//!
//! The handler error return types we support are:
//! - Result<T, E> where E: From<ServerFnError> + Serialize + DeserializeOwned (basically any custom `thiserror` impl)
//! - Result<T, anyhow::Error> where we transport the error as a string and/or through ServerFnError
//!
//! The handler return types we support are:
//! - T where T: FromResponse
//! - T where T: DeserializeOwned
//!
//! Note that FromResponse and IntoRequest are *custom* traits defined in this crate. The intention
//! is to provide "inverse" traits of the axum traits, allowing types to flow seamlessly between client and server.
//!
//! These are unfortunately in conflict with the serialization traits. Types like `Bytes` implement both
//! IntoResponse and Serialize, so what should you use?
//!
//! This module implements auto-deref specialization to allow tiering of the above cases.
//!
//! This is sadly quite "magical", but it works. Because the FromResponse traits are defined in this crate,
//! they are sealed against types that implement Deserialize/Serialize, meaning you cannot implement
//! FromResponse for a type that implements Serialize.
//!
//! This module is broken up into several parts, attempting to match how the server macro generates code:
//! - ReqwestEncoder: encodes a set of arguments into a reqwest request

use std::{
    any::{type_name, TypeId},
    pin::Pin,
    prelude::rust_2024::Future,
};

use axum::Json;
use dioxus_fullstack_core::ServerFnError;
use futures::FutureExt;
use serde::{de::DeserializeOwned, Serialize};

#[pin_project::pin_project]
#[must_use = "Requests do nothing unless you `.await` them"]
pub struct ServerFnRequest<Output> {
    _phantom: std::marker::PhantomData<Output>,
    #[pin]
    fut: Pin<Box<dyn Future<Output = Output> + Send>>,
}

impl<O> ServerFnRequest<O> {
    pub fn new(res: impl Future<Output = O> + Send + 'static) -> Self {
        ServerFnRequest {
            _phantom: std::marker::PhantomData,
            fut: Box::pin(res),
        }
    }
}

impl<T, E> std::future::Future for ServerFnRequest<Result<T, E>> {
    type Output = Result<T, E>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.project().fut.poll(cx)
    }
}

pub trait FromResponse: Sized {
    fn from_response(
        res: reqwest::Response,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send;
}

pub trait IntoRequest: Sized {
    type Output;
    fn into_request(
        input: Self,
        request_builder: reqwest::RequestBuilder,
    ) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> + Send + 'static;
}

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

    pub struct ReqwestEncoder<In> {
        _t: std::marker::PhantomData<In>,
    }
    unsafe impl<A> Send for ReqwestEncoder<A> {}
    unsafe impl<A> Sync for ReqwestEncoder<A> {}
    impl<T> ReqwestEncoder<T> {
        pub fn new() -> Self {
            ReqwestEncoder {
                _t: std::marker::PhantomData,
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
        use axum_core::extract::FromRequest as Freq;
        use axum_core::extract::FromRequestParts as Prts;
        use serde::{ser::Serialize as DeO_____, Serialize};
        use dioxus_fullstack_core::DioxusServerState as Dsr;
        use crate::{FromResponse, IntoRequest, ServerFnError};

        use super::*;

        type Res = Result<reqwest::Response, reqwest::Error>;

        /*
        Handle the regular axum-like handlers with tiered overloading with a single trait.
        */
        pub trait EncodeRequest {
            type Input;
            fn fetch(&self, ctx: EncodeState, data: Self::Input) -> impl Future<Output = Res> + Send + 'static;
        }

        // // fallback case for *all invalid*
        // // todo...
        // impl<In> EncodeRequest for ReqwestEncoder<In> {
        //     type Input = In;
        //     fn fetch(&self, _ctx: EncodeState, _data: Self::Input) -> impl Future<Output = Res> + Send + 'static {
        //         async move { panic!("Could not encode request") }
        //     }
        // }

        // Zero-arg case
        impl EncodeRequest for &&&&&&&&&&ReqwestEncoder<()> {
            type Input = ();
            fn fetch(&self, ctx: EncodeState, _: Self::Input) -> impl Future<Output = Res> + Send + 'static {
                send_wrapper::SendWrapper::new(async move {
                    ctx.client.send().await
                })
            }
        }

        // One-arg case
        impl<A> EncodeRequest for &&&&&&&&&&ReqwestEncoder<(A,)> where A: DeO_____ + Serialize + 'static {
            type Input = (A,);
            fn fetch(&self, ctx: EncodeState, data: Self::Input) -> impl Future<Output = Res> + Send + 'static {
                send_wrapper::SendWrapper::new(async move {
                    let (a,) = data;
                    #[derive(Serialize)]
                    struct SerOne<A> {
                        data: A,
                    }

                    ctx.client.body(serde_json::to_string(&SerOne { data: a }).unwrap()).send().await
                })
            }
        }

        impl<A: 'static> EncodeRequest for &&&&&&&&&ReqwestEncoder<(A,)> where A: Freq<Dsr> + IntoRequest {
            type Input = (A,);
            fn fetch(&self, ctx: EncodeState, data: Self::Input) -> impl Future<Output = Res> + Send + 'static {
                A::into_request(data.0, ctx.client)
            }
        }

        // impl<A> EncodeRequest for  &&&&&&&&ReqwestEncoder<(A,)> where A: Prts<Dsr> {
        //     type Input = (A,);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Res> + Send + 'static {
        //         async move { todo!() }
        //     }
        // }


        // Two-arg case
        // impl<A, B> EncodeRequest for &&&&&&&&&&ReqwestEncoder<(A, B)> where A: Prts<Dsr>, B: Freq<Dsr> {
        //     type Input = (A, B);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Res> + Send + 'static {
        //         async move { todo!() }
        //     }
        // }
        // impl<A, B> EncodeRequest for  &&&&&&&&&ReqwestEncoder<(A, B)> where A: Prts<Dsr>, B: Prts<Dsr> {
        //     type Input = (A, B);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Res> + Send + 'static {
        //         async move { todo!() }
        //     }
        // }

        // impl<A, B> EncodeRequest for   &&&&&&&&ReqwestEncoder<(A, B)> where A: Prts<Dsr>, B: DeO_____ {
        //     type Input = (A, B);
        //     fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Res> + Send + 'static {
        //         async move { todo!() }
        //     }
        // }

        impl<A, B> EncodeRequest for    &&&&&&&ReqwestEncoder<(A, B)> where A: DeO_____, B: DeO_____ {
            type Input = (A, B);
            fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Res> + Send + 'static {
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

pub use req_from::*;
pub mod req_from {
    use std::prelude::rust_2024::Future;

    use axum_core::extract::{FromRequest, Request};
    use dioxus_fullstack_core::DioxusServerState;
    use http::HeaderMap;
    pub use impls::*;

    use crate::ServerFnRejection;

    #[derive(Default)]
    pub struct ExtractState {
        pub state: DioxusServerState,
        pub request: Request,
    }

    unsafe impl Send for ExtractState {}
    unsafe impl Sync for ExtractState {}

    pub struct AxumRequestDecoder<T, BodyTy = (), B = ()> {
        _t: std::marker::PhantomData<T>,
        _body: std::marker::PhantomData<BodyTy>,
        _encoding: std::marker::PhantomData<B>,
    }

    unsafe impl<A, B, C> Send for AxumRequestDecoder<A, B, C> {}
    unsafe impl<A, B, C> Sync for AxumRequestDecoder<A, B, C> {}

    fn assert_is_send(_: impl Send) {}
    fn check_it() {
        // (&&&&&&&&&&&&&&&&&&&DeSer::<(HeaderMap, Json<String>), Json<String>>::new()
        //     .extract_request(request));
    }

    impl<T, Encoding> AxumRequestDecoder<T, Encoding> {
        pub fn new() -> Self {
            AxumRequestDecoder {
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
        use bytes::Bytes;
        use dioxus_fullstack_core::DioxusServerState;
        use serde::de::DeserializeOwned as DeO_____;
        use DioxusServerState as Ds;

        // Zero-arg case
        impl ExtractRequest for &&&&&&&&&&AxumRequestDecoder<()> {
            type Output = ();
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static {
                async move { Ok(()) }
            }
        }

        // One-arg case
        impl<A> ExtractRequest for &&&&&&&&&&AxumRequestDecoder<(A,)> where A: DeO_____ {
            type Output = (A,);
            fn extract(&self, ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static {
                async move {
                    #[derive(serde::Deserialize)]
                    struct SerOne<A> {
                        data: A,
                    }

                    let bytes = Bytes::from_request(ctx.request, &()).await.unwrap();
                    let as_str = String::from_utf8_lossy(&bytes);
                    tracing::info!("deserializing request body: {}", as_str);
                    let res = serde_json::from_slice::<SerOne<A>>(&bytes).map(|s| (s.data,));
                    res.map_err(|e| ServerFnRejection {})
                }
            }
        }
        impl<A> ExtractRequest for  &&&&&&&&&AxumRequestDecoder<(A,)> where A: Freq<Ds> {
            type Output = (A,);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static {
                send_wrapper::SendWrapper::new(async move {
                    let res: Result<A, A::Rejection> = A::from_request(_ctx.request, &_ctx.state)
                        .await;

                    res.map(|a| (a,)).map_err(|_e| ServerFnRejection {})
                })
            }
        }

        // impl<A> ExtractRequest for   &&&&&&&&AxumRequestDecoder<(A,)> where A: Prts<Ds> {
        //     type Output = (A,);
        //     fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        // }


        // Two-arg case
        impl<A, B> ExtractRequest for &&&&&&&&&&AxumRequestDecoder<(A, B)> where A: Prts<Ds>, B: Freq<Ds> {
            type Output = (A, B);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B> ExtractRequest for  &&&&&&&&&AxumRequestDecoder<(A, B)> where A: Prts<Ds>, B: Prts<Ds> {
            type Output = (A, B);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B> ExtractRequest for   &&&&&&&&AxumRequestDecoder<(A, B)> where A: Prts<Ds>, B: DeO_____ {
            type Output = (A, B);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B> ExtractRequest for    &&&&&&&AxumRequestDecoder<(A, B)> where A: DeO_____, B: DeO_____ {
            type Output = (A, B);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }


        // the three-arg case
        impl<A, B, C> ExtractRequest for &&&&&&&&&&AxumRequestDecoder<(A, B, C)> where A: Prts<Ds>, B: Prts<Ds>, C: Freq<Ds>, {
            type Output = (A, B, C);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C> ExtractRequest for  &&&&&&&&&AxumRequestDecoder<(A, B, C)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds> {
            type Output = (A, B, C);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C> ExtractRequest for   &&&&&&&&AxumRequestDecoder<(A, B, C)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____ {
            type Output = (A, B, C);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C> ExtractRequest for   &&&&&&&AxumRequestDecoder<(A, B, C)> where A: Prts<Ds>, B: DeO_____, C: DeO_____ {
            type Output = (A, B, C);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C> ExtractRequest for    &&&&&&AxumRequestDecoder<(A, B, C)> where A: DeO_____, B: DeO_____, C: DeO_____ {
            type Output = (A, B, C);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }



        // the four-arg case
        impl<A, B, C, D> ExtractRequest for &&&&&&&&&&AxumRequestDecoder<(A, B, C, D)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Freq<Ds> {
            type Output = (A, B, C, D);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D> ExtractRequest for  &&&&&&&&&AxumRequestDecoder<(A, B, C, D)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds> {
            type Output = (A, B, C, D);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D> ExtractRequest for   &&&&&&&&AxumRequestDecoder<(A, B, C, D)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: DeO_____ {
            type Output = (A, B, C, D);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D> ExtractRequest for    &&&&&&&AxumRequestDecoder<(A, B, C, D)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____, D: DeO_____ {
            type Output = (A, B, C, D);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D> ExtractRequest for     &&&&&&AxumRequestDecoder<(A, B, C, D)> where A: Prts<Ds>, B: DeO_____, C: DeO_____, D: DeO_____ {
            type Output = (A, B, C, D);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D> ExtractRequest for      &&&&&AxumRequestDecoder<(A, B, C, D)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____ {
            type Output = (A, B, C, D);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }

        // the five-arg case
        impl<A, B, C, D, E> ExtractRequest for &&&&&&&&&&AxumRequestDecoder<(A, B, C, D, E)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Freq<Ds> {
            type Output = (A, B, C, D, E);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E> ExtractRequest for  &&&&&&&&&AxumRequestDecoder<(A, B, C, D, E)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds> {
            type Output = (A, B, C, D, E);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E> ExtractRequest for   &&&&&&&&AxumRequestDecoder<(A, B, C, D, E)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: DeO_____ {
            type Output = (A, B, C, D, E);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E> ExtractRequest for    &&&&&&&AxumRequestDecoder<(A, B, C, D, E)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: DeO_____, E: DeO_____ {
            type Output = (A, B, C, D, E);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E> ExtractRequest for     &&&&&&AxumRequestDecoder<(A, B, C, D, E)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____, D: DeO_____, E: DeO_____ {
            type Output = (A, B, C, D, E);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E> ExtractRequest for      &&&&&AxumRequestDecoder<(A, B, C, D, E)> where A: Prts<Ds>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____ {
            type Output = (A, B, C, D, E);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E> ExtractRequest for       &&&&AxumRequestDecoder<(A, B, C, D, E)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____ {
            type Output = (A, B, C, D, E);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }

        // the six-arg case
        impl<A, B, C, D, E, F> ExtractRequest for &&&&&&&&&&AxumRequestDecoder<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Freq<Ds> {
            type Output = (A, B, C, D, E, F);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F> ExtractRequest for  &&&&&&&&&AxumRequestDecoder<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds> {
            type Output = (A, B, C, D, E, F);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F> ExtractRequest for   &&&&&&&&AxumRequestDecoder<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: DeO_____ {
            type Output = (A, B, C, D, E, F);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F> ExtractRequest for    &&&&&&&AxumRequestDecoder<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: DeO_____, F: DeO_____ {
            type Output = (A, B, C, D, E, F);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F> ExtractRequest for     &&&&&&AxumRequestDecoder<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: DeO_____, E: DeO_____, F: DeO_____ {
            type Output = (A, B, C, D, E, F);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F> ExtractRequest for      &&&&&AxumRequestDecoder<(A, B, C, D, E, F)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
            type Output = (A, B, C, D, E, F);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F> ExtractRequest for       &&&&AxumRequestDecoder<(A, B, C, D, E, F)> where A: Prts<Ds>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
            type Output = (A, B, C, D, E, F);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F> ExtractRequest for        &&&AxumRequestDecoder<(A, B, C, D, E, F)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____ {
            type Output = (A, B, C, D, E, F);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }



        // the seven-arg case
        impl<A, B, C, D, E, F, G> ExtractRequest for &&&&&&&&&&AxumRequestDecoder<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: Freq<Ds> {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G> ExtractRequest for  &&&&&&&&&AxumRequestDecoder<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: Prts<Ds> {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G> ExtractRequest for   &&&&&&&&AxumRequestDecoder<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: DeO_____ {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G> ExtractRequest for    &&&&&&&AxumRequestDecoder<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: DeO_____, G: DeO_____ {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G> ExtractRequest for     &&&&&&AxumRequestDecoder<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: DeO_____, F: DeO_____, G: DeO_____ {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G> ExtractRequest for      &&&&&AxumRequestDecoder<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G> ExtractRequest for       &&&&AxumRequestDecoder<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G> ExtractRequest for        &&&AxumRequestDecoder<(A, B, C, D, E, F, G)> where A: Prts<Ds>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G> ExtractRequest for         &&AxumRequestDecoder<(A, B, C, D, E, F, G)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____ {
            type Output = (A, B, C, D, E, F, G);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }



        // the eight-arg case
        impl<A, B, C, D, E, F, G, H> ExtractRequest for &&&&&&&&&&AxumRequestDecoder<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: Prts<Ds>, H: Freq<Ds> {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for  &&&&&&&&&AxumRequestDecoder<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: Prts<Ds>, H: Prts<Ds> {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for   &&&&&&&&AxumRequestDecoder<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: Prts<Ds>, H: DeO_____ {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for    &&&&&&&AxumRequestDecoder<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: Prts<Ds>, G: DeO_____, H: DeO_____ {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for     &&&&&&AxumRequestDecoder<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: Prts<Ds>, F: DeO_____, G: DeO_____, H: DeO_____ {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for      &&&&&AxumRequestDecoder<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: Prts<Ds>, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for       &&&&AxumRequestDecoder<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: Prts<Ds>, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for        &&&AxumRequestDecoder<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: Prts<Ds>, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for         &&AxumRequestDecoder<(A, B, C, D, E, F, G, H)> where A: Prts<Ds>, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A, B, C, D, E, F, G, H> ExtractRequest for          &AxumRequestDecoder<(A, B, C, D, E, F, G, H)> where A: DeO_____, B: DeO_____, C: DeO_____, D: DeO_____, E: DeO_____, F: DeO_____, G: DeO_____, H: DeO_____ {
            type Output = (A, B, C, D, E, F, G, H);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
    }
}

pub use resp::*;
mod resp {
    use axum::response::IntoResponse;
    use dioxus_fullstack_core::ServerFnError;
    use serde::{de::DeserializeOwned, Serialize};

    use crate::FromResponse;

    pub struct AxumResponseEncoder<I> {
        _p: std::marker::PhantomData<I>,
        prefers_content_type: Option<String>,
    }

    impl<I> AxumResponseEncoder<I> {
        pub fn new() -> Self {
            Self {
                _p: std::marker::PhantomData,
                prefers_content_type: None,
            }
        }
    }

    /// A trait for converting the result of the Server Function into an Axum response.
    ///
    /// This is to work around the issue where we want to return both Deserialize types and FromResponse types.
    /// Stuff like websockets
    ///
    /// We currently have an `Input` type even though it's not useful since we might want to support regular axum endpoints later.
    /// For now, it's just Result<T, E> where T is either DeserializeOwned or FromResponse
    pub trait FromResIt {
        type Input;
        fn make_axum_response(self, s: Self::Input) -> axum::response::Response;
    }

    // Higher priority impl for special types like websocket/file responses that generate their own responses
    // The FromResponse impl helps narrow types to those usable on the client
    impl<T, E> FromResIt for &&&AxumResponseEncoder<Result<T, E>>
    where
        T: FromResponse + IntoResponse,
        E: From<ServerFnError>,
    {
        type Input = Result<T, E>;
        fn make_axum_response(self, s: Self::Input) -> axum::response::Response {
            match s {
                Ok(res) => res.into_response(),
                Err(err) => todo!(),
            }
        }
    }

    // Lower priority impl for regular serializable types
    // We try to match the encoding from the incoming request, otherwise default to JSON
    impl<T, E> FromResIt for &&AxumResponseEncoder<Result<T, E>>
    where
        T: DeserializeOwned + Serialize,
        E: From<ServerFnError>,
    {
        type Input = Result<T, E>;
        fn make_axum_response(self, s: Self::Input) -> axum::response::Response {
            match s.map(|v| serde_json::to_string(&v)) {
                Ok(Ok(v)) => {
                    let mut res = (axum::http::StatusCode::OK, v).into_response();
                    res.headers_mut().insert(
                        axum::http::header::CONTENT_TYPE,
                        axum::http::HeaderValue::from_static("application/json"),
                    );
                    res
                }
                Ok(Err(e)) => {
                    todo!()
                }
                Err(e) => {
                    todo!()
                }
            }
        }
    }
}

pub use decode_ok::*;
mod decode_ok {
    use std::prelude::rust_2024::Future;

    use dioxus_fullstack_core::ServerFnError;
    use http::StatusCode;
    use serde::{de::DeserializeOwned, Serialize};

    use crate::FromResponse;

    pub struct ReqwestDecoder<T> {
        _p: std::marker::PhantomData<T>,
    }

    impl<T> ReqwestDecoder<T> {
        pub fn new() -> Self {
            Self {
                _p: std::marker::PhantomData,
            }
        }
    }

    /// Conver the reqwest response into the desired type, in place.
    /// The point here is to prefer FromResponse types *first* and then DeserializeOwned types second.
    ///
    /// This is because FromResponse types are more specialized and can handle things like websockets and files.
    /// DeserializeOwned types are more general and can handle things like JSON responses.
    pub trait ReqwestDecodeResult<T> {
        fn decode_response(
            &self,
            res: Result<reqwest::Response, reqwest::Error>,
        ) -> impl Future<Output = Result<Result<T, ServerFnError>, reqwest::Error>> + Send;
    }

    impl<T: FromResponse, E> ReqwestDecodeResult<T> for &&&ReqwestDecoder<Result<T, E>> {
        fn decode_response(
            &self,
            res: Result<reqwest::Response, reqwest::Error>,
        ) -> impl Future<Output = Result<Result<T, ServerFnError>, reqwest::Error>> + Send {
            send_wrapper::SendWrapper::new(async move {
                match res {
                    Err(err) => Err(err),
                    Ok(res) => Ok(T::from_response(res).await),
                }
            })
        }
    }

    impl<T: DeserializeOwned, E> ReqwestDecodeResult<T> for &&ReqwestDecoder<Result<T, E>> {
        fn decode_response(
            &self,
            res: Result<reqwest::Response, reqwest::Error>,
        ) -> impl Future<Output = Result<Result<T, ServerFnError>, reqwest::Error>> + Send {
            send_wrapper::SendWrapper::new(async move {
                match res {
                    Err(err) => Err(err),
                    Ok(res) => {
                        let bytes = res.bytes().await.unwrap();
                        let as_bytes = if bytes.is_empty() {
                            b"null".as_slice()
                        } else {
                            &bytes
                        };
                        let res = serde_json::from_slice::<T>(as_bytes);
                        match res {
                            Ok(t) => Ok(Ok(t)),
                            Err(e) => Ok(Err(ServerFnError::Deserialization(e.to_string()))),
                        }
                    }
                }
            })
        }
    }

    pub trait ReqwestDecodeErr<T, E> {
        fn decode_err(
            &self,
            res: Result<Result<T, ServerFnError>, reqwest::Error>,
        ) -> impl Future<Output = Result<T, E>> + Send;
    }

    impl<T, E: From<ServerFnError> + DeserializeOwned + Serialize> ReqwestDecodeErr<T, E>
        for &&&ReqwestDecoder<Result<T, E>>
    {
        fn decode_err(
            &self,
            res: Result<Result<T, ServerFnError>, reqwest::Error>,
        ) -> impl Future<Output = Result<T, E>> + Send {
            send_wrapper::SendWrapper::new(async move {
                match res {
                    Ok(Ok(res)) => Ok(res),
                    Ok(Err(e)) => Err(e.into()),
                    // todo: implement proper through-error conversion, instead of just ServerFnError::Request
                    // we should expand these cases.
                    Err(err) => Err(ServerFnError::Request {
                        message: err.to_string(),
                        code: err.status().map(|s| s.as_u16()),
                    }
                    .into()),
                }
            })
        }
    }

    /// Here we convert to ServerFnError and then into the anyhow::Error, letting the user downcast
    /// from the ServerFnError if they want to.
    ///
    /// This loses any actual type information, but is the most flexible for users.
    impl<T> ReqwestDecodeErr<T, anyhow::Error> for &&ReqwestDecoder<Result<T, anyhow::Error>> {
        fn decode_err(
            &self,
            res: Result<Result<T, ServerFnError>, reqwest::Error>,
        ) -> impl Future<Output = Result<T, anyhow::Error>> + Send {
            send_wrapper::SendWrapper::new(async move {
                match res {
                    Ok(Ok(res)) => Ok(res),
                    Ok(Err(e)) => Err(anyhow::Error::from(e)),
                    Err(err) => Err(anyhow::Error::from(ServerFnError::Request {
                        message: err.to_string(),
                        code: err.status().map(|s| s.as_u16()),
                    })),
                }
            })
        }
    }

    /// This converts to statuscode, which can be useful but loses a lot of information.
    impl<T> ReqwestDecodeErr<T, StatusCode> for &ReqwestDecoder<Result<T, StatusCode>> {
        fn decode_err(
            &self,
            res: Result<Result<T, ServerFnError>, reqwest::Error>,
        ) -> impl Future<Output = Result<T, StatusCode>> + Send {
            send_wrapper::SendWrapper::new(async move {
                match res {
                    Ok(Ok(res)) => Ok(res),
                    Ok(Err(e)) => {
                        //
                        match e {
                            // todo: we've caught the reqwest error here, so we should give it back in the form of a proper status code.
                            ServerFnError::Request { message, code } => {
                                Err(StatusCode::INTERNAL_SERVER_ERROR)
                            }

                            ServerFnError::ServerError(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                            ServerFnError::Registration(_) => {
                                Err(StatusCode::INTERNAL_SERVER_ERROR)
                            }
                            ServerFnError::UnsupportedRequestMethod(_) => {
                                Err(StatusCode::INTERNAL_SERVER_ERROR)
                            }

                            ServerFnError::MiddlewareError(_) => {
                                Err(StatusCode::INTERNAL_SERVER_ERROR)
                            }
                            ServerFnError::Deserialization(_) => {
                                Err(StatusCode::INTERNAL_SERVER_ERROR)
                            }
                            ServerFnError::Serialization(_) => {
                                Err(StatusCode::INTERNAL_SERVER_ERROR)
                            }
                            ServerFnError::Args(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                            ServerFnError::MissingArg(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                            ServerFnError::Response(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                        }
                    }
                    Err(_) => todo!(),
                }
            })
        }
    }

    /// This tries to catch http::Error and its subtypes, but will not catch everything that is normally "IntoResponse"
    impl<T, E> ReqwestDecodeErr<T, E> for ReqwestDecoder<Result<T, E>>
    where
        E: Into<http::Error>,
        E: From<http::Error>,
    {
        fn decode_err(
            &self,
            res: Result<Result<T, ServerFnError>, reqwest::Error>,
        ) -> impl Future<Output = Result<T, E>> + Send {
            send_wrapper::SendWrapper::new(async move {
                match res {
                    Ok(Ok(res)) => Ok(res),
                    Ok(Err(e)) => todo!(),
                    Err(_) => todo!(),
                }
            })
        }
    }
}
