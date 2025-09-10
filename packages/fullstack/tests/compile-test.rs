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
    route, serverfn_sugar, DioxusServerState, ServerFunction,
};
use http::{Method, StatusCode};
use reqwest::RequestBuilder;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;

#[tokio::main]
async fn main() {}

mod simple_extractors {
    use super::*;

    /// We can extract the state and return anything thats IntoResponse
    #[get("/home")]
    async fn one(state: State<DioxusServerState>) -> String {
        "hello home".to_string()
    }

    /// We can extract the path arg and return anything thats IntoResponse
    #[get("/home/{id}")]
    async fn two(id: String) -> String {
        format!("hello home {}", id)
    }

    /// We can do basically nothing
    #[get("/")]
    async fn three() {}

    /// We can do basically nothing, with args
    #[get("/{one}/{two}?a&b&c")]
    async fn four(one: String, two: String, a: String, b: String, c: String) {}

    /// We can return anything that implements IntoResponse
    #[get("/hello")]
    async fn five() -> Html<&'static str> {
        Html("<h1>Hello!</h1>")
    }

    /// We can return anything that implements IntoResponse
    #[get("/hello")]
    async fn six() -> Json<&'static str> {
        Json("Hello!")
    }

    /// We can return a Result with anything that implements IntoResponse
    #[get("/hello")]
    async fn seven() -> Bytes {
        Bytes::from_static(b"Hello!")
    }

    /// We can return a Result with anything that implements IntoResponse
    #[get("/hello")]
    async fn eight() -> Result<Bytes, StatusCode> {
        Ok(Bytes::from_static(b"Hello!"))
    }

    /// We can use the anyhow error type
    #[get("/hello")]
    async fn nine() -> Result<Bytes> {
        Ok(Bytes::from_static(b"Hello!"))
    }

    /// We can use the ServerFnError error type
    #[get("/hello")]
    async fn ten() -> Result<Bytes, ServerFnError> {
        Ok(Bytes::from_static(b"Hello!"))
    }

    /// We can use the ServerFnError error type
    #[get("/hello")]
    async fn elevent() -> Result<Bytes, http::Error> {
        Ok(Bytes::from_static(b"Hello!"))
    }
}

mod custom_serialize {
    use super::*;

    #[derive(Serialize, Deserialize)]
    struct YourObject {
        id: i32,
        amount: Option<i32>,
        offset: Option<i32>,
    }

    /// Directly return the object, and it will be serialized to JSON
    #[get("/item/{id}?amount&offset")]
    async fn get_item1(id: i32, amount: Option<i32>, offset: Option<i32>) -> Json<YourObject> {
        Json(YourObject { id, amount, offset })
    }

    #[get("/item/{id}?amount&offset")]
    async fn get_item2(
        id: i32,
        amount: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Json<YourObject>> {
        Ok(Json(YourObject { id, amount, offset }))
    }

    #[get("/item/{id}?amount&offset")]
    async fn get_item3(id: i32, amount: Option<i32>, offset: Option<i32>) -> Result<YourObject> {
        Ok(YourObject { id, amount, offset })
    }

    #[get("/item/{id}?amount&offset")]
    async fn get_item4(
        id: i32,
        amount: Option<i32>,
        offset: Option<i32>,
    ) -> Result<YourObject, StatusCode> {
        Ok(YourObject { id, amount, offset })
    }
}

mod custom_types {
    use super::*;

    /// We can extract the path arg and return anything thats IntoResponse
    #[get("/upload/image/")]
    async fn streaming_file(body: FileUpload) -> Result<Json<i32>> {
        todo!()
    }

    #[get("/")]
    async fn ws_endpoint() -> Result<WebSocket<String, String>> {
        todo!()
    }
}
