use std::path::{Path, PathBuf};
use wry::{
    http::{status::StatusCode, Request, Response, ResponseBuilder},
    Result,
};

const MODULE_LOADER: &str = r#"
<script>
    import("./index.js").then(function (module) {
    module.main();
    });
</script>
"#;

pub(super) fn desktop_handler(
    request: &Request,
    asset_root: Option<PathBuf>,
    custom_head: Option<String>,
    custom_index: Option<String>,
) -> Result<Response> {
    // Any content that uses the `dioxus://` scheme will be shuttled through this handler as a "special case".
    // For now, we only serve two pieces of content which get included as bytes into the final binary.
    let path = request.uri().replace("dioxus://", "");

    // all assets should be called from index.html
    let trimmed = path.trim_start_matches("index.html/");

    if trimmed.is_empty() {
        // If a custom index is provided, just defer to that, expecting the user to know what they're doing.
        // we'll look for the closing </body> tag and insert our little module loader there.
        if let Some(custom_index) = custom_index {
            let rendered = custom_index
                .replace("</body>", &format!("{}</body>", MODULE_LOADER))
                .into_bytes();
            ResponseBuilder::new().mimetype("text/html").body(rendered)
        } else {
            // Otherwise, we'll serve the default index.html and apply a custom head if that's specified.
            let mut template = include_str!("./index.html").to_string();
            if let Some(custom_head) = custom_head {
                template = template.replace("<!-- CUSTOM HEAD -->", &custom_head);
            }
            template = template.replace("<!-- MODULE LOADER -->", MODULE_LOADER);

            ResponseBuilder::new()
                .mimetype("text/html")
                .body(template.into_bytes())
        }
    } else if trimmed == "index.js" {
        ResponseBuilder::new()
            .mimetype("text/javascript")
            .body(dioxus_interpreter_js::INTERPRETER_JS.as_bytes().to_vec())
    } else {
        let asset_root = asset_root
            .unwrap_or_else(|| get_asset_root().unwrap_or_else(|| Path::new(".").to_path_buf()))
            .canonicalize()?;

        let asset = asset_root.join(trimmed).canonicalize()?;

        if !asset.starts_with(asset_root) {
            return ResponseBuilder::new()
                .status(StatusCode::FORBIDDEN)
                .body(String::from("Forbidden").into_bytes());
        }

        if !asset.exists() {
            return ResponseBuilder::new()
                .status(StatusCode::NOT_FOUND)
                .body(String::from("Not Found").into_bytes());
        }

        let mime = mime_guess::from_path(&asset).first_or_octet_stream();

        // do not let path searching to go two layers beyond the caller level
        let data = std::fs::read(asset)?;
        let meta = format!("{}", mime);

        ResponseBuilder::new().mimetype(&meta).body(data)
    }
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
        let bundle_path = dbg!(bundle.path()?);
        let resources_path = dbg!(bundle.resources_path()?);
        let absolute_resources_root = dbg!(bundle_path.join(resources_path));
        let canonical_resources_root = dbg!(dunce::canonicalize(absolute_resources_root).ok()?);

        return Some(canonical_resources_root);
    }

    None
}
