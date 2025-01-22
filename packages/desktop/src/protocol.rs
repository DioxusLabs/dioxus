use crate::document::NATIVE_EVAL_JS;
use crate::{assets::*, webview::WebviewEdits};
use dioxus_interpreter_js::unified_bindings::SLEDGEHAMMER_JS;
use dioxus_interpreter_js::NATIVE_JS;
use std::path::{Path, PathBuf};
use wry::{
    http::{status::StatusCode, Request, Response},
    RequestAsyncResponder, Result,
};

#[cfg(any(target_os = "android", target_os = "windows"))]
const EDITS_PATH: &str = "http://dioxus.index.html/__edits";

#[cfg(not(any(target_os = "android", target_os = "windows")))]
const EDITS_PATH: &str = "dioxus://index.html/__edits";

#[cfg(any(target_os = "android", target_os = "windows"))]
const EVENTS_PATH: &str = "http://dioxus.index.html/__events";

#[cfg(not(any(target_os = "android", target_os = "windows")))]
const EVENTS_PATH: &str = "dioxus://index.html/__events";

static DEFAULT_INDEX: &str = include_str!("./index.html");

#[allow(clippy::too_many_arguments)] // just for now, should fix this eventually
/// Handle a request from the webview
///
/// - Tries to stream edits if they're requested.
/// - If that doesn't match, tries a user provided asset handler
/// - If that doesn't match, tries to serve a file from the filesystem
pub(super) fn desktop_handler(
    request: Request<Vec<u8>>,
    asset_handlers: AssetHandlerRegistry,
    responder: RequestAsyncResponder,
    edit_state: &WebviewEdits,
    custom_head: Option<String>,
    custom_index: Option<String>,
    root_name: &str,
    headless: bool,
) {
    // Try to serve the index file first
    if let Some(index_bytes) =
        index_request(&request, custom_head, custom_index, root_name, headless)
    {
        return responder.respond(index_bytes);
    }

    // If the request is asking for edits (ie binary protocol streaming), do that
    let trimmed_uri = request.uri().path().trim_matches('/');
    if trimmed_uri == "__edits" {
        return edit_state.wry_queue.handle_request(responder);
    }

    // If the request is asking for an event response, do that
    if trimmed_uri == "__events" {
        return edit_state.handle_event(request, responder);
    }

    // todo: we want to move the custom assets onto a different protocol or something
    if let Some(name) = request.uri().path().split('/').nth(1) {
        if asset_handlers.has_handler(name) {
            let _name = name.to_string();
            return asset_handlers.handle_request(&_name, request, responder);
        }
    }

    match serve_asset(request) {
        Ok(res) => responder.respond(res),
        Err(_e) => responder.respond(
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(String::from("Failed to serve asset").into_bytes())
                .unwrap(),
        ),
    }
}

fn serve_asset(request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>> {
    // If the user provided a custom asset handler, then call it and return the response if the request was handled.
    // The path is the first part of the URI, so we need to trim the leading slash.
    let mut uri_path = PathBuf::from(
        urlencoding::decode(request.uri().path())
            .expect("expected URL to be UTF-8 encoded")
            .as_ref(),
    );

    // Attempt to serve from the asset dir on android using its loader
    #[cfg(target_os = "android")]
    {
        if let Some(asset) = to_java_load_asset(request.uri().path()) {
            return Ok(Response::builder()
                .header("Content-Type", get_mime_by_ext(&uri_path))
                .header("Access-Control-Allow-Origin", "*")
                .body(asset)?);
        }
    }

    // If the asset doesn't exist, or starts with `/assets/`, then we'll try to serve out of the bundle
    // This lets us handle both absolute and relative paths without being too "special"
    // It just means that our macos bundle is a little "special" because we need to place an `assets`
    // dir in the `Resources` dir.
    //
    // If there's no asset root, we use the cargo manifest dir as the root, or the current dir
    if !uri_path.exists() || uri_path.starts_with("/assets/") {
        let bundle_root = get_asset_root();
        let relative_path = uri_path.strip_prefix("/").unwrap();
        uri_path = bundle_root.join(relative_path);
    }

    // If the asset exists, then we can serve it!
    if uri_path.exists() {
        let mime_type = get_mime_from_path(&uri_path);
        return Ok(Response::builder()
            .header("Content-Type", mime_type?)
            .header("Access-Control-Allow-Origin", "*")
            .body(std::fs::read(uri_path)?)?);
    }

    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(String::from("Not Found").into_bytes())?)
}

/// Build the index.html file we use for bootstrapping a new app
///
/// We use wry/webview by building a special index.html that forms a bridge between the webview and your rust code
///
/// This is similar to tauri, except we give more power to your rust code and less power to your frontend code.
/// This lets us skip a build/bundle step - your code just works - but limits how your Rust code can actually
/// mess with UI elements. We make this decision since other renderers like LiveView are very separate and can
/// never properly bridge the gap. Eventually of course, the idea is to build a custom CSS/HTML renderer where you
/// *do* have native control over elements, but that still won't work with liveview.
fn index_request(
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
    window.interpreter = new NativeInterpreter("{EDITS_PATH}", "{EVENTS_PATH}");

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
/// Currently supports:
/// - [x] macOS
/// - [x] iOS
/// - [x] Windows
/// - [x] Linux (appimage)
/// - [ ] Linux (rpm)
/// - [ ] Linux (deb)
/// - [ ] Android
#[allow(unreachable_code)]
fn get_asset_root() -> PathBuf {
    let cur_exe = std::env::current_exe().unwrap();

    #[cfg(target_os = "macos")]
    {
        return cur_exe
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Resources");
    }

    // For all others, the structure looks like this:
    // app.(exe/appimage)
    //   main.exe
    //   assets/
    cur_exe.parent().unwrap().to_path_buf()
}

/// Get the mime type from a path-like string
fn get_mime_from_path(asset: &Path) -> Result<&'static str> {
    if asset.extension().is_some_and(|ext| ext == "svg") {
        return Ok("image/svg+xml");
    }

    match infer::get_from_path(asset)?.map(|f| f.mime_type()) {
        Some(f) if f != "text/plain" => Ok(f),
        _other => Ok(get_mime_by_ext(asset)),
    }
}

/// Get the mime type from a URI using its extension
fn get_mime_by_ext(trimmed: &Path) -> &'static str {
    match trimmed.extension().and_then(|e| e.to_str()) {
        // The common assets are all utf-8 encoded
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml; charset=utf-8",
        Some("html") => "text/html; charset=utf-8",

        // the rest... idk? probably not
        Some("mjs") => "text/javascript; charset=utf-8",
        Some("bin") => "application/octet-stream",
        Some("csv") => "text/csv",
        Some("ico") => "image/vnd.microsoft.icon",
        Some("jsonld") => "application/ld+json",
        Some("rtf") => "application/rtf",
        Some("mp4") => "video/mp4",
        // Assume HTML when a TLD is found for eg. `dioxus:://dioxuslabs.app` | `dioxus://hello.com`
        Some(_) => "text/html; charset=utf-8",
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types/Common_types
        // using octet stream according to this:
        None => "application/octet-stream",
    }
}

#[cfg(target_os = "android")]
pub(crate) fn to_java_load_asset(filepath: &str) -> Option<Vec<u8>> {
    let normalized = filepath
        .trim_start_matches("/assets/")
        .trim_start_matches('/');

    // in debug mode, the asset might be under `/data/local/tmp/dx/` - attempt to read it from there if it exists
    #[cfg(debug_assertions)]
    {
        let path = dioxus_cli_config::android_session_cache_dir().join(normalized);
        if path.exists() {
            return std::fs::read(path).ok();
        }
    }

    use std::ptr::NonNull;

    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.unwrap();
    let mut env = vm.attach_current_thread().unwrap();

    // Query the Asset Manager
    let asset_manager_ptr = env
        .call_method(
            unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) },
            "getAssets",
            "()Landroid/content/res/AssetManager;",
            &[],
        )
        .expect("Failed to get asset manager")
        .l()
        .expect("Failed to get asset manager as object");

    unsafe {
        let asset_manager =
            ndk_sys::AAssetManager_fromJava(env.get_native_interface(), *asset_manager_ptr);

        let asset_manager = ndk::asset::AssetManager::from_ptr(
            NonNull::new(asset_manager).expect("Invalid asset manager"),
        );

        let cstr = std::ffi::CString::new(normalized).unwrap();

        let mut asset = asset_manager.open(&cstr)?;
        Some(asset.buffer().unwrap().to_vec())
    }
}
