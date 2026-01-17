//! An example of handling errors from server functions.
//!
//! This example showcases a few important error handling patterns when using Dioxus Fullstack.
//!
//! Run with:
//!
//! ```sh
//! dx serve --web
//! ```
//!
//! What this example shows:
//! - You can return `anyhow::Result<T>` (i.e. `Result<T>` without an `E`) for
//!   untyped errors with rich context (converted to HTTP 500 responses by default).
//! - You can return `Result<T, E>` where `E` is one of:
//!   - `HttpError` (convenience for returning HTTP status codes)
//!   - `StatusCode` (return raw status codes)
//!   - a custom error type that implements `From<ServerFnError>` or
//!     is `Serialize`/`Deserialize` so it can be sent to the client.
//! - This file demonstrates external API errors, custom typed errors, explicit
//!   HTTP errors, and basic success cases. The UI uses `use_action` to call
//!   server functions and shows loading/result states simply.
//!
//! Try running requests against the endpoints directly with `curl` or `postman` to see the actual HTTP responses!

use dioxus::fullstack::{AsStatusCode, Json, StatusCode};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

fn main() {
    dioxus::launch(|| {
        let mut dog_data = use_action(get_dog_data);
        let mut dog_data_err = use_action(get_dog_data_err);
        let mut ip_data = use_action(get_ip_data);
        let mut custom_data = use_action(move || {
            get_custom_encoding(Json(serde_json::json!({
                "example": "data",
                "number": 123,
                "array": [1, 2, 3],
            })))
        });
        let mut error_data = use_action(get_throws_error);
        let mut typed_error_data = use_action(get_throws_typed_error);
        let mut throws_ok_data = use_action(get_throws_ok);
        let mut http_error_data = use_action(throws_http_error);
        let mut http_error_context_data = use_action(throws_http_error_context);

        rsx! {
            button { onclick: move |_| { dog_data.call(); }, "Fetch dog data" }
            button { onclick: move |_| { ip_data.call(); }, "Fetch IP data" }
            button { onclick: move |_| { custom_data.call(); }, "Fetch custom encoded data" }
            button { onclick: move |_| { error_data.call(); }, "Fetch error data" }
            button { onclick: move |_| { typed_error_data.call(); }, "Fetch typed error data" }
            button { onclick: move |_| { dog_data_err.call(); }, "Fetch dog error data" }
            button { onclick: move |_| { throws_ok_data.call(); }, "Fetch throws ok data" }
            button { onclick: move |_| { http_error_data.call(); }, "Fetch HTTP 400" }
            button { onclick: move |_| { http_error_context_data.call(); }, "Fetch HTTP 400 (context)" }
            button {
                onclick: move |_| {
                    ip_data.reset();
                    dog_data.reset();
                    custom_data.reset();
                    error_data.reset();
                    typed_error_data.reset();
                    dog_data_err.reset();
                    throws_ok_data.reset();
                    http_error_data.reset();
                    http_error_context_data.reset();
                },
                "Clear data"
            }
            div { display: "flex", flex_direction: "column", gap: "8px",
                pre { "Dog data: {dog_data.value():#?}" }
                pre { "IP data: {ip_data.value():#?}" }
                pre { "Custom encoded data: {custom_data.value():#?}" }
                pre { "Error data: {error_data.value():#?}" }
                pre { "Typed error data: {typed_error_data.value():#?}" }
                pre { "HTTP 400 data: {http_error_data.value():#?}" }
                pre { "HTTP 400 (context) data: {http_error_context_data.value():#?}" }
                pre { "Dog error data: {dog_data_err.value():#?}" }
                pre { "Throws ok data: {throws_ok_data.value():#?}" }
            }
        }
    });
}

/// Simple POST endpoint used to show a successful server function that returns `StatusCode`.
#[post("/api/data")]
async fn post_server_data(data: String) -> Result<(), StatusCode> {
    println!("Server received: {}", data);
    Ok(())
}

/// Fetches IP info from an external service. Demonstrates propagation of external errors.
#[get("/api/ip-data")]
async fn get_ip_data() -> Result<serde_json::Value> {
    Ok(reqwest::get("https://httpbin.org/ip").await?.json().await?)
}

/// Fetches a random dog image (successful external API example).
#[get("/api/dog-data")]
async fn get_dog_data() -> Result<serde_json::Value> {
    Ok(reqwest::get("https://dog.ceo/api/breeds/image/random")
        .await?
        .json()
        .await?)
}

/// Calls the Dog API with an invalid breed to trigger an external API error (e.g. 404).
#[get("/api/dog-data-err")]
async fn get_dog_data_err() -> Result<serde_json::Value> {
    Ok(
        reqwest::get("https://dog.ceo/api/breed/NOT_A_REAL_DOG/images")
            .await?
            .json()
            .await?,
    )
}

/// Accepts JSON and returns a custom-encoded JSON response.
#[post("/api/custom-encoding")]
async fn get_custom_encoding(takes: Json<serde_json::Value>) -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "message": "This response was encoded with a custom encoder!",
        "success": true,
        "you sent": takes.0,
    }))
}

/// Returns an untyped `anyhow` error with context (results in HTTP 500).
#[get("/api/untyped-error")]
async fn get_throws_error() -> Result<()> {
    Err(None.context("This is an example error using anyhow::Error")?)
}

/// Demonstrates returning an explicit HTTP error (400 Bad Request) using `HttpError`.
#[get("/api/throws-http-error")]
async fn throws_http_error() -> Result<()> {
    HttpError::bad_request("Bad request example")?;
    Ok(())
}

/// Convenience example: handles an Option and returns HTTP 400 with a message if None.
#[get("/api/throws-http-error-context")]
async fn throws_http_error_context() -> Result<String> {
    let res = None.or_bad_request("Value was None")?;
    Ok(res)
}

/// A simple server function that always succeeds.
#[get("/api/throws-ok")]
async fn get_throws_ok() -> Result<()> {
    Ok(())
}

#[derive(thiserror::Error, Debug, Serialize, Deserialize)]
enum MyCustomError {
    #[error("bad request")]
    BadRequest { custom_name: String },

    #[error("not found")]
    NotFound,

    #[error("internal server error: {0}")]
    ServerFnError(#[from] ServerFnError),
}

impl AsStatusCode for MyCustomError {
    fn as_status_code(&self) -> StatusCode {
        match self {
            MyCustomError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            MyCustomError::NotFound => StatusCode::NOT_FOUND,
            MyCustomError::ServerFnError(e) => e.as_status_code(),
        }
    }
}

/// Returns a custom typed error (serializable) so clients can handle specific cases.
///
/// Our custom error must implement `AsStatusCode` so it can properly set the outgoing HTTP status code.
#[get("/api/typed-error")]
async fn get_throws_typed_error() -> Result<(), MyCustomError> {
    Err(MyCustomError::BadRequest {
        custom_name: "Invalid input".into(),
    })
}

/// Simple POST endpoint used to show a successful server function that returns `StatusCode`.
#[post("/api/data")]
async fn get_throws_serverfn_error() -> Result<(), ServerFnError> {
    Err(ServerFnError::ServerError {
        message: "Unauthorized access".to_string(),
        code: StatusCode::UNAUTHORIZED.as_u16(),
        details: None,
    })
}
