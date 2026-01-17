//! Native specific utilities for resolving assets in a bundle. This module is intended for use in renderers that
//! need to resolve asset bundles for resources like images, and fonts.

use http::{status::StatusCode, Response};
use std::path::{Path, PathBuf};

use crate::{AssetPathError, NativeAssetResolveError};

/// An error that can occur when serving an asset.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum AssetServeError {
    /// The asset path could not be resolved.
    #[error("Failed to resolve asset: {0}")]
    ResolveError(#[from] NativeAssetResolveError),

    /// An error occurred while constructing the HTTP response.
    #[error("Failed to construct response: {0}")]
    ResponseError(#[from] http::Error),
}

/// Try to resolve the path of an asset from a given URI path.
pub(crate) fn resolve_native_asset_path(path: &str) -> Result<PathBuf, AssetPathError> {
    #[allow(clippy::unnecessary_lazy_evaluations)]
    resolve_asset_path_from_filesystem(path).ok_or_else(|| {
        #[cfg(target_os = "android")]
        {
            // If the asset exists in the Android asset manager, return the can't be represented as a path
            // error instead
            if to_java_load_asset(path).is_some() {
                return AssetPathError::CannotRepresentAsPath;
            }
        }

        AssetPathError::NotFound
    })
}

/// Try to resolve the path of an asset from a given URI path.
fn resolve_asset_path_from_filesystem(path: &str) -> Option<PathBuf> {
    // If the user provided a custom asset handler, then call it and return the response if the request was handled.
    // The path is the first part of the URI, so we need to trim the leading slash.
    let mut uri_path = PathBuf::from(
        percent_encoding::percent_decode_str(path)
            .decode_utf8()
            .expect("expected URL to be UTF-8 encoded")
            .as_ref(),
    );

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

    // If the asset exists, return it
    uri_path.exists().then_some(uri_path)
}

struct ResolvedAsset {
    mime_type: &'static str,
    body: Vec<u8>,
}

impl ResolvedAsset {
    fn new(mime_type: &'static str, body: Vec<u8>) -> Self {
        Self { mime_type, body }
    }

    fn into_response(self) -> Result<Response<Vec<u8>>, AssetServeError> {
        Ok(Response::builder()
            .header("Content-Type", self.mime_type)
            .header("Access-Control-Allow-Origin", "*")
            .body(self.body)?)
    }
}

/// Read the bytes for an asset
pub(crate) fn resolve_native_asset(path: &str) -> Result<Vec<u8>, NativeAssetResolveError> {
    // Attempt to serve from the asset dir on android using its loader
    #[cfg(target_os = "android")]
    {
        if let Some(asset) = to_java_load_asset(path) {
            return Ok(asset);
        }
    }

    let Some(uri_path) = resolve_asset_path_from_filesystem(path) else {
        return Err(NativeAssetResolveError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Asset not found",
        )));
    };
    Ok(std::fs::read(uri_path)?)
}

/// Resolve the asset and its mime type
fn resolve_asset(path: &str) -> Result<Option<ResolvedAsset>, NativeAssetResolveError> {
    // Attempt to serve from the asset dir on android using its loader
    #[cfg(target_os = "android")]
    {
        if let Some(asset) = to_java_load_asset(path) {
            let extension = path.rsplit_once('.').and_then(|(_, ext)| Some(ext));
            let mime_type = get_mime_from_ext(extension);
            return Ok(Some(ResolvedAsset::new(mime_type, asset)));
        }
    }

    let Some(uri_path) = resolve_asset_path_from_filesystem(path) else {
        return Ok(None);
    };
    let mime_type = get_mime_from_path(&uri_path)?;
    let body = std::fs::read(uri_path)?;
    Ok(Some(ResolvedAsset::new(mime_type, body)))
}

/// Serve an asset from the filesystem or a custom asset handler.
///
/// This method properly accesses the asset directory based on the platform and serves the asset
/// wrapped in an HTTP response.
///
/// Platform specifics:
/// - On the web, this returns AssetServerError since there's no filesystem access. Use `fetch` instead.
/// - On Android, it attempts to load assets using the Android AssetManager.
/// - On other platforms, it serves assets from the filesystem.
pub fn serve_asset(path: &str) -> Result<Response<Vec<u8>>, AssetServeError> {
    match resolve_asset(path)? {
        Some(asset) => asset.into_response(),
        None => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(String::from("Not Found").into_bytes())?),
    }
}

/// Get the asset directory, following tauri/cargo-bundles directory discovery approach
///
/// Currently supports:
/// - [x] macOS
/// - [x] iOS
/// - [x] Windows
/// - [x] Linux (appimage)
/// - [ ] Linux (rpm)
/// - [x] Linux (deb)
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

    #[cfg(target_os = "linux")]
    {
        // In linux bundles, the assets are placed in the lib/$product_name directory
        // bin/
        //   main
        // lib/
        //   $product_name/
        //     assets/
        if let Some(product_name) = dioxus_cli_config::product_name() {
            let lib_asset_path = || {
                let path = cur_exe.parent()?.parent()?.join("lib").join(product_name);
                path.exists().then_some(path)
            };
            if let Some(asset_dir) = lib_asset_path() {
                return asset_dir;
            }
        }
    }

    // For all others, the structure looks like this:
    // app.(exe/appimage)
    //   main.exe
    //   assets/
    cur_exe.parent().unwrap().to_path_buf()
}

/// Get the mime type from a path-like string
fn get_mime_from_path(asset: &Path) -> std::io::Result<&'static str> {
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
    let ext = trimmed.extension().as_ref().and_then(|ext| ext.to_str());
    get_mime_from_ext(ext)
}

/// Get the mime type from a URI using its extension
pub fn get_mime_from_ext(ext: Option<&str>) -> &'static str {
    match ext {
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
