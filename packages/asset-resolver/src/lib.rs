#![warn(missing_docs)]
//! Asset resolver for Dioxus applications.

#[cfg(feature = "native")]
mod native;
use std::{fmt::Debug, path::PathBuf};

#[cfg(feature = "native")]
pub use native::*;

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

/// Platform behavior:
/// - Desktop platforms (Linux, macOS, Windows): Resolves assets from the filesystem.
/// - Android: Assets are bundled in the APK, they cannot be represented as paths.
/// - Web: Assets are fetched via HTTP requests, they cannot be represented as paths.
#[allow(unused)]
pub fn resolve_asset_path(path: &str) -> Result<PathBuf, AssetPathError> {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    return Err(AssetPathError::CannotRepresentAsPath);

    #[cfg(feature = "native")]
    return native::resolve_native_asset_path(path);

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

/// Read the bytes for an asset
#[allow(unreachable_code)]
pub async fn resolve_asset(path: &str) -> Result<Vec<u8>, AssetResolveError> {
    #[cfg(feature = "web")]
    return web::resolve_web_asset(path)
        .await
        .map_err(AssetResolveError::Web);

    #[cfg(feature = "native")]
    return tokio::task::spawn_blocking(move || native::resolve_native_asset(path))
        .await
        .map_err(|err| AssetResolveError::Native(NativeAssetResolveError::JoinError(err)))
        .and_then(|result| result.map_err(AssetResolveError::Native));

    Err(AssetResolveError::UnsupportedPlatform)
}

/// An error that occurs when resolving a native asset.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum NativeAssetResolveError {
    /// An IO error occurred while reading the asset from the filesystem.
    #[error("Failed to serve asset: {0}")]
    IoError(#[from] std::io::Error),

    /// The asset resolver failed to complete and could not be joined.
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
        let mut debug = f.debug_struct("WebAssetFetchError");
        #[cfg(feature = "web")]
        debug.field("name", &self.error.name());
        #[cfg(feature = "web")]
        debug.field("message", &self.error.message());
        debug.finish()
    }
}

impl std::fmt::Display for WebAssetResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(not(feature = "web"))]
        write!(f, "WebAssetResolveError")?;
        #[cfg(feature = "web")]
        write!(f, "{}", self.error.message())
    }
}

impl std::error::Error for WebAssetResolveError {}
