use axum::{
    extract::{FromRequest, FromRequestParts, Request},
    handler::Handler,
    Json, RequestExt,
};
use dioxus_fullstack::fromreq::{DeSer, ExtractRequest, ExtractState};
use dioxus_fullstack::DioxusServerState;
use http::HeaderMap;
use serde::de::DeserializeOwned;

#[allow(clippy::needless_borrow)]
#[tokio::main]
async fn main() {
    let req = (&&&&&&&&&&&&&&DeSer::<(HeaderMap,), _>::new())
        .extract(ExtractState::default())
        .await;

    let req = (&&&&&&&&&&&&&&DeSer::<(HeaderMap, i32), _>::new())
        .extract(ExtractState::default())
        .await;

    let req = (&&&&&&&&&&&&&&DeSer::<(HeaderMap, i32, i32), _>::new())
        .extract(ExtractState::default())
        .await;

    let req = (&&&&&&&&&&&&&&DeSer::<(HeaderMap, Json<String>), _>::new())
        .extract(ExtractState::default())
        .await;

    let req = (&&&&&&&&&&&&&&DeSer::<(HeaderMap, Request), _>::new())
        .extract(ExtractState::default())
        .await;

    // struct ServerFnBody;
    // impl<S> FromRequest<S> for ServerFnBody {
    //     type Rejection = ();

    //     async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
    //         Ok(ServerFnBody)
    //     }
    // }
}

// #[axum::debug_handler]
// async fn my_handler(t: (HeaderMap, HeaderMap, Json<String>, Json<String>)) {}

// #[axum::debug_handler]
// async fn my_handler2(a: HeaderMap, b: HeaderMap, c: Json<String>, d: ()) {}

// // fn test<M, T: Handler<M, ()>>(_: T) -> M {
// //     todo!()
// // }

// fn check() {
//     let a = test(my_handler);
// }
