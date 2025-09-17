use std::{
    any::{type_name, TypeId},
    pin::Pin,
    prelude::rust_2024::Future,
};

use axum::Json;
use dioxus_fullstack_core::ServerFnError;
use futures::FutureExt;
use serde::de::DeserializeOwned;

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
        res: axum_core::response::Response,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send;
}

impl<T> FromResponse for Json<T> {
    fn from_response(
        res: axum_core::response::Response,
    ) -> impl Future<Output = Result<Self, ServerFnError>> + Send {
        async move { todo!() }
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
        use serde::{ser::Serialize as DeO_____, Serialize};
        use dioxus_fullstack_core::DioxusServerState as Dsr;
        use dioxus_fullstack_core::ServerFnError as Sfe;
        use serde_json::json;


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
        impl<O, E> EncodeRequest for &&&&&&&&&&ClientRequest<(), Result<O, E>> where E: From<Sfe>, O: FromResponse {
            type Input = ();
            type Output = Result<O, E>;
            fn fetch(&self, ctx: EncodeState, _: Self::Input) -> impl Future<Output = Self::Output> + Send + 'static {
                send_wrapper::SendWrapper::new(async move {
                    let res = ctx.client.send().await;
                    // let res = ctx.client.body(serde_json::to_string(&json!({})).unwrap()).send().await;
                    match res {
                        Ok(res) => {
                            todo!()
                            // let
                            // O::from_response(res).await.map_err(|e| e.into())
                        },
                        Err(err) => Err(Sfe::Request { message: err.to_string(), code: err.status().map(|s| s.as_u16()) }.into())
                    }
                })
            }
        }

        // One-arg case
        impl<A, O, E> EncodeRequest for &&&&&&&&&&ClientRequest<(A,), Result<O, E>> where A: DeO_____ + Serialize + 'static, E: From<Sfe>, O: FromResponse {
            type Input = (A,);
            type Output = Result<O, E>;
            fn fetch(&self, ctx: EncodeState, data: Self::Input) -> impl Future<Output = Self::Output> + Send + 'static {
                send_wrapper::SendWrapper::new(async move {
                    let (a,) = data;
                    #[derive(Serialize)]
                    struct SerOne<A> {
                        data: A,
                    }

                    let res = ctx.client.body(serde_json::to_string(&SerOne { data: a }).unwrap()).send().await;
                    match res {
                        Ok(res) => {
                            todo!()
                            // O::from_response(res).await.map_err(|e| e.into())
                        },
                        Err(err) => Err(Sfe::Request { message: err.to_string(), code: err.status().map(|s| s.as_u16()) }.into())
                    }
                })
            }
        }
        impl<A, O, E> EncodeRequest for &&&&&&&&&ClientRequest<(A,), Result<O, E>> where A: Freq<Dsr>, E: From<Sfe> {
            type Input = (A,);
            type Output = Result<O, E>;
            fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Self::Output> + Send + 'static {
                async move { todo!() }
            }
        }
        impl<A, O, E> EncodeRequest for  &&&&&&&&ClientRequest<(A,), Result<O, E>> where A: Prts<Dsr>, E: From<Sfe> {
            type Input = (A,);
            type Output = Result<O, E>;
            fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Self::Output> + Send + 'static {
                async move { todo!() }
            }
        }


        // Two-arg case
        impl<A, B, O, E> EncodeRequest for &&&&&&&&&&ClientRequest<(A, B), Result<O, E>> where A: Prts<Dsr>, B: Freq<Dsr>, E: From<Sfe> {
            type Input = (A, B);
            type Output = Result<O, E>;
            fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Self::Output> + Send + 'static {
                async move { todo!() }
            }
        }
        impl<A, B, O, E> EncodeRequest for  &&&&&&&&&ClientRequest<(A, B), Result<O, E>> where A: Prts<Dsr>, B: Prts<Dsr>, E: From<Sfe> {
            type Input = (A, B);
            type Output = Result<O, E>;
            fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Self::Output> + Send + 'static {
                async move { todo!() }
            }
        }
        impl<A, B, O, E> EncodeRequest for   &&&&&&&&ClientRequest<(A, B), Result<O, E>> where A: Prts<Dsr>, B: DeO_____, E: From<Sfe> {
            type Input = (A, B);
            type Output = Result<O, E>;
            fn fetch(&self, _ctx: EncodeState, data: Self::Input) -> impl Future<Output = Self::Output> + Send + 'static {
                async move { todo!() }
            }
        }
        impl<A, B, O, E> EncodeRequest for    &&&&&&&ClientRequest<(A, B), Result<O, E>> where A: DeO_____, B: DeO_____, E: From<Sfe> {
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
        pub request: Request,
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
        use bytes::Bytes;
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
        impl<A> ExtractRequest for &&&&&&&&&&DeSer<(A,)> where A: DeO_____ {
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
        impl<A> ExtractRequest for  &&&&&&&&&DeSer<(A,)> where A: Freq<Ds> {
            type Output = (A,);
            fn extract(&self, _ctx: ExtractState) -> impl Future<Output = Result<Self::Output, ServerFnRejection>> + Send + 'static { async move { todo!() } }
        }
        impl<A> ExtractRequest for   &&&&&&&&DeSer<(A,)> where A: Prts<Ds> {
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

pub use resp::*;
mod resp {
    use axum::response::IntoResponse;
    use dioxus_fullstack_core::ServerFnError;
    use serde::{de::DeserializeOwned, Serialize};

    use crate::FromResponse;

    pub struct ResDeser<I> {
        _p: std::marker::PhantomData<I>,
    }

    impl<I> ResDeser<I> {
        pub fn new() -> Self {
            Self {
                _p: std::marker::PhantomData,
            }
        }
    }

    /// A trait for converting the result of the Server Function into an Axum response.
    /// This is to work around the issue where we want to return both Deserialize types and FromResponse types.
    /// Stuff like websockets
    pub trait FromResIt {
        type Output;
        type Input;
        fn make_axum_response(self, s: Self::Input) -> Self::Output;
    }

    impl<T, E> FromResIt for &&ResDeser<Result<T, E>>
    where
        T: FromResponse,
        E: From<ServerFnError>,
    {
        type Input = Result<T, E>;
        type Output = axum::response::Response;
        fn make_axum_response(self, s: Self::Input) -> Self::Output {
            todo!()
        }
    }

    impl<T, E> FromResIt for &ResDeser<Result<T, E>>
    where
        T: DeserializeOwned + Serialize,
        E: From<ServerFnError>,
    {
        type Output = axum::response::Response;
        type Input = Result<T, E>;
        fn make_axum_response(self, s: Self::Input) -> Self::Output {
            todo!()
            //         send_wrapper::SendWrapper::new(async move {
            //             let bytes = res.bytes().await.unwrap();
            //             let as_str = String::from_utf8_lossy(&bytes);
            //             tracing::info!(
            //                 "Response bytes: {:?} for type {:?} ({})",
            //                 as_str,
            //                 TypeId::of::<T>(),
            //                 type_name::<T>()
            //             );

            //             let bytes = if bytes.is_empty() {
            //                 b"null".as_slice()
            //             } else {
            //                 &bytes
            //             };

            //             let res = serde_json::from_slice::<T>(&bytes);

            //             match res {
            //                 Err(err) => Err(ServerFnError::Deserialization(err.to_string())),
            //                 Ok(res) => Ok(res),
            //             }
        }
    }
}
