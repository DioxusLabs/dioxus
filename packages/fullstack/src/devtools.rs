use axum::{routing::any, Router};
use http::Uri;

/// Opens an endpoint on the devserver that proxies every request aimned at "/_dioxus/*" to the devtools server
///
/// This way we have a single unified server that handles hotreloading, logging, devtools, etc.
pub fn make_devtools_proxy(router: Router) -> Router {
    router.route(
        "/_dioxus/*path",
        any(|request: axum::extract::Request| async move {
            let path = request.uri().path();
            let path = format!("http://localhost:6478/{}", path);
            let client = server_fn::request::reqwest::Client::new();
            let response = client.get(&path).send().await.unwrap();
            let body = response.text().await.unwrap();
            axum::http::Response::new(body)
        }),
    )
}
