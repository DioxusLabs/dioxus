//! This example shows how you can extract a HeaderMap from requests to read custom headers.
//!
//! The extra arguments in the `#[get(...)]` macro are passed to the underlying axum handler,
//! and only visible on the server. This lets you run normal axum extractors like `HeaderMap`,
//! `TypedHeader`, `Query`, etc.
//!
//! Note that headers returned by the server are not always visible to the client due to CORS.
//! Headers like `Set-Cookie` are hidden by default, and need to be explicitly allowed
//! by the server using the `Access-Control-Expose-Headers` header (which dioxus-fullstack does not
//! currently expose directly).

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut headers = use_action(get_headers);

    rsx! {
        h1 { "Header Map Example" }
        button { onclick: move |_| headers.call(), "Get Headers" }
        if let Some(Ok(headers)) = headers.value() {
            p { "Response from server:" }
            pre { "{headers}" }
        } else {
            p { "No headers yet" }
        }
    }
}

#[get("/api/example", headers: dioxus::fullstack::HeaderMap)]
async fn get_headers() -> Result<String> {
    Ok(format!("{:#?}", headers))
}
