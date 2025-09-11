use anyhow::Result;
use axum::extract::FromRequest;
use axum::response::IntoResponse;
use axum::{extract::State, response::Html, Json};
use bytes::Bytes;
use dioxus::prelude::*;
use dioxus_fullstack::fromreq::{DeSer, ExtractRequest, ExtractState};
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

    /// We can use mutliple args that are Deserialize
    #[get("/hello")]
    async fn twelve(a: i32, b: i32, c: i32) -> Result<Bytes, http::Error> {
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

    /// We can extract the path arg and return anything thats IntoResponse
    #[get("/upload/image/?name&size&ftype")]
    async fn streaming_file_args(
        name: String,
        size: usize,
        ftype: String,
        body: FileUpload,
    ) -> Result<Json<i32>> {
        todo!()
    }

    #[get("/")]
    async fn ws_endpoint() -> Result<WebSocket<String, String>> {
        todo!()
    }

    struct MyCustomPayload {}
    impl IntoResponse for MyCustomPayload {
        fn into_response(self) -> axum::response::Response {
            todo!()
        }
    }
    impl<T> FromRequest<T> for MyCustomPayload {
        type Rejection = ServerFnRejection;
        #[allow(clippy::manual_async_fn)]
        fn from_request(
            _req: axum::extract::Request,
            _state: &T,
        ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
            async move { Ok(MyCustomPayload {}) }
        }
    }

    #[get("/myendpoint")]
    async fn my_custom_handler1(payload: MyCustomPayload) -> Result<MyCustomPayload> {
        Ok(payload)
    }

    #[get("/myendpoint2")]
    async fn my_custom_handler2(payload: MyCustomPayload) -> Result<MyCustomPayload, StatusCode> {
        Ok(payload)
    }
}

mod overlap {
    use super::*;

    #[derive(Serialize, Deserialize)]
    struct MyCustomPayload {}
    impl IntoResponse for MyCustomPayload {
        fn into_response(self) -> axum::response::Response {
            todo!()
        }
    }
    impl<T> FromRequest<T> for MyCustomPayload {
        type Rejection = ServerFnRejection;
        #[allow(clippy::manual_async_fn)]
        fn from_request(
            _req: axum::extract::Request,
            _state: &T,
        ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
            async move { Ok(MyCustomPayload {}) }
        }
    }

    /// When we have overlapping serialize + IntoResponse impls, the autoref logic will only pick Serialize
    /// if IntoResponse is not available. Otherwise, IntoResponse is preferred.
    #[get("/myendpoint")]
    async fn my_custom_handler3(payload: MyCustomPayload) -> Result<MyCustomPayload, StatusCode> {
        Ok(payload)
    }

    /// Same, but with the anyhow::Error path
    #[get("/myendpoint")]
    async fn my_custom_handler4(payload: MyCustomPayload) -> Result<MyCustomPayload> {
        Ok(payload)
    }
}

mod http_ext {
    use super::*;

    /// Extract regular axum endpoints
    #[get("/myendpoint")]
    async fn my_custom_handler1(request: axum::extract::Request) {
        let mut data = request.into_data_stream();
        while let Some(chunk) = data.next().await {
            let _ = chunk.unwrap();
        }
    }

    #[get("/myendpoint")]
    async fn my_custom_handler2(_state: State<DioxusServerState>, request: axum::extract::Request) {
        let mut data = request.into_data_stream();
        while let Some(chunk) = data.next().await {
            let _ = chunk.unwrap();
        }
    }
}

mod input_types {

    use super::*;

    #[derive(Serialize, Deserialize)]
    struct CustomPayload {
        name: String,
        age: u32,
    }

    /// We can take `()` as input
    #[post("/")]
    async fn zero(a: (), b: (), c: ()) {}

    /// We can take `()` as input
    #[post("/")]
    async fn zero_1(a: Json<CustomPayload>) {}

    /// We can take regular axum extractors as input
    #[post("/")]
    async fn one(data: Json<CustomPayload>) {}

    /// We can take Deserialize types as input, and they will be deserialized from JSON
    #[post("/")]
    async fn two(name: String, age: u32) {}

    /// We can take Deserialize types as input, with custom server extensions
    #[post("/", headers: HeaderMap)]
    async fn three(name: String, age: u32) {}

    /// We can take a regular axum-like mix with extractors and Deserialize types
    #[post("/")]
    async fn four(headers: HeaderMap, data: Json<CustomPayload>) {}
}
