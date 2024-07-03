use axum::{routing::any, Router};
use http::Uri;

/// Opens an endpoint on the devserver that proxies every request aimned at "/__dioxus_devtools/*" to the devtools server
///
/// This way we have a single unified server that handles hotreloading, logging, devtools, etc.
pub fn make_devtools_proxy(mut router: Router, host: Option<Uri>) -> Router {
    router.route(
        "/__dioxus_devtools/*path",
        any(|request: axum::extract::Request| async move {
            // let path = request.uri().path();
            // let path = path.trim_start_matches("/__dioxusd_devtools/");
            // let path = format!("http://localhost:3000/{}", path);
            // let client = reqwest::Client::new();
            // let response = client.get(&path).send().await.unwrap();
            // let body = response.text().await.unwrap();
            // axum::http::Response::new(body.into())
        }),
    )
}
