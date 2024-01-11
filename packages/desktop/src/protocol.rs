use crate::{assets::*, edits::EditQueue};
use std::path::{Path, PathBuf};
use wry::{
    http::{status::StatusCode, Request, Response, Uri},
    RequestAsyncResponder, Result,
};

static MINIFIED: &str = include_str!("./minified.js");
static DEFAULT_INDEX: &str = include_str!("./index.html");

/// Build the index.html file we use for bootstrapping a new app
///
/// We use wry/webview by building a special index.html that forms a bridge between the webview and your rust code
///
/// This is similar to tauri, except we give more power to your rust code and less power to your frontend code.
/// This lets us skip a build/bundle step - your code just works - but limits how your Rust code can actually
/// mess with UI elements. We make this decision since other renderers like LiveView are very separate and can
/// never properly bridge the gap. Eventually of course, the idea is to build a custom CSS/HTML renderer where you
/// *do* have native control over elements, but that still won't work with liveview.
pub(super) fn index_request(
    request: &Request<Vec<u8>>,
    custom_head: Option<String>,
    custom_index: Option<String>,
    root_name: &str,
    headless: bool,
) -> Option<Response<Vec<u8>>> {
    // If the request is for the root, we'll serve the index.html file.
    if request.uri().path() != "/" {
        return None;
    }

    // Load a custom index file if provided
    let mut index = custom_index.unwrap_or_else(|| DEFAULT_INDEX.to_string());

    // Insert a custom head if provided
    // We look just for the closing head tag. If a user provided a custom index with weird syntax, this might fail
    if let Some(head) = custom_head {
        index.insert_str(index.find("</head>").expect("Head element to exist"), &head);
    }

    // Inject our module loader by looking for a body tag
    // A failure mode here, obviously, is if the user provided a custom index without a body tag
    // Might want to document this
    index.insert_str(
        index.find("</body>").expect("Body element to exist"),
        &module_loader(root_name, headless),
    );

    Response::builder()
        .header("Content-Type", "text/html")
        .header("Access-Control-Allow-Origin", "*")
        .body(index.into())
        .ok()
}

/// Handle a request from the webview
///
/// - Tries to stream edits if they're requested.
/// - If that doesn't match, tries a user provided asset handler
/// - If that doesn't match, tries to serve a file from the filesystem
pub(super) fn desktop_handler(
    mut request: Request<Vec<u8>>,
    asset_handlers: AssetHandlerRegistry,
    edit_queue: &EditQueue,
    responder: RequestAsyncResponder,
) {
    // If the request is asking for edits (ie binary protocol streaming, do that)
    if request.uri().path().trim_matches('/') == "edits" {
        return edit_queue.handle_request(responder);
    }

    // If the user provided a custom asset handler, then call it and return the response if the request was handled.
    // The path is the first part of the URI, so we need to trim the leading slash.
    let path = PathBuf::from(
        urlencoding::decode(request.uri().path().trim_start_matches('/'))
            .expect("expected URL to be UTF-8 encoded")
            .as_ref(),
    );

    let Some(name) = path.parent() else {
        return tracing::error!("Asset request has no root {path:?}");
    };

    if let Some(name) = name.to_str() {
        if asset_handlers.has_handler(name) {
            // Trim the leading path from the URI
            //
            // I hope this is reliable!
            //
            // so a request for /assets/logos/logo.png?query=123 will become /logos/logo.png?query=123
            strip_uri_prefix(&mut request, name);

            return asset_handlers.handle_request(name, request, responder);
        }
    }

    // Else, try to serve a file from the filesystem.
    match serve_from_fs(path) {
        Ok(res) => responder.respond(res),
        Err(e) => tracing::error!("Error serving request from filesystem {}", e),
    }
}

fn serve_from_fs(path: PathBuf) -> Result<Response<Vec<u8>>> {
    // If the path is relative, we'll try to serve it from the assets directory.
    let mut asset = get_asset_root_or_default().join(&path);

    // If we can't find it, make it absolute and try again
    if !asset.exists() {
        asset = PathBuf::from("/").join(path);
    }

    if !asset.exists() {
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(String::from("Not Found").into_bytes())?);
    }

    Ok(Response::builder()
        .header("Content-Type", get_mime_from_path(&asset)?)
        .body(std::fs::read(asset)?)?)
}

fn strip_uri_prefix(request: &mut Request<Vec<u8>>, name: &str) {
    // trim the leading path
    if let Some(path) = request.uri().path_and_query() {
        let new_path = path
            .path()
            .trim_start_matches('/')
            .strip_prefix(name)
            .expect("expected path to have prefix");

        let new_uri = Uri::builder()
            .scheme(request.uri().scheme_str().unwrap_or("http"))
            .path_and_query(format!("{}{}", new_path, path.query().unwrap_or("")))
            .authority("index.html")
            .build()
            .expect("failed to build new URI");

        *request.uri_mut() = new_uri;
    }
}

/// Construct the inline script that boots up the page and bridges the webview with rust code.
///
/// The arguments here:
/// - root_name: the root element (by Id) that we stream edits into
/// - headless: is this page being loaded but invisible? Important because not all windows are visible and the
///             interpreter can't connect until the window is ready.
fn module_loader(root_id: &str, headless: bool) -> String {
    format!(
        r#"
<script type="module">
    {MINIFIED}
    // Wait for the page to load
    window.onload = function() {{
        let rootname = "{root_id}";
        let root_element = window.document.getElementById(rootname);
        if (root_element != null) {{
            window.interpreter.initialize(root_element);
            window.ipc.postMessage(window.interpreter.serializeIpcMessage("initialize"));
        }}
        window.interpreter.wait_for_request({headless});
    }}
</script>
"#
    )
}

/// Get the asset directory, following tauri/cargo-bundles directory discovery approach
///
/// Defaults to the current directory if no asset directory is found, which is useful for development when the app
/// isn't bundled.
fn get_asset_root_or_default() -> PathBuf {
    get_asset_root().unwrap_or_else(|| Path::new(".").to_path_buf())
}

/// Get the asset directory, following tauri/cargo-bundles directory discovery approach
///
/// Currently supports:
/// - [x] macOS
/// - [ ] Windows
/// - [ ] Linux (rpm)
/// - [ ] Linux (deb)
/// - [ ] iOS
/// - [ ] Android
#[allow(unreachable_code)]
fn get_asset_root() -> Option<PathBuf> {
    // If running under cargo, there's no bundle!
    // There might be a smarter/more resilient way of doing this
    if std::env::var_os("CARGO").is_some() {
        return None;
    }

    #[cfg(target_os = "macos")]
    {
        let bundle = core_foundation::bundle::CFBundle::main_bundle();
        let bundle_path = bundle.path()?;
        let resources_path = bundle.resources_path()?;
        let absolute_resources_root = bundle_path.join(resources_path);
        return dunce::canonicalize(absolute_resources_root).ok();
    }

    None
}

/// Get the mime type from a path-like string
fn get_mime_from_path(trimmed: &Path) -> Result<&'static str> {
    if trimmed.extension().is_some_and(|ext| ext == "svg") {
        return Ok("image/svg+xml");
    }

    match infer::get_from_path(trimmed)?.map(|f| f.mime_type()) {
        Some(f) if f != "text/plain" => Ok(f),
        _ => Ok(get_mime_by_ext(trimmed)),
    }
}

/// Get the mime type from a URI using its extension
fn get_mime_by_ext(trimmed: &Path) -> &'static str {
    match trimmed.extension().and_then(|e| e.to_str()) {
        Some("bin") => "application/octet-stream",
        Some("css") => "text/css",
        Some("csv") => "text/csv",
        Some("html") => "text/html",
        Some("ico") => "image/vnd.microsoft.icon",
        Some("js") => "text/javascript",
        Some("json") => "application/json",
        Some("jsonld") => "application/ld+json",
        Some("mjs") => "text/javascript",
        Some("rtf") => "application/rtf",
        Some("svg") => "image/svg+xml",
        Some("mp4") => "video/mp4",
        // Assume HTML when a TLD is found for eg. `dioxus:://dioxuslabs.app` | `dioxus://hello.com`
        Some(_) => "text/html",
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types/Common_types
        // using octet stream according to this:
        None => "application/octet-stream",
    }
}
