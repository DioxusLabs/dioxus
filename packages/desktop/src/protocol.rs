use std::path::Path;
use wry::{
    http::{status::StatusCode, Request, Response, ResponseBuilder},
    Result,
};

pub(super) fn desktop_handler(request: &Request) -> Result<Response> {
    // Any content that uses the `dioxus://` scheme will be shuttled through this handler as a "special case".
    // For now, we only serve two pieces of content which get included as bytes into the final binary.
    let path = request.uri().replace("dioxus://", "");

    // all assets should be called from index.html
    let trimmed = path.trim_start_matches("index.html/");

    if trimmed.is_empty() {
        ResponseBuilder::new()
            .mimetype("text/html")
            .body(include_bytes!("./index.html").to_vec())
    } else if trimmed == "index.js" {
        ResponseBuilder::new()
            .mimetype("text/javascript")
            .body(dioxus_interpreter_js::INTERPRETER_JS.as_bytes().to_vec())
    } else {
        let path_buf = Path::new(trimmed).canonicalize()?;
        let cur_path = Path::new(".").canonicalize()?;

        if !path_buf.starts_with(cur_path) {
            return ResponseBuilder::new()
                .status(StatusCode::FORBIDDEN)
                .body(String::from("Forbidden").into_bytes());
        }

        if !path_buf.exists() {
            return ResponseBuilder::new()
                .status(StatusCode::NOT_FOUND)
                .body(String::from("Not Found").into_bytes());
        }

        let mime = mime_guess::from_path(&path_buf).first_or_octet_stream();

        // do not let path searching to go two layers beyond the caller level
        let data = std::fs::read(path_buf)?;
        let meta = format!("{}", mime);

        ResponseBuilder::new().mimetype(&meta).body(data)
    }
}
