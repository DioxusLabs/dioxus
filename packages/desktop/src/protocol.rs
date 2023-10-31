use dioxus_interpreter_js::{COMMON_JS, INTERPRETER_JS};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};
use wry::{
    http::{status::StatusCode, Request, Response},
    Result,
};

fn module_loader(root_name: &str) -> String {
    let js = INTERPRETER_JS.replace(
        "/*POST_HANDLE_EDITS*/",
        r#"// Prevent file inputs from opening the file dialog on click
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
                const message = serializeIpcMessage("file_diolog", { accept: target.getAttribute("accept"), directory: target.getAttribute("webkitdirectory") === "true", multiple: target.hasAttribute("multiple"), target: parseInt(target_id), bubbles: event_bubbles(event_name), event: event_name });
                window.ipc.postMessage(message);
              };
              send("change&input");
            }
            event.preventDefault();
          });
        }
      }
    }"#,
    );
    format!(
        r#"
<script type="module">
    {js}

    let rootname = "{root_name}";
    let root = window.document.getElementById(rootname);
    if (root != null) {{
        window.interpreter = new Interpreter(root, new InterpreterConfig(true));
        window.ipc.postMessage(serializeIpcMessage("initialize"));
    }}
</script>
"#
    )
}

pub(super) fn desktop_handler(
    request: &Request<Vec<u8>>,
    custom_head: Option<String>,
    custom_index: Option<String>,
    root_name: &str,
) -> Result<Response<Cow<'static, [u8]>>> {
    // If the request is for the root, we'll serve the index.html file.
    if request.uri().path() == "/" {
        // If a custom index is provided, just defer to that, expecting the user to know what they're doing.
        // we'll look for the closing </body> tag and insert our little module loader there.
        let body = match custom_index {
            Some(custom_index) => custom_index
                .replace("</body>", &format!("{}</body>", module_loader(root_name)))
                .into_bytes(),

            None => {
                // Otherwise, we'll serve the default index.html and apply a custom head if that's specified.
                let mut template = include_str!("./index.html").to_string();

                if let Some(custom_head) = custom_head {
                    template = template.replace("<!-- CUSTOM HEAD -->", &custom_head);
                }

                template
                    .replace("<!-- MODULE LOADER -->", &module_loader(root_name))
                    .into_bytes()
            }
        };

        return Response::builder()
            .header("Content-Type", "text/html")
            .body(Cow::from(body))
            .map_err(From::from);
    } else if request.uri().path() == "/common.js" {
        return Response::builder()
            .header("Content-Type", "text/javascript")
            .body(Cow::from(COMMON_JS.as_bytes()))
            .map_err(From::from);
    }

    // Else, try to serve a file from the filesystem.
    let decoded = urlencoding::decode(request.uri().path().trim_start_matches('/'))
        .expect("expected URL to be UTF-8 encoded");
    let path = PathBuf::from(&*decoded);

    // If the path is relative, we'll try to serve it from the assets directory.
    let mut asset = get_asset_root()
        .unwrap_or_else(|| Path::new(".").to_path_buf())
        .join(&path);

    if !asset.exists() {
        asset = PathBuf::from("/").join(path);
    }

    if asset.exists() {
        return Response::builder()
            .header("Content-Type", get_mime_from_path(&asset)?)
            .body(Cow::from(std::fs::read(asset)?))
            .map_err(From::from);
    }

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Cow::from(String::from("Not Found").into_bytes()))
        .map_err(From::from)
}

#[allow(unreachable_code)]
fn get_asset_root() -> Option<PathBuf> {
    /*
    We're matching exactly how cargo-bundle works.

    - [x] macOS
    - [ ] Windows
    - [ ] Linux (rpm)
    - [ ] Linux (deb)
    - [ ] iOS
    - [ ] Android

    */

    if std::env::var_os("CARGO").is_some() {
        return None;
    }

    // TODO: support for other platforms
    #[cfg(target_os = "macos")]
    {
        let bundle = core_foundation::bundle::CFBundle::main_bundle();
        let bundle_path = bundle.path()?;
        let resources_path = bundle.resources_path()?;
        let absolute_resources_root = bundle_path.join(resources_path);
        let canonical_resources_root = dunce::canonicalize(absolute_resources_root).ok()?;

        return Some(canonical_resources_root);
    }

    None
}

/// Get the mime type from a path-like string
fn get_mime_from_path(trimmed: &Path) -> Result<&'static str> {
    if trimmed.extension().is_some_and(|ext| ext == "svg") {
        return Ok("image/svg+xml");
    }

    let res = match infer::get_from_path(trimmed)?.map(|f| f.mime_type()) {
        Some(f) => {
            if f == "text/plain" {
                get_mime_by_ext(trimmed)
            } else {
                f
            }
        }
        None => get_mime_by_ext(trimmed),
    };

    Ok(res)
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
