//! This example shows how to get access to the full axum request in a handler.
//!
//! The extra arguments in the `post` macro are passed to the handler function, but not exposed
//! to the client. This means we can still call the endpoint from the client, but have full access
//! to the request on the server.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut file_id = use_action(full_request);

    rsx! {
        div { "Access to full axum request" }
        button { onclick: move |_| file_id.call(), "Upload file" }
    }
}

/// Example of accessing the full axum request in a handler
///
/// The `request: axum_core::extract::Request` argument is placed in the handler function, but not
/// exposed to the client.
#[post("/api/full_request_access", request: axum_core::extract::Request)]
async fn full_request() -> Result<()> {
    let headers = request.headers();

    if headers.contains_key("x-api-key") {
        println!("API key found");
    } else {
        println!("No API key found");
    }

    Ok(())
}
