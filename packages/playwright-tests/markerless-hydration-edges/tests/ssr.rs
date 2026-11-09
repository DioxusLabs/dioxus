//! Run with `UPDATE_FIXTURES=1` to regenerate the hydration fixture.
#![cfg(feature = "server")]

use dioxus::prelude::{DioxusRouterExt, ServeConfig};
use dioxus::server::axum::{
    Router,
    body::{Body, to_bytes},
};
use dioxus::server::http::{Request, StatusCode};
use dioxus_playwright_markerless_hydration_edges_test::app;
use tower::ServiceExt;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ssr.html");

async fn render_document() -> String {
    let response = Router::new()
        .serve_api_application(ServeConfig::new(), app)
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

fn without_scripts(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find("<script") {
        out.push_str(&rest[..start]);
        let end = rest[start..].find("</script>").unwrap() + start + "</script>".len();
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

#[tokio::test]
async fn fixture_matches_server_render() {
    let html = render_document().await;
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(FIXTURE, &html).unwrap();
    }
    let fixture = std::fs::read_to_string(FIXTURE).unwrap();
    assert!(
        fixture == html,
        "{FIXTURE} is stale, rerun with UPDATE_FIXTURES=1"
    );
}

#[tokio::test]
async fn ssr_emits_no_hydration_markers() {
    let body = without_scripts(&render_document().await);
    for marker in [
        "data-node-hydration",
        "<!--node-id",
        "<!--placeholder",
        "<!--#-->",
    ] {
        assert!(!body.contains(marker), "SSR output contains {marker}");
    }
}

#[tokio::test]
async fn scope_with_no_ssr_roots_renders_nothing() {
    let body = without_scripts(&render_document().await);
    assert!(!body.contains("late-empty-root"));
    assert!(!body.contains("root text ready"));
}

#[tokio::test]
async fn dangerous_inner_html_is_server_rendered() {
    let html = render_document().await;
    assert!(html.contains(r#"<p id="raw-inner-child">raw <strong>HTML</strong></p>"#));
}

#[tokio::test]
async fn hidden_nested_trailing_conditional_is_not_server_rendered() {
    let html = render_document().await;
    assert!(!html.contains("nested-trailing-late"));
}

#[tokio::test]
async fn textarea_body_is_server_rendered_without_markers() {
    let html = render_document().await;
    assert!(html.contains(r#"<textarea id="textarea-hydration">initial textarea body</textarea>"#));
}
