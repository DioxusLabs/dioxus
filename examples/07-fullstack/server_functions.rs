//! This example is a simple showcase of Dioxus Server Functions.
//!
//! The other examples in this folder showcase advanced features of server functions like custom
//! data types, error handling, websockets, and more.
//!
//! This example is meant to just be a simple starting point to show how server functions work.
//!
//! ## Server Functions
//!
//! In Dioxus, Server Functions are `axum` backend endpoints that can be called directly from the client
//! as if you were simply calling a local Rust function. You can do anything with a server function
//! that an Axum handler can do like extracting path, query, headers, and body parameters.
//!
//! ## Server Function Arguments
//!
//! Unlike Axum handlers, the arguments of the server functions have some special magic enabled by
//! the accompanying `#[get]`/`#[post]` attributes. This magic enables you to choose between
//! arguments that are purely serializable (i.e. `String`, `i32`, `Vec<T>`, etc) as the JSON body of
//!
//! the request *or* arguments that implement Axum's `FromRequest` trait. This magic enables simple
//! RPC functions but also complex extractors for things like auth, sessions, cookies, and more.
//!
//! ## Server Function Return Types
//!
//! The return type of the server function is also somewhat magical. Unlike Axum handlers, all server
//! functions must return a `Result` type, giving the client an opportunity to handle errors properly.
//!
//! The `Ok` type can be anything that implements `Serialize + DeserializeOwned` so it can be sent
//! to the client as JSON, or it can be anything that implements `IntoResponse` just like an Axum handler.
//!
//! ## Error Types
//!
//! The `Err` type of the server function return type is also somewhat special. The `Err` type can be:
//! - `anyhow::Error` (the `dioxus_core::Err` type alias) for untyped errors with rich context. Note
//!   that these errors will always downcast to `ServerFnError` on the client, losing the original
//!   error stack and type.
//! - `ServerFnError` for typed errors with a status code and optional message.
//! - `StatusCode` for returning raw HTTP status codes.
//! - `HttpError` for returning HTTP status codes with custom messages.
//! - Any custom errors that implement `From<ServerFnError>` and are `Serialize`/`Deserialize`
//!
//! The only way to set the HTTP status code of the response is to use one of the above error types,
//! or to implement a custom `IntoResponse` type that sets the status code manually.
//!
//! The `anyhow::Error` type is the best choice for rapid development, but is somewhat limited when
//! handling specific error cases on the client since all errors are downcast to `ServerFnError`.
//!
//! ## Calling Server Functions from the Client
//!
//! Server functions can be called from the client by simply importing the function and calling it
//! like a normal Rust function. Unlike regular axum handlers, Dioxus server functions have a few
//! non-obvious restrictions.
//!
//! Most importantly, the arguments to the server function must implement either `Deserialize` *or*
//! `IntoRequest`. The `IntoRequest` trait is a Dioxus abstraction that represents the "inverse" of the
//! Axum `FromRequest` trait. Anything that is sent to the server from the client must be both extractable
//! with `FromRequest` on the server *and* constructible with `IntoRequest` on the client.
//!
//! Types like `WebsocketOptions` implement `IntoRequest` and pass along things like upgrade headers
//! to the server so that the server can properly upgrade the connection.
//!
//! When receiving data from the server, the return type must implement `Deserialize` *or* `FromResponse`.
//! The `FromResponse` trait is the inverse of Axum's `IntoResponse` trait, and is implemented
//! for types like `Websocket` where the raw HTTP response is needed to complete the construction
//! of the type.
//!
//! ## Server-only Extractors
//!
//! Because the arguments of the server function define the structure of the public API, some extractors
//! might not make sense to expose directly, nor would they be possible to construct on the client.
//! For example, on the web, you typically don't work directly with cookies since the browser handles
//! them for you. In these cases, the client would omit the `Cookie` header entirely, and we would need
//! "hoist" our extractor into a "server-only extractor".
//!
//! Server-only extractors are function arguments placed after the path in the `#[get]`/`#[post]` attribute.
//! These arguments are extracted on the server, but not passed in from the client. This lets the
//! server function remain callable from the client, while still allowing full access to axum's
//! extractors.
//!
//! ```
//! #[post("/api/authenticate", auth: AuthCookie)]
//! async fn authenticate() -> Result<User> { /* ... */ }
//! ```
//!
//! ## Automatic Registration
//!
//! Unlike axum handlers, server functions do not need to be manually registered with a router.
//! By default, *all* server functions in your app will be automatically registered with the
//! server when you call `dioxus::launch` or create a router manually with `dioxus::server::router()`.
//!
//! However, not all server functions are automatically registered by default. Server functions that
//! take a `State<T>` extractor cannot be automatically added to the router since the dioxus router
//! type does not know how to construct the `T` type.
//!
//! These server functions will be registered once the `ServerState<T>` layer is added to the app with
//! `router = router.layer(ServerState::new(your_state))`.
//!
//! ## Middleware
//!
//! Middleware can be added to server functions using the `#[middleware(MiddlewareType)]` attribute.
//! Middleware will be applied in the order they are specified, and will be applied before any
//! server-only extractors.
//!
//! To add router-level middleware, you can customize the axum `Router` using layers and extensions
//! as you would in a normal axum app.
//!
//! ## Anonymous Server Functions
//!
//! The `#[server]` attribute can be used without a path to create an anonymous server function.
//! These functions are still exposed as HTTP endpoints, but their names are procedurally generated
//! from the module path, function name, and a hash of the function signature. This makes it hard to
//! call these functions with `curl` or `postman`, but save you the trouble of coming up with unique
//! names for simple functions that are only called from your Dioxus app.
//!
//! If you're shipping desktop/mobile apps, we don't recommend using anonymous server functions
//! since the function names could change between builds and thus make older versions of your app
//! incompatible with newer versions of your server.
//!
//! ## Cross-platform Clients
//!
//! Server functions can be called from any platform (web, desktop mobile, etc) and use the best
//! underlying `fetch` implementation available.
//!
//! ## More examples
//!
//! With Dioxus Fullstack 0.7, pretty much anything you can do with an Axum handler, you can do with
//! a server function. More advanced examples can be found in this folder showcasing custom data types,
//! error handling, websockets, and more.

use axum_core::response::IntoResponse;
use dioxus::prelude::*;
use dioxus_fullstack::FromResponse;
use dioxus_fullstack::http::StatusCode;
use serde::{Deserialize, Serialize};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut echo_action = use_action(echo);
    let mut chat_action = use_action(chat);
    let mut dog_data = use_action(get_data);
    let mut custom_data = use_action(get_custom_data);
    let mut anonymous_action = use_action(anonymous);
    let mut custom_anonymous_action = use_action(custom_anonymous);
    let mut custom_response_action = use_action(get_custom_response);

    rsx! {
        h1 { "Server Functions Example" }
        div {
            display: "flex",
            flex_direction: "column",
            gap: "8px",

            button { onclick: move |_| echo_action.call("Hello from client".into()), "Echo: Hello" }
            button { onclick: move |_| chat_action.call(42u32, Some(7u32)), "Chat (user 42, room 7)" }
            button { onclick: move |_| dog_data.call(), "Get dog data" }
            button { onclick: move |_| custom_data.call(), "Get custom data" }
            button { onclick: move |_| anonymous_action.call(), "Call anonymous" }
            button { onclick: move |_| custom_anonymous_action.call(), "Call custom anonymous" }
            button { onclick: move |_| custom_response_action.call(), "Get custom response" }

            button {
                onclick: move |_| {
                    echo_action.reset();
                    chat_action.reset();
                    dog_data.reset();
                    custom_data.reset();
                    anonymous_action.reset();
                    custom_anonymous_action.reset();
                    custom_response_action.reset();
                },
                "Clear results"
            }

            pre { "Echo result: {echo_action.value():#?}" }
            pre { "Chat result: {chat_action.value():#?}" }
            pre { "Dog data: {dog_data.value():#?}" }
            pre { "Custom data: {custom_data.value():#?}" }
            pre { "Anonymous: {anonymous_action.value():#?}" }
            pre { "Custom anonymous: {custom_anonymous_action.value():#?}" }
            pre { "Custom response: {custom_response_action.value():#?}" }
        }
    }
}

/// A plain server function at a `POST` endpoint that takes a string and returns it.
/// Here, we use the `Result` return type which is an alias to `Result<T, anyhow::Error>`.
#[post("/api/echo")]
async fn echo(body: String) -> Result<String> {
    Ok(body)
}

/// A Server function that takes path and query parameters, as well as a server-only extractor.
#[post("/api/{user_id}/chat?room_id", headers: dioxus_fullstack::HeaderMap)]
async fn chat(user_id: u32, room_id: Option<u32>) -> Result<String> {
    Ok(format!(
        "User ID: {}, Room ID: {} - Headers: {:#?}",
        user_id,
        room_id.map_or("None".to_string(), |id| id.to_string()),
        headers
    ))
}

/// A plain server function at a `GET` endpoint that returns some JSON data. Because `DogData` is
/// `Serialize` and `Deserialize`, it can be sent to the client as JSON automatically.
///
/// You can `curl` this endpoint and it will return a 200 status code with a JSON body:
///
/// ```json
/// {
///   "name": "Fido",
///   "age": 4
/// }
/// ```
#[get("/api/dog")]
async fn get_data() -> Result<DogData> {
    Ok(DogData {
        name: "Fido".to_string(),
        age: 4,
    })
}

#[derive(Serialize, Deserialize, Debug)]
struct DogData {
    name: String,
    age: u8,
}

/// A server function that returns a custom struct as JSON.
#[get("/api/custom")]
async fn get_custom_data() -> Result<CustomData> {
    Ok(CustomData {
        message: "Hello from the server!".to_string(),
    })
}

#[derive(Debug)]
struct CustomData {
    message: String,
}
impl IntoResponse for CustomData {
    fn into_response(self) -> axum_core::response::Response {
        axum_core::response::Response::builder()
            .status(StatusCode::ACCEPTED)
            .body(serde_json::to_string(&self.message).unwrap().into())
            .unwrap()
    }
}

impl FromResponse for CustomData {
    async fn from_response(res: dioxus_fullstack::ClientResponse) -> Result<Self, ServerFnError> {
        let message = res.json::<String>().await?;
        Ok(CustomData { message })
    }
}

/// A server function that returns an axum type directly.
///
/// When make these endpoints, we need to use the `axum::response::Response` type and then call `into_response`
/// on the return value to convert it into a response.
#[get("/api/custom_response")]
async fn get_custom_response() -> Result<axum_core::response::Response> {
    Ok(axum_core::response::Response::builder()
        .status(StatusCode::CREATED)
        .body("Created!".to_string())
        .unwrap()
        .into_response())
}

/// An anonymous server function - the url path is generated from the module path and function name.
///
/// This will end up as `/api/anonymous_<hash>` where `<hash>` is a hash of the function signature.
#[server]
async fn anonymous() -> Result<String> {
    Ok("Hello from an anonymous server function!".to_string())
}

/// An anonymous server function with a custom prefix and a fixed endpoint name.
///
/// This is less preferred over the `#[get]`/`#[post]` syntax but is still functional for backwards
/// compatibility. Previously, only the `#[server]` attribute was available, but as of Dioxus 0.7,
/// the `#[get]`/`#[post]` attributes are preferred for new code.
///
/// You can also use server-only extractors here as well, provided they come after the configuration.
#[server(prefix = "/api/custom", endpoint = "my_anonymous", headers: dioxus_fullstack::HeaderMap)]
async fn custom_anonymous() -> Result<String> {
    Ok(format!(
        "Hello from a custom anonymous server function! -> {:#?}",
        headers
    ))
}
