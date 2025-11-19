#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

mod android_plugin;
mod ios_plugin;

/// Declare an Android plugin that will be embedded in the binary
///
/// This macro declares prebuilt Android artifacts (AARs) and embeds their metadata into the compiled
/// binary using the shared `SymbolData` stream (the same linker section used for assets and
/// permissions). The Dioxus CLI reads that metadata to copy the AARs into the generated Gradle
/// project and to append any additional Gradle dependencies.
///
/// # Syntax
///
/// Basic plugin declaration with full relative paths:
/// ```rust,no_run
/// #[cfg(target_os = "android")]
/// dioxus_platform_bridge::android_plugin!(
///     plugin = "geolocation",
///     aar = { path = "android/build/outputs/aar/geolocation-plugin-release.aar" }
/// );
/// ```
///
/// # Parameters
///
/// - `plugin`: The plugin identifier for organization (e.g., "geolocation")
/// - `aar`: A block with either `{ path = "relative/path/to.aar" }` or `{ env = "ENV_WITH_PATH" }`
///
/// When `path` is used, it is resolved relative to `CARGO_MANIFEST_DIR`. When `env` is used,
/// the environment variable is read at compile time via `env!`.
///
/// The macro wraps the resolved artifact path and dependency strings in
/// `SymbolData::AndroidArtifact` and stores it under the `__ASSETS__*` linker prefix. Because the CLI
/// already scans that prefix for assets and permissions, no extra scanner is required.
///
/// # Example Structure
///
/// ```text
/// your-plugin-crate/
/// └── android/
///     ├── build.gradle.kts        # Builds the AAR
///     ├── settings.gradle.kts
///     └── build/outputs/aar/
///         └── geolocation-plugin-release.aar
/// ```
#[proc_macro]
pub fn android_plugin(input: TokenStream) -> TokenStream {
    let android_plugin = parse_macro_input!(input as android_plugin::AndroidPluginParser);

    quote! { #android_plugin }.into()
}

/// Declare an iOS/macOS plugin that will be embedded in the binary
///
/// This macro declares Swift packages and embeds their metadata into the compiled binary using the
/// shared `SymbolData` stream. The Dioxus CLI uses this metadata to ensure the Swift runtime is
/// bundled correctly whenever Swift code is linked.
///
/// # Syntax
///
/// Basic plugin declaration:
/// ```rust,no_run
/// #[cfg(any(target_os = "ios", target_os = "macos"))]
/// dioxus_platform_bridge::ios_plugin!(
///     plugin = "geolocation",
///     spm = { path = "ios", product = "GeolocationPlugin" }
/// );
/// ```
///
/// # Parameters
///
/// - `plugin`: The plugin identifier for organization (e.g., "geolocation")
/// - `spm`: A Swift Package declaration with `{ path = "...", product = "MyPlugin" }` relative to
///   `CARGO_MANIFEST_DIR`.
///
/// The macro expands paths using `env!("CARGO_MANIFEST_DIR")` so package manifests are
/// resolved relative to the crate declaring the plugin.
///
/// The metadata is serialized as `SymbolData::SwiftPackage` and emitted under the `__ASSETS__*`
/// prefix, alongside assets, permissions, and Android artifacts.
///
/// # Example Structure
///
/// ```text
/// your-plugin-crate/
/// └── ios/
///     ├── Package.swift
///     └── Sources/
///         └── GeolocationPlugin.swift
/// ```
#[proc_macro]
pub fn ios_plugin(input: TokenStream) -> TokenStream {
    let ios_plugin = parse_macro_input!(input as ios_plugin::IosPluginParser);

    quote! { #ios_plugin }.into()
}
