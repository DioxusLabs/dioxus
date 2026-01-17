#![warn(missing_docs)]
//! The asset resolver for the Dioxus bundle format. Each platform has its own way of resolving assets. This crate handles
//! resolving assets in a cross-platform way.
//!
//! There are two broad locations for assets depending on the platform:
//! - **Web**: Assets are stored on a remote server and fetched via HTTP requests.
//! - **Native**: Assets are read from the local bundle. Each platform has its own bundle structure which may store assets
//!   as a file at a specific path or in an opaque format like Android's AssetManager.
//!
//! [`read_asset_bytes`]( abstracts over both of these methods, allowing you to read the bytes of an asset
//! regardless of the platform.
//!
//! If you know you are on a desktop platform, you can use [`asset_path`] to resolve the path of an asset and read
//! the contents with [`std::fs`].
//!
//! ## Example
//! ```rust
//! # async fn asset_example() {
//! use dioxus::prelude::*;
//!
//! // Bundle the static JSON asset into the application
//! static JSON_ASSET: Asset = asset!("/assets/data.json");
//!
//! // Read the bytes of the JSON asset
//! let bytes = dioxus::asset_resolver::read_asset_bytes(&JSON_ASSET).await.unwrap();
//!
//! // Deserialize the JSON data
//! let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
//! assert_eq!(json["key"].as_str(), Some("value"));
//! # }
//! ```

use std::{fmt::Debug, path::PathBuf};

#[cfg(feature = "native")]
pub mod native;

#[cfg(feature = "web")]
mod web;

/// An error that can occur when resolving an asset to a path. Not all platforms can represent assets as paths,
/// an error may mean that the asset doesn't exist or it cannot be represented as a path.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum AssetPathError {
    /// The asset was not found by the resolver.
    #[error("Failed to find the path in the asset directory")]
    NotFound,

    /// The asset may exist, but it cannot be represented as a path.
    #[error("Asset cannot be represented as a path")]
    CannotRepresentAsPath,
}

/// Tries to resolve the path of an asset from a given URI path. Depending on the platform, this may
/// return an error even if the asset exists because some platforms cannot represent assets as paths.
/// You should prefer [`read_asset_bytes`] to read the asset bytes directly
/// for cross-platform compatibility.
///
/// ## Platform specific behavior
///
/// This function will only work on desktop platforms. It will always return an error in web and Android
/// bundles. On Android assets are bundled in the APK, and cannot be represented as paths. In web bundles,
/// Assets are fetched via HTTP requests and don't have a filesystem path.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
///
/// // Bundle the static JSON asset into the application
/// static JSON_ASSET: Asset = asset!("/assets/data.json");
///
/// // Resolve the path of the asset. This will not work in web or Android bundles
/// let path = dioxus::asset_resolver::asset_path(&JSON_ASSET).unwrap();
///
/// println!("Asset path: {:?}", path);
///
/// // Read the bytes of the JSON asset
/// let bytes = std::fs::read(path).unwrap();
///
/// // Deserialize the JSON data
/// let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
/// assert_eq!(json["key"].as_str(), Some("value"));
/// ```
///
/// ## Resolving assets from a folder
///
/// To resolve an asset from a folder, you can pass the path of the file joined with your folder asset as a string:
/// ```rust
/// # async fn asset_example() {
/// use dioxus::prelude::*;
///
/// // Bundle the whole assets folder into the application
/// static ASSETS: Asset = asset!("/assets");
///
/// // Resolve the path of the asset. This will not work in web or Android bundles
/// let path = dioxus::asset_resolver::asset_path(format!("{ASSETS}/data.json")).unwrap();
///
/// println!("Asset path: {:?}", path);
///
/// // Read the bytes of the JSON asset
/// let bytes = std::fs::read(path).unwrap();
///
/// // Deserialize the JSON data
/// let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
/// assert_eq!(json["key"].as_str(), Some("value"));
/// # }
/// ```
#[allow(unused)]
pub fn asset_path(asset: impl ToString) -> Result<PathBuf, AssetPathError> {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    return Err(AssetPathError::CannotRepresentAsPath);

    #[cfg(feature = "native")]
    return native::resolve_native_asset_path(asset.to_string().as_str());

    Err(AssetPathError::NotFound)
}

/// An error that can occur when resolving an asset.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum AssetResolveError {
    /// An error occurred while resolving a native asset.
    #[error("Failed to resolve native asset: {0}")]
    Native(#[from] NativeAssetResolveError),

    /// An error occurred while resolving a web asset.
    #[error("Failed to resolve web asset: {0}")]
    Web(#[from] WebAssetResolveError),

    /// An error that occurs when no asset resolver is available for the current platform.
    #[error("Asset resolution is not supported on this platform")]
    UnsupportedPlatform,
}

/// Read the bytes of an asset. This will work on both web and native platforms. On the web,
/// it will fetch the asset via HTTP, and on native platforms, it will read the asset from the filesystem or bundle.
///
/// ## Errors
/// This function will return an error if the asset cannot be found or if it fails to read which may be due to I/O errors or
/// network issues.
///
/// ## Example
///
/// ```rust
/// # async fn asset_example() {
/// use dioxus::prelude::*;
///
/// // Bundle the static JSON asset into the application
/// static JSON_ASSET: Asset = asset!("/assets/data.json");
///
/// // Read the bytes of the JSON asset
/// let bytes = dioxus::asset_resolver::read_asset_bytes(&JSON_ASSET).await.unwrap();
///
/// // Deserialize the JSON data
/// let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
/// assert_eq!(json["key"].as_str(), Some("value"));
/// # }
/// ```
///
/// ## Loading assets from a folder
///
/// To load an asset from a folder, you can pass the path of the file joined with your folder asset as a string:
/// ```rust
/// # async fn asset_example() {
/// use dioxus::prelude::*;
///
/// // Bundle the whole assets folder into the application
/// static ASSETS: Asset = asset!("/assets");
///
/// // Read the bytes of the JSON asset
/// let bytes = dioxus::asset_resolver::read_asset_bytes(format!("{ASSETS}/data.json")).await.unwrap();
///
/// // Deserialize the JSON data
/// let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
/// assert_eq!(json["key"].as_str(), Some("value"));
/// # }
/// ```
#[allow(unused)]
pub async fn read_asset_bytes(asset: impl ToString) -> Result<Vec<u8>, AssetResolveError> {
    let path = asset.to_string();

    #[cfg(feature = "web")]
    return web::resolve_web_asset(&path)
        .await
        .map_err(AssetResolveError::Web);

    #[cfg(feature = "native")]
    return tokio::task::spawn_blocking(move || native::resolve_native_asset(&path))
        .await
        .map_err(|err| AssetResolveError::Native(NativeAssetResolveError::JoinError(err)))
        .and_then(|result| result.map_err(AssetResolveError::Native));

    Err(AssetResolveError::UnsupportedPlatform)
}

/// An error that occurs when resolving a native asset.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum NativeAssetResolveError {
    /// An I/O error occurred while reading the asset from the filesystem.
    #[error("Failed to read asset: {0}")]
    IoError(#[from] std::io::Error),

    /// The asset resolver failed to complete and could not be joined.
    #[cfg(feature = "native")]
    #[error("Asset resolver join failed: {0}")]
    JoinError(tokio::task::JoinError),
}

/// An error that occurs when resolving an asset on the web.
pub struct WebAssetResolveError {
    #[cfg(feature = "web")]
    error: js_sys::Error,
}

impl Debug for WebAssetResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("WebAssetResolveError");
        #[cfg(feature = "web")]
        debug.field("name", &self.error.name());
        #[cfg(feature = "web")]
        debug.field("message", &self.error.message());
        debug.finish()
    }
}

impl std::fmt::Display for WebAssetResolveError {
    #[allow(unreachable_code)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(feature = "web")]
        return write!(f, "{}", self.error.message());
        write!(f, "WebAssetResolveError")
    }
}

impl std::error::Error for WebAssetResolveError {}
