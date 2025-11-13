#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

mod android_plugin;
mod ios_plugin;

/// Declare an Android plugin that will be embedded in the binary
///
/// This macro collects Java source files and embeds their metadata into the compiled
/// binary using linker symbols. The Dioxus CLI will extract this metadata and copy the
/// Java files into the Gradle build structure for compilation to DEX.
///
/// # Syntax
///
/// Basic plugin declaration with full relative paths:
/// ```rust,no_run
/// #[cfg(target_os = "android")]
/// dioxus_platform_bridge::android_plugin!(
///     package = "dioxus.mobile.geolocation",
///     plugin = "geolocation",
///     files = [
///         "src/sys/android/LocationCallback.java",
///         "src/sys/android/PermissionsHelper.java"
///     ]
/// );
/// ```
///
/// # Parameters
///
/// - `package`: The Java package name (e.g., "dioxus.mobile.geolocation")
/// - `plugin`: The plugin identifier for organization (e.g., "geolocation")
/// - `files`: Array of Java file paths relative to `CARGO_MANIFEST_DIR` (e.g., "src/sys/android/File.java")
///
/// # File Paths
///
/// File paths should be specified relative to your crate's manifest directory (`CARGO_MANIFEST_DIR`).
/// Common directory structures include:
/// - `src/sys/android/`
/// - `src/android/`
/// - Any other directory structure you prefer
///
/// The macro will resolve these paths at compile time using `env!("CARGO_MANIFEST_DIR")`.
///
/// # Embedding
///
/// The macro embeds absolute file paths into the binary using linker symbols with the
/// `__JAVA_SOURCE__` prefix. This allows the Dioxus CLI to directly locate and copy Java
/// source files without searching the workspace at build time.
///
/// # Example Structure
///
/// ```text
/// your-plugin-crate/
/// └── src/
///     ├── lib.rs                  # Contains android_plugin!() macro invocation
///     └── sys/
///         └── android/
///             ├── LocationCallback.java    # Java plugin sources
///             └── PermissionsHelper.java
/// ```
#[proc_macro]
pub fn android_plugin(input: TokenStream) -> TokenStream {
    let android_plugin = parse_macro_input!(input as android_plugin::AndroidPluginParser);

    quote! { #android_plugin }.into()
}

/// Declare an iOS/macOS plugin that will be embedded in the binary
///
/// This macro declares Swift packages and embeds their metadata into the compiled
/// binary using linker symbols. The Dioxus CLI uses this metadata to ensure the Swift
/// runtime is bundled correctly.
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
/// # Embedding
///
/// The macro embeds package metadata into the binary using linker symbols with the
/// `__SWIFT_SOURCE__` prefix. This allows the Dioxus CLI to detect when Swift support
/// (stdlib embedding, diagnostics, etc.) is required.
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
