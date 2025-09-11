use axum::{extract::FromRequestParts, Json};
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
}
