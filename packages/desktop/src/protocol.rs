use crate::document::NATIVE_EVAL_JS;
use crate::{assets::*, webview::WebviewEdits};
use dioxus_interpreter_js::unified_bindings::SLEDGEHAMMER_JS;
use dioxus_interpreter_js::NATIVE_JS;
use wry::{
    http::{status::StatusCode, Request, Response},
    RequestAsyncResponder,
};

#[cfg(target_os = "android")]
const EVENTS_PATH: &str = "https://dioxus.index.html/__events";

#[cfg(target_os = "windows")]
const EVENTS_PATH: &str = "http://dioxus.index.html/__events";

#[cfg(not(any(target_os = "android", target_os = "windows")))]
const EVENTS_PATH: &str = "dioxus://index.html/__events";

#[cfg(debug_assertions)]
static DEFAULT_INDEX: &str = include_str!("./assets/dev.index.html");

#[cfg(not(debug_assertions))]
static DEFAULT_INDEX: &str = include_str!("./assets/prod.index.html");

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
    if let Some(index_bytes) = index_request(
        &request,
        custom_head,
        custom_index,
        root_name,
        headless,
        edit_state,
    ) {
        return responder.respond(index_bytes);
    }

    // If the request is asking for edits (ie binary protocol streaming), do that
    let trimmed_uri = request.uri().path().trim_matches('/');

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

    match dioxus_asset_resolver::native::serve_asset(request.uri().path()) {
        Ok(res) => responder.respond(res),
        Err(_e) => responder.respond(
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(String::from("Failed to serve asset").into_bytes())
                .unwrap(),
        ),
    }
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
    edit_state: &WebviewEdits,
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
        &module_loader(root_name, headless, edit_state),
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
///   interpreter can't connect until the window is ready.
/// - port: the port that the websocket server is listening on for edits
/// - webview_id: the id of the webview that we're loading this into. This is used to differentiate between
///   multiple webviews in the same application, so that we can send edits to the correct one.
fn module_loader(root_id: &str, headless: bool, edit_state: &WebviewEdits) -> String {
    let edits_path = edit_state.wry_queue.edits_path();
    let expected_key = edit_state.wry_queue.required_server_key();

    format!(
        r#"
<script type="module">
    // Bring the sledgehammer code
    {SLEDGEHAMMER_JS}

    // And then extend it with our native bindings
    {NATIVE_JS}

    // The native interpreter extends the sledgehammer interpreter with a few extra methods that we use for IPC
    window.interpreter = new NativeInterpreter("{EVENTS_PATH}", {headless});

    // Wait for the page to load before sending the initialize message
    window.onload = function() {{
        let root_element = window.document.getElementById("{root_id}");
        if (root_element != null) {{
            window.interpreter.initialize(root_element);
            window.ipc.postMessage(window.interpreter.serializeIpcMessage("initialize"));
        }}
        window.interpreter.waitForRequest("{edits_path}", "{expected_key}");
    }}
</script>
<script type="module">
    // Include the code for eval
    {NATIVE_EVAL_JS}
</script>
"#
    )
}
