use crate::{assets::*, edits::EditQueue};
use dioxus_html::document::NATIVE_EVAL_JS;
use dioxus_interpreter_js::unified_bindings::SLEDGEHAMMER_JS;
use dioxus_interpreter_js::NATIVE_JS;
use serde::Deserialize;
use std::{
    path::{Component, Path, PathBuf},
    process::Command,
    sync::OnceLock,
};
use wry::{
    http::{status::StatusCode, Request, Response},
    RequestAsyncResponder, Result,
};

#[cfg(any(target_os = "android", target_os = "windows"))]
const EDITS_PATH: &str = "http://dioxus.index.html/edits";

#[cfg(not(any(target_os = "android", target_os = "windows")))]
const EDITS_PATH: &str = "dioxus://index.html/edits";

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
        let assets_head_path = PathBuf::from("__assets_head.html");
        let head = resolve_resource(&assets_head_path);
        match std::fs::read_to_string(&head) {
            Ok(s) => Some(s),
            Err(err) => {
                tracing::warn!("Assets built with manganis cannot be preloaded (failed to read {head:?}). This warning may occur when you build a desktop application without the dioxus CLI. If you do not use manganis, you can ignore this warning: {err}.");
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

fn resolve_resource(path: &Path) -> PathBuf {
    let mut base_path = get_asset_root_or_default();

    if running_in_dev_mode() {
        base_path.push(path);

        // Special handler for Manganis filesystem fallback.
        // We need this since Manganis provides assets from workspace root.
        if !base_path.exists() {
            let workspace_root = get_workspace_root_from_cargo();
            let asset_path = workspace_root.join(path);
            println!("ASSET PATH: {:?}", asset_path);
            return asset_path;
        }
    } else {
        let mut resource_path = PathBuf::new();
        for component in path.components() {
            // Tauri-bundle inserts special path segments for abnormal component paths
            match component {
                Component::Prefix(_) => {}
                Component::RootDir => resource_path.push("_root_"),
                Component::CurDir => {}
                Component::ParentDir => resource_path.push("_up_"),
                Component::Normal(p) => resource_path.push(p),
            }
        }
        base_path.push(resource_path);
    }
    base_path
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
        Err(e) => {
            tracing::error!("Error serving request from filesystem {}", e);
        }
    }
}

fn serve_from_fs(path: PathBuf) -> Result<Response<Vec<u8>>> {
    // If the path is relative, we'll try to serve it from the assets directory.
    let mut asset = resolve_resource(&path);

    // If we can't find it, make it absolute and try again
    if !asset.exists() {
        asset = PathBuf::from("/").join(&path);
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
    format!(
        r#"
<script type="module">
    // Bring the sledgehammer code
    {SLEDGEHAMMER_JS}

    // And then extend it with our native bindings
    {NATIVE_JS}

    // The native interpreter extends the sledgehammer interpreter with a few extra methods that we use for IPC
    window.interpreter = new NativeInterpreter("{EDITS_PATH}");

    // Wait for the page to load before sending the initialize message
    window.onload = function() {{
        let root_element = window.document.getElementById("{root_id}");
        if (root_element != null) {{
            window.interpreter.initialize(root_element);
            window.ipc.postMessage(window.interpreter.serializeIpcMessage("initialize"));
        }}
        window.interpreter.waitForRequest({headless});
    }}
</script>
<script type="module">
    // Include the code for eval
    {NATIVE_EVAL_JS}
</script>
"#
    )
}

/// Get the asset directory, following tauri/cargo-bundles directory discovery approach
///
/// Defaults to the current directory if no asset directory is found, which is useful for development when the app
/// isn't bundled.
fn get_asset_root_or_default() -> PathBuf {
    get_asset_root().unwrap_or_else(|| std::env::current_dir().unwrap())
}

fn running_in_dev_mode() -> bool {
    // If running under cargo, there's no bundle!
    // There might be a smarter/more resilient way of doing this
    std::env::var_os("CARGO").is_some()
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
    if running_in_dev_mode() {
        return dioxus_cli_config::CURRENT_CONFIG
            .as_ref()
            .map(|c| c.application.out_dir.clone())
            .ok();
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

/// A global that stores the workspace root. Used in [`get_workspace_root_from_cargo`].
static WORKSPACE_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Describes the metadata we need from `cargo metadata`.
#[derive(Deserialize)]
struct CargoMetadata {
    workspace_root: PathBuf,
}

/// Get the workspace root using `cargo metadata`. Should not be used in release mode.
pub(crate) fn get_workspace_root_from_cargo() -> PathBuf {
    WORKSPACE_ROOT
        .get_or_init(|| {
            let out = Command::new("cargo")
                .args(["metadata", "--format-version", "1", "--no-deps"])
                .output()
                .expect("`cargo metadata` failed to run");

            let out =
                String::from_utf8(out.stdout).expect("failed to parse output of `cargo metadata`");
            let metadata = serde_json::from_str::<CargoMetadata>(&out)
                .expect("failed to deserialize data from `cargo metadata`");

            metadata.workspace_root
        })
        .to_owned()
}
