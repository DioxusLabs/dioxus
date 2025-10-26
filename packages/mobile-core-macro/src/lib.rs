#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

mod java_plugin;
use java_plugin::JavaPluginParser;

/// Declare a Java plugin that will be embedded in the binary
///
/// This macro collects Java source files and embeds their metadata into the compiled
/// binary using linker symbols. The Dioxus CLI will extract this metadata and copy the
/// Java files into the Gradle build structure for compilation to DEX.
///
/// # Syntax
///
/// Basic plugin declaration:
/// ```rust,no_run
/// #[cfg(target_os = "android")]
/// dioxus_mobile_core::java_plugin!(
///     package = "dioxus.mobile.geolocation",
///     plugin = "geolocation",
///     files = ["LocationCallback.java", "PermissionsHelper.java"]
/// );
/// ```
///
/// # Parameters
///
/// - `package`: The Java package name (e.g., "dioxus.mobile.geolocation")
/// - `plugin`: The plugin identifier for organization (e.g., "geolocation")
/// - `files`: Array of Java filenames relative to your crate's `src/sys/android/` or `src/android/` directory
///
/// # File Resolution
///
/// The macro searches for Java files in the following locations relative to `CARGO_MANIFEST_DIR`:
/// - `src/sys/android/` (recommended)
/// - `src/android/`
/// - Root directory (last resort)
///
/// If a file is not found, the macro will emit a compile error with details about where it searched.
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

