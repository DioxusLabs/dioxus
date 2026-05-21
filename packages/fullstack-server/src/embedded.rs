use axum::{
    Router,
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use http::header::{CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE};

use rust_embed::RustEmbed;

use crate::server::file_name_looks_immutable;

#[derive(RustEmbed)]
#[folder = "$DIOXUS_EMBED_DIR"]
struct PublicAssets;

/// Read the embedded index.html contents, if present.
pub(crate) fn embedded_index_html() -> Option<String> {
    let file = PublicAssets::get("index.html")?;
    String::from_utf8(file.data.into_owned()).ok()
}

pub(crate) fn serve_embedded_assets<S>(mut router: Router<S>) -> Router<S>
where
    S: Send + Sync + Clone + 'static,
{
    for file_path in PublicAssets::iter() {
        let path = file_path.to_string();

        // Don't serve index.html — SSR generates it
        if path == "index.html" {
            continue;
        }

        let route = format!("/{path}");
        let immutable = file_name_looks_immutable(&route);

        // Check if a brotli-compressed variant exists
        let has_br = PublicAssets::get(&format!("{path}.br")).is_some();

        // Don't register the .br files as their own routes
        if path.ends_with(".br") {
            continue;
        }

        let mime = mime_guess::from_path(&path)
            .first_or_octet_stream()
            .to_string();

        router = router.route(
            &route,
            get(move |headers: http::HeaderMap| async move {
                // Serve brotli variant if client supports it
                let accepts_br = headers
                    .get(http::header::ACCEPT_ENCODING)
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v.contains("br"));

                let (body, is_br) = if has_br && accepts_br {
                    match PublicAssets::get(&format!("{path}.br")) {
                        Some(file) => (file.data.into_owned(), true),
                        None => match PublicAssets::get(&path) {
                            Some(file) => (file.data.into_owned(), false),
                            None => return StatusCode::NOT_FOUND.into_response(),
                        },
                    }
                } else {
                    match PublicAssets::get(&path) {
                        Some(file) => (file.data.into_owned(), false),
                        None => return StatusCode::NOT_FOUND.into_response(),
                    }
                };

                let mut builder = Response::builder().header(CONTENT_TYPE, &mime);

                if is_br {
                    builder = builder.header(CONTENT_ENCODING, "br");
                }

                if immutable {
                    builder = builder.header(CACHE_CONTROL, "public, max-age=31536000, immutable");
                }

                builder
                    .body(Body::from(body))
                    .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
            }),
        );
    }

    router
}
