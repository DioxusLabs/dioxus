use anyhow::Result;
use axum::extract::FromRequest;
use axum::response::IntoResponse;
use axum::{extract::State, response::Html, Json};
use bytes::Bytes;
use dioxus::prelude::*;
use dioxus_fullstack::req_from::{DeSer, ExtractRequest, ExtractState};
use dioxus_fullstack::{
    fetch::{FileUpload, WebSocket},
    DioxusServerState, ServerFnRejection, ServerFnSugar, ServerFunction,
};
use futures::StreamExt;
use http::HeaderMap;
use http::StatusCode;
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use std::prelude::rust_2024::Future;

fn main() {}

/// Extract regular axum endpoints
#[get("/myendpoint")]
async fn my_custom_handler1(request: axum::extract::Request) {
    // let mut data = request.into_data_stream();
    // while let Some(chunk) = data.next().await {
    //     let _ = chunk.unwrap();
    // }
}
