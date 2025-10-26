#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

mod ios_plugin;
mod java_plugin;
use ios_plugin::IosPluginParser;
use java_plugin::JavaPluginParser;

/// Declare an iOS framework plugin that will be embedded in the binary
///
/// This macro declares which iOS frameworks your crate requires. While the frameworks
/// are linked automatically by objc2 at compile time, this macro allows you to declare
/// framework dependencies in a clean, declarative way similar to `java_plugin!`.
///
/// # Syntax
///
/// Basic plugin declaration:
/// ```rust,no_run
/// #[cfg(target_os = "ios")]
/// dioxus_platform_bridge::ios_plugin!(
///     plugin = "geolocation",
///     frameworks = ["CoreLocation", "Foundation"]
/// );
/// ```
///
/// # Parameters
///
/// - `plugin`: The plugin identifier for organization (e.g., "geolocation")
/// - `frameworks`: Array of iOS framework names (e.g., ["CoreLocation", "Foundation"])
///
/// # Embedding
///
/// The macro embeds framework metadata into the binary using linker symbols with the
/// `__IOS_FRAMEWORK__` prefix. This allows documentation and tooling to understand
/// which frameworks your crate requires.
///
/// # Note
///
/// This macro is primarily for documentation and metadata purposes. The actual framework
/// linking is handled automatically by objc2 when you use its APIs.
#[proc_macro]
pub fn ios_plugin(input: TokenStream) -> TokenStream {
    let ios_plugin = parse_macro_input!(input as IosPluginParser);
    
    quote! { #ios_plugin }.into()
}

/// Declare a Java plugin that will be embedded in the binary
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
/// dioxus_platform_bridge::java_plugin!(
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
///     ├── lib.rs                  # Contains java_plugin!() macro invocation
///     └── sys/
///         └── android/
///             ├── LocationCallback.java    # Java plugin sources
///             └── PermissionsHelper.java
/// ```
#[proc_macro]
pub fn java_plugin(input: TokenStream) -> TokenStream {
    let java_plugin = parse_macro_input!(input as JavaPluginParser);
    
    quote! { #java_plugin }.into()
}

