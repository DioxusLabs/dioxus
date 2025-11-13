#![allow(clippy::manual_async_fn)]
#![allow(unused_variables)]

use anyhow::Result;
use axum::extract::FromRequest;
use axum::response::IntoResponse;
use axum::{response::Html, Json};
use bytes::Bytes;
use dioxus::prelude::*;
use dioxus_fullstack::{get, FileStream, ServerFnError, Text, TextStream, Websocket};
use futures::StreamExt;
use http::HeaderMap;
use http::StatusCode;
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use std::future::Future;

fn main() {}

mod simple_extractors {
    use super::*;

    /// We can extract the state and return anything thats IntoResponse
    #[get("/home")]
    async fn one() -> Result<String> {
        Ok("hello home".to_string())
    }

    /// We can extract the path arg and return anything thats IntoResponse
    #[get("/home/{id}")]
    async fn two(id: String) -> Result<String> {
        Ok(format!("hello home {}", id))
    }

    /// We can do basically nothing
    #[get("/")]
    async fn three() -> Result<()> {
        Ok(())
    }

    /// We can do basically nothing, with args
    #[get("/{one}/{two}?a&b&c")]
    async fn four(one: String, two: String, a: String, b: String, c: String) -> Result<()> {
        Ok(())
    }

    /// We can return anything that implements IntoResponse
    #[get("/hello")]
    async fn five() -> Result<Html<String>> {
        Ok(Html("<h1>Hello!</h1>".to_string()))
    }

    /// We can return anything that implements IntoResponse
    #[get("/hello")]
    async fn six() -> Result<Json<String>> {
        Ok(Json("Hello!".to_string()))
    }

    /// We can return our own custom `Text<T>` type for sending plain text
    #[get("/hello")]
    async fn six_2() -> Result<Text<String>> {
        Ok(Text("Hello!".to_string()))
    }

    /// We can return our own custom TextStream type for sending plain text streams
    #[get("/hello")]
    async fn six_3() -> Result<TextStream> {
        Ok(TextStream::new(futures::stream::iter(vec![
            "Hello 1".to_string(),
            "Hello 2".to_string(),
            "Hello 3".to_string(),
        ])))
    }

    /// We can return a Result with anything that implements IntoResponse
    #[get("/hello")]
    async fn seven() -> Result<Bytes> {
        Ok(Bytes::from_static(b"Hello!"))
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
    async fn elevent() -> Result<Bytes> {
        Ok(Bytes::from_static(b"Hello!"))
    }

    /// We can use multiple args that are Deserialize
    #[get("/hello")]
    async fn twelve(a: i32, b: i32, c: i32) -> Result<Bytes> {
        Ok(format!("Hello! {} {} {}", a, b, c).into())
    }

    // How should we handle generics? Doesn't make a lot of sense with distributed registration?
    // I think we should just not support them for now. Reworking it will be a big change though.
    //
    // /// We can use generics
    // #[get("/hello")]
    // async fn thirteen<S: Serialize + DeserializeOwned>(a: S) -> Result<Bytes> {
    //     Ok(format!("Hello! {}", serde_json::to_string(&a)?).into())
    // }

    // /// We can use impl-style generics
    // #[get("/hello")]
    // async fn fourteen(a: impl Serialize + DeserializeOwned) -> Result<Bytes> {
    //     Ok(format!("Hello! {}", serde_json::to_string(&a)?).into())
    // }
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
    async fn get_item1(
        id: i32,
        amount: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Json<YourObject>> {
        Ok(Json(YourObject { id, amount, offset }))
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
    use axum::response::Response;
    // use axum_core::response::Response;
    use dioxus_fullstack::{
        ClientRequest, ClientResponse, FromResponse, IntoRequest, RequestError, WebSocketOptions,
    };

    use super::*;

    /// We can extract the path arg and return anything thats IntoResponse
    #[get("/upload/image/")]
    async fn streaming_file(body: FileStream) -> Result<Json<i32>> {
        unimplemented!()
    }

    /// We can extract the path arg and return anything thats IntoResponse
    #[get("/upload/image/?name&size&ftype")]
    async fn streaming_file_args(
        name: String,
        size: usize,
        ftype: String,
        body: FileStream,
    ) -> Result<Json<i32>> {
        unimplemented!()
    }

    #[get("/")]
    async fn ws_endpoint(options: WebSocketOptions) -> Result<Websocket<String, String>> {
        unimplemented!()
    }

    struct MyCustomPayload {}
    impl FromResponse for MyCustomPayload {
        fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
            async move { Ok(MyCustomPayload {}) }
        }
    }
    impl IntoResponse for MyCustomPayload {
        fn into_response(self) -> Response {
            unimplemented!()
        }
    }
    impl<T> FromRequest<T> for MyCustomPayload {
        type Rejection = ServerFnError;
        #[allow(clippy::manual_async_fn)]
        fn from_request(
            _req: axum::extract::Request,
            _state: &T,
        ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
            async move { Ok(MyCustomPayload {}) }
        }
    }
    impl IntoRequest for MyCustomPayload {
        fn into_request(
            self,
            request_builder: ClientRequest,
        ) -> impl Future<Output = Result<ClientResponse, RequestError>> + 'static {
            async move { unimplemented!() }
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
            unimplemented!()
        }
    }
    impl<T> FromRequest<T> for MyCustomPayload {
        type Rejection = ServerFnError;
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
    use dioxus::Result;

    use super::*;

    /// Extract requests directly for full control
    #[get("/myendpoint")]
    async fn my_custom_handler1(request: axum::extract::Request) -> Result<()> {
        let mut data = request.into_data_stream();
        while let Some(chunk) = data.next().await {
            let _ = chunk.unwrap();
        }
        Ok(())
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
    async fn zero(a: (), b: (), c: ()) -> Result<()> {
        Ok(())
    }

    /// We can take `()` as input in serde types
    #[post("/")]
    async fn zero_1(a: Json<()>) -> Result<()> {
        Ok(())
    }

    /// We can take regular axum extractors as input
    #[post("/")]
    async fn one(data: Json<CustomPayload>) -> Result<()> {
        Ok(())
    }

    /// We can take Deserialize types as input, and they will be deserialized from JSON
    #[post("/")]
    async fn two(name: String, age: u32) -> Result<()> {
        Ok(())
    }

    /// We can take Deserialize types as input, with custom server extensions
    #[post("/", headers: HeaderMap)]
    async fn three(name: String) -> Result<()> {
        Ok(())
    }

    /// We can take a regular axum-like mix with extractors and Deserialize types
    #[post("/", headers: HeaderMap)]
    async fn four(data: Json<CustomPayload>) -> Result<()> {
        Ok(())
    }

    /// We can even accept string in the final position.
    #[post("/")]
    async fn five(age: u32, name: String) -> Result<()> {
        Ok(())
    }
}

mod handlers {
    use super::*;

    #[get("/handlers/get")]
    async fn handle_get() -> Result<String> {
        Ok("handled get".to_string())
    }

    #[post("/handlers/post")]
    async fn handle_post() -> Result<String> {
        Ok("handled post".to_string())
    }

    #[put("/handlers/put")]
    async fn handle_put() -> Result<String> {
        Ok("handled put".to_string())
    }

    #[patch("/handlers/patch")]
    async fn handle_patch() -> Result<String> {
        Ok("handled patch".to_string())
    }

    #[delete("/handlers/delete")]
    async fn handle_delete() -> Result<String> {
        Ok("handled delete".to_string())
    }
}
