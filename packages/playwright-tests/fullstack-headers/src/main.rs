// This test fixture is used by the fullstack-headers playwright test
// Tests:
// - add_response_header Set-Cookie append behavior (RFC 6265)
// - add_response_header non-Set-Cookie overwrite behavior

#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    #[cfg(feature = "server")]
    dioxus::serve(|| async move {
        let cfg = dioxus::server::ServeConfig::builder().enable_out_of_order_streaming();
        let router = dioxus::server::axum::Router::new().serve_dioxus_application(cfg, app);
        Ok(router)
    });
    #[cfg(not(feature = "server"))]
    launch(app);
}

fn app() -> Element {
    rsx! {
        button {
            id: "set-cookie-btn",
            onclick: move |_| async move {
                _ = test_set_cookie().await;
            },
            "Set Cookie"
        }
        button {
            id: "override-header-btn",
            onclick: move |_| async move {
                _ = test_override_header().await;
            },
            "Override Header"
        }
    }
}

#[server(endpoint = "test_set_cookie")]
async fn test_set_cookie() -> ServerFnResult {
    use dioxus::fullstack::FullstackContext;
    let ctx = FullstackContext::current().unwrap();
    ctx.add_response_header(
        http::header::SET_COOKIE,
        http::HeaderValue::from_static("session_id=abc123; Path=/"),
    );
    ctx.add_response_header(
        http::header::SET_COOKIE,
        http::HeaderValue::from_static("theme=dark; Path=/"),
    );
    Ok(())
}

#[server(endpoint = "test_override_header")]
async fn test_override_header() -> ServerFnResult {
    use dioxus::fullstack::FullstackContext;
    let ctx = FullstackContext::current().unwrap();
    let name = http::header::HeaderName::from_static("x-custom-header");
    ctx.add_response_header(name.clone(), http::HeaderValue::from_static("first"));
    ctx.add_response_header(name, http::HeaderValue::from_static("second"));
    Ok(())
}
