use anyhow::Result;
use std::{
    any::TypeId,
    marker::PhantomData,
    prelude::rust_2024::{Future, IntoFuture},
    process::Output,
};

use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::MethodRouter,
    Json,
};
use bytes::Bytes;
use dioxus::prelude::*;
use dioxus_fullstack::{
    fetch::{FileUpload, WebSocket},
    route, DioxusServerState, ServerFnSugar, ServerFunction,
};
use http::Method;
use reqwest::RequestBuilder;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;

#[get("/play")]
async fn go_play() -> Html<&'static str> {
    Html("hello play")
}
