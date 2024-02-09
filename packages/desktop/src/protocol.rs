use crate::{assets::*, edits::EditQueue};
use dioxus_interpreter_js::binary_protocol::SLEDGEHAMMER_JS;
use std::path::{Path, PathBuf};
use wry::{
    http::{status::StatusCode, Request, Response},
    RequestAsyncResponder, Result,
};

fn handle_edits_code() -> String {
    const EDITS_PATH: &str = {
        #[cfg(any(target_os = "android", target_os = "windows"))]
        {
            "http://dioxus.index.html/edits"
        }
        #[cfg(not(any(target_os = "android", target_os = "windows")))]
        {
            "dioxus://index.html/edits"
        }
    };

    let prevent_file_upload = r#"// Prevent file inputs from opening the file dialog on click
    let inputs = document.querySelectorAll("input");
    for (let input of inputs) {
      if (!input.getAttribute("data-dioxus-file-listener")) {
        // prevent file inputs from opening the file dialog on click
        const type = input.getAttribute("type");
        if (type === "file") {
          input.setAttribute("data-dioxus-file-listener", true);
          input.addEventListener("click", (event) => {
            let target = event.target;
            let target_id = find_real_id(target);
            if (target_id !== null) {
              const send = (event_name) => {
                const message = window.interpreter.serializeIpcMessage("file_diolog", { accept: target.getAttribute("accept"), directory: target.getAttribute("webkitdirectory") === "true", multiple: target.hasAttribute("multiple"), target: parseInt(target_id), bubbles: event_bubbles(event_name), event: event_name });
                window.ipc.postMessage(message);
              };
              send("change&input");
            }
            event.preventDefault();
          });
        }
      }
    }"#;
    let polling_request = format!(
        r#"// Poll for requests
    window.interpreter.wait_for_request = (headless) => {{
      fetch(new Request("{EDITS_PATH}"))
          .then(response => {{
              response.arrayBuffer()
                  .then(bytes => {{
                      // In headless mode, the requestAnimationFrame callback is never called, so we need to run the bytes directly
                      if (headless) {{
                        run_from_bytes(bytes);
                      }}
                      else {{
                        requestAnimationFrame(() => {{
                          run_from_bytes(bytes);
                        }});
                      }}
                      window.interpreter.wait_for_request(headless);
                  }});
          }})
    }}"#
    );
    let mut interpreter = SLEDGEHAMMER_JS
        .replace("/*POST_HANDLE_EDITS*/", prevent_file_upload)
        .replace("export", "")
        + &polling_request;
    while let Some(import_start) = interpreter.find("import") {
        let import_end = interpreter[import_start..]
            .find(|c| c == ';' || c == '\n')
            .map(|i| i + import_start)
            .unwrap_or_else(|| interpreter.len());
        interpreter.replace_range(import_start..import_end, "");
    }

    format!("{interpreter}\nconst config = new InterpreterConfig(true);")
}

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
    let head = match custom_head {
        Some(mut head) => {
            if let Some(assets_head) = assets_head() {
                head.push_str(&assets_head);
            }
            Some(head)
        }
        None => assets_head(),
    };
    if let Some(head) = head {
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

fn assets_head() -> Option<String> {
    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        let head = crate::protocol::get_asset_root_or_default();
        let head = head.join("__assets_head.html");
        match std::fs::read_to_string(&head) {
            Ok(s) => Some(s),
            Err(err) => {
                tracing::error!("Failed to read {head:?}: {err}");
                None
            }
        }
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    {
        None
    }
}

/// Handle a request from the webview
///
/// - Tries to stream edits if they're requested.
/// - If that doesn't match, tries a user provided asset handler
/// - If that doesn't match, tries to serve a file from the filesystem
pub(super) fn desktop_handler(
    request: Request<Vec<u8>>,
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

    if path.parent().is_none() {
        return tracing::error!("Asset request has no parent {path:?}");
    }

    if let Some(name) = path.iter().next().unwrap().to_str() {
        if asset_handlers.has_handler(name) {
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

/// Construct the inline script that boots up the page and bridges the webview with rust code.
///
/// The arguments here:
/// - root_name: the root element (by Id) that we stream edits into
/// - headless: is this page being loaded but invisible? Important because not all windows are visible and the
///             interpreter can't connect until the window is ready.
fn module_loader(root_id: &str, headless: bool) -> String {
    let js = handle_edits_code();
    format!(
        r#"
<script type="module">
    {js}
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
