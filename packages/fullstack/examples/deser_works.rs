use std::prelude::rust_2024::Future;

use axum::{
    extract::{FromRequest, FromRequestParts, Request},
    handler::Handler,
    Json, RequestExt,
};
use dioxus_fullstack::DioxusServerState;
use dioxus_fullstack::{
    fromreq::{DeSer, DeTys, ExtractRequest, ExtractState},
    DioxusServerContext,
};
use http::HeaderMap;
use serde::de::DeserializeOwned;

#[allow(clippy::needless_borrow)]
#[tokio::main]
async fn main() {
    let (a,) = (&&&&&&&&&&&&&&DeSer::<(HeaderMap,), _>::new())
        .extract(ExtractState::default())
        .await;

    let (a, b) = (&&&&&&&&&&&&&&DeSer::<(HeaderMap, i32), _>::new())
        .extract(ExtractState::default())
        .await;

    let req = (&&&&&&&&&&&&&&DeSer::<(HeaderMap, Json<String>), _>::new())
        .extract(ExtractState::default())
        .await;

    let req = (&&&&&&&&&&&&&&DeSer::<(HeaderMap, Request), _>::new())
        .extract(ExtractState::default())
        .await;

    let (a, b, c) = (&&&&&&&&&&&&&&DeSer::<(HeaderMap, i32, i32), _>::new())
        .extract(ExtractState::default())
        .await;

    let handler: fn(_, _, _) -> _ = |a, b, c| async move { todo!() };
    let p = || handler(a, b, c);

    axum::Router::<DioxusServerState>::new().route("/", axum::routing::get(handler));

    // axum::routing::get(|a: HeaderMap, b: (), c: DeTys<(String,)>| async move { "hello" }),

    // impl<S> FromRequest<S> for ServerFnBody {
    //     type Rejection = ();

    //     async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
    //         Ok(ServerFnBody)
    //     }
    // }
}

fn return_handler() {}

// #[axum::debug_handler]
// async fn my_handler(t: (HeaderMap, HeaderMap, Json<String>, Json<String>)) {}

#[axum::debug_handler]
async fn my_handler2(a: HeaderMap, b: HeaderMap, c: (), d: Json<String>) {}

#[axum::debug_handler]
async fn my_handler23(a: HeaderMap, b: HeaderMap, c: (), d: (), e: Json<String>) {}

// // fn test<M, T: Handler<M, ()>>(_: T) -> M {
// //     todo!()
// // }

// fn check() {
//     let a = test(my_handler);
// }

fn hmm() {
    struct Wrapped<T>(T);

    trait Indexed {
        type First;
        type Second;
        type Third;
        async fn handler(a: Self::First, b: Self::Second, c: Self::Third);
    }

    impl<A, B, C> Indexed for Wrapped<(A, B, C)> {
        type First = A;
        type Second = B;
        type Third = C;
        async fn handler(a: Self::First, b: Self::Second, c: Self::Third) {}
    }

    impl<F, T, G> Indexed for G
    where
        F: Future<Output = T>,
        T: Indexed,
        G: Fn() -> F,
    {
        type First = T::First;
        type Second = T::Second;
        type Third = T::Third;
        async fn handler(a: Self::First, b: Self::Second, c: Self::Third) {}
    }

    async fn make_thing() -> impl Indexed {
        Wrapped(
            (&&&&&&&&&&&&&&DeSer::<(HeaderMap, HeaderMap, Request), _>::new())
                .extract(ExtractState::default())
                .await,
        )
    }

    fn type_of<T>(_: T) -> T {
        todo!()
    }

    // <make_thing as Indexed>::handler(HeaderMap::new(), HeaderMap::new(), Request::new(()))

    // struct ServerFnBody;
}
