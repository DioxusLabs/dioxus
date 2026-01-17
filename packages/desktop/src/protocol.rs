use std::path::PathBuf;

use crate::{assets::*, webview::WebviewEdits};
use crate::{document::NATIVE_EVAL_JS, file_upload::FileDialogRequest};
use base64::prelude::BASE64_STANDARD;
use dioxus_core::AnyhowContext;
use dioxus_html::{SerializedFileData, SerializedFormObject};
use dioxus_interpreter_js::unified_bindings::SLEDGEHAMMER_JS;
use dioxus_interpreter_js::NATIVE_JS;
use wry::{
    http::{status::StatusCode, Request, Response},
    RequestAsyncResponder,
};

#[cfg(target_os = "android")]
const BASE_URI: &str = "https://dioxus.index.html/";

#[cfg(target_os = "windows")]
const BASE_URI: &str = "http://dioxus.index.html/";

#[cfg(not(any(target_os = "android", target_os = "windows")))]
const BASE_URI: &str = "dioxus://index.html/";

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

    // If the request is asking for a file dialog, handle that, returning the list of files selected
    if trimmed_uri == "__file_dialog" {
        if let Err(err) = file_dialog_responder_sync(request, responder) {
            tracing::error!("Failed to handle file dialog request: {err:?}");
        }

        return;
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
    window.interpreter = new NativeInterpreter("{BASE_URI}", {headless});

    // Wait for the page to load before sending the initialize message
    window.onload = function() {{
        let root_element = window.document.getElementById("{root_id}");
        if (root_element != null) {{
            window.interpreter.initialize(root_element);
            window.interpreter.sendIpcMessage("initialize");
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

fn file_dialog_responder_sync(
    request: wry::http::Request<Vec<u8>>,
    responder: wry::RequestAsyncResponder,
) -> dioxus_core::Result<()> {
    // Handle the file dialog request
    // We can't use the body, just the headers
    let header = request
        .headers()
        .get("x-dioxus-data")
        .context("Failed to get x-dioxus-data header")?;

    let data_from_header = base64::Engine::decode(&BASE64_STANDARD, header.as_bytes())
        .context("Failed to decode x-dioxus-data header from base64")?;

    let file_dialog: FileDialogRequest = serde_json::from_slice(&data_from_header)
        .context("Failed to parse x-dioxus-data header as JSON")?;

    #[cfg(feature = "tokio_runtime")]
    tokio::spawn(async move {
        let file_list = file_dialog.get_file_event_async().await;
        _ = respond_to_file_dialog(file_dialog, file_list, responder);
    });

    #[cfg(not(feature = "tokio_runtime"))]
    {
        let file_list = file_dialog.get_file_event_sync();
        respond_to_file_dialog(file_dialog, file_list, responder)?;
    }

    Ok(())
}

fn respond_to_file_dialog(
    mut file_dialog: FileDialogRequest,
    file_list: Vec<PathBuf>,
    responder: wry::RequestAsyncResponder,
) -> dioxus_core::Result<()> {
    // Get the position of the entry we're updating, so we can insert new entries in the same place
    // If we can't find it, just append to the end. This is usually due to the input not being in a form element.
    let position_of_entry = file_dialog
        .values
        .iter()
        .position(|x| x.key == file_dialog.target_name)
        .unwrap_or(file_dialog.values.len());

    // Remove any existing entries
    file_dialog
        .values
        .retain(|x| x.key != file_dialog.target_name);

    // And then insert the new ones
    for path in file_list {
        let file = std::fs::metadata(&path).context("Failed to get file metadata")?;
        file_dialog.values.insert(
            position_of_entry,
            SerializedFormObject {
                key: file_dialog.target_name.clone(),
                text: None,
                file: Some(SerializedFileData {
                    size: file.len(),
                    last_modified: file
                        .modified()
                        .context("Failed to get file modified time")?
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or_default() as _,
                    content_type: Some(
                        dioxus_asset_resolver::native::get_mime_from_ext(
                            path.extension().and_then(|s| s.to_str()),
                        )
                        .to_string(),
                    ),
                    contents: Default::default(),
                    path,
                }),
            },
        );
    }

    // And then respond with the updated file dialog
    let response_data = serde_json::to_vec(&file_dialog)
        .context("Failed to serialize FileDialogRequest to JSON")?;

    responder.respond(
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(response_data)
            .context("Failed to build response")?,
    );

    Ok(())
}
