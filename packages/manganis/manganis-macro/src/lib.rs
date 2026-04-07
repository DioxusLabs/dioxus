#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use std::path::PathBuf;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, ItemStruct,
};

pub(crate) mod asset;
pub(crate) mod css_module;
pub(crate) mod ffi;
pub(crate) mod linker;

use crate::css_module::{expand_css_module_struct, CssModuleAttribute};

/// The asset macro collects assets that will be included in the final binary
///
/// # Files
///
/// The file builder collects an arbitrary file. Relative paths are resolved relative to the package root
/// ```rust
/// # use manganis::{asset, Asset};
/// const _: Asset = asset!("/assets/asset.txt");
/// ```
/// Macros like `concat!` and `env!` are supported in the asset path.
/// ```rust
/// # use manganis::{asset, Asset};
/// const _: Asset = asset!(concat!("/assets/", env!("CARGO_CRATE_NAME"), ".dat"));
/// ```
///
/// # Images
///
/// You can collect images which will be automatically optimized with the image builder:
/// ```rust
/// # use manganis::{asset, Asset};
/// const _: Asset = asset!("/assets/image.png");
/// ```
/// Resize the image at compile time to make the assets file size smaller:
/// ```rust
/// # use manganis::{asset, Asset, AssetOptions, ImageSize};
/// const _: Asset = asset!("/assets/image.png", AssetOptions::image().with_size(ImageSize::Manual { width: 52, height: 52 }));
/// ```
/// Or convert the image at compile time to a web friendly format:
/// ```rust
/// # use manganis::{asset, Asset, AssetOptions, ImageSize, ImageFormat};
/// const _: Asset = asset!("/assets/image.png", AssetOptions::image().with_format(ImageFormat::Avif));
/// ```
/// You can mark images as preloaded to make them load faster in your app
/// ```rust
/// # use manganis::{asset, Asset, AssetOptions};
/// const _: Asset = asset!("/assets/image.png", AssetOptions::image().with_preload(true));
/// ```
#[proc_macro]
pub fn asset(input: TokenStream) -> TokenStream {
    let asset = parse_macro_input!(input as asset::AssetParser);

    quote! { #asset }.into_token_stream().into()
}

/// Resolve an asset at compile time, returning `None` if the asset does not exist.
///
/// This behaves like the `asset!` macro when the asset can be resolved, but mirrors
/// [`option_env!`](core::option_env) by returning an `Option` instead of emitting a compile error
/// when the asset is missing.
///
/// ```rust
/// # use manganis::{asset, option_asset, Asset};
/// const REQUIRED: Asset = asset!("/assets/style.css");
/// const OPTIONAL: Option<Asset> = option_asset!("/assets/maybe.css");
/// ```
#[proc_macro]
pub fn option_asset(input: TokenStream) -> TokenStream {
    let asset = parse_macro_input!(input as asset::AssetParser);

    asset.expand_option_tokens().into()
}

/// Generate type-safe styles with scoped CSS class names.
///
/// The `css_module` attribute macro creates scoped CSS modules that prevent class name collisions
/// by making each class globally unique. It expands the annotated struct to provide type-safe
/// identifiers for your CSS classes, allowing you to reference styles in your Rust code with
/// compile-time guarantees.
///
/// # Syntax
///
/// The `css_module` attribute takes:
/// - The asset string path - the absolute path (from the crate root) to your CSS file.
/// - Optional `AssetOptions` to configure the processing of your CSS module.
///
/// It must be applied to a unit struct:
/// ```rust, ignore
/// #[css_module("/assets/my-styles.css")]
/// struct Styles;
///
/// #[css_module("/assets/my-styles.css", AssetOptions::css_module().with_minify(true))]
/// struct Styles;
/// ```
///
/// # Generation
///
/// The `css_module` attribute macro does two things:
/// - It generates an asset and automatically inserts it as a stylesheet link in the document.
/// - It expands the annotated struct with snake-case associated constants for your CSS class names.
///
/// ```rust, ignore
/// // This macro usage:
/// #[css_module("/assets/mycss.css")]
/// struct Styles;
///
/// // Will expand the struct to (simplified):
/// struct Styles {}
///
/// impl Styles {
///     // Snake-cased class names can be accessed like this:
///     pub const your_class: &str = "your_class-a1b2c3";
/// }
/// ```
///
/// # CSS Class Name Scoping
///
/// **The macro only processes CSS class selectors (`.class-name`).** Other selectors like IDs (`#id`),
/// element selectors (`div`, `p`), attribute selectors, etc. are left unchanged and not exposed as
/// Rust constants.
///
/// The macro collects all class selectors in your CSS file and transforms them to be globally unique
/// by appending a hash. For example, `.myClass` becomes `.myClass-a1b2c3` where `a1b2c3` is a hash
/// of the file path.
///
/// Class names are converted to snake_case for the Rust constants. For example:
/// - `.fooBar` becomes `Styles::foo_bar`
/// - `.my-class` becomes `Styles::my_class`
///
/// To prevent a class from being scoped, wrap it in `:global()`:
/// ```css
/// /* This class will be scoped */
/// .my-class { color: blue; }
///
/// /* This class will NOT be scoped (no hash added) */
/// :global(.global-class) { color: red; }
///
/// /* Element selectors and other CSS remain unchanged */
/// div { margin: 0; }
/// #my-id { padding: 10px; }
/// ```
///
/// # Using Multiple CSS Modules
///
/// Multiple `css_module` attributes can be used in the same scope by applying them to different structs:
/// ```rust, ignore
/// // First CSS module
/// #[css_module("/assets/styles1.css")]
/// struct Styles;
///
/// // Second CSS module with a different struct name
/// #[css_module("/assets/styles2.css")]
/// struct OtherStyles;
///
/// // Access classes from both:
/// rsx! {
///     div { class: Styles::container }
///     div { class: OtherStyles::button }
/// }
/// ```
///
/// # Asset Options
///
/// Similar to the `asset!()` macro, you can pass optional `AssetOptions` to configure processing:
/// ```rust, ignore
/// #[css_module(
///     "/assets/mycss.css",
///     AssetOptions::css_module()
///         .with_minify(true)
///         .with_preload(false)
/// )]
/// struct Styles;
/// ```
///
/// # Example
///
/// First create a CSS file:
/// ```css
/// /* assets/styles.css */
///
/// .container {
///     padding: 20px;
/// }
///
/// .button {
///     background-color: #373737;
/// }
///
/// :global(.global-text) {
///     font-weight: bold;
/// }
/// ```
///
/// Then use the `css_module` attribute:
/// ```rust, ignore
/// use dioxus::prelude::*;
///
/// fn app() -> Element {
///     #[css_module("/assets/styles.css")]
///     struct Styles;
///
///     rsx! {
///         div { class: Styles::container,
///             button { class: Styles::button, "Click me" }
///             span { class: Styles::global_text, "This uses global class" }
///         }
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn css_module(input: TokenStream, item: TokenStream) -> TokenStream {
    let attribute = parse_macro_input!(input as CssModuleAttribute);
    let item_struct = parse_macro_input!(item as ItemStruct);
    let mut tokens = proc_macro2::TokenStream::new();
    expand_css_module_struct(&mut tokens, &attribute, &item_struct);
    tokens.into()
}

/// Generate FFI bindings between Rust and native platforms (Swift/Kotlin)
///
/// This attribute macro parses an `extern "Swift"` or `extern "Kotlin"` block and generates:
/// 1. Opaque type wrappers for foreign types
/// 2. Function implementations with direct JNI/ObjC bindings
/// 3. Linker metadata for the CLI to compile the native source
///
/// # Syntax
///
/// ```rust,ignore
/// #[manganis::ffi("/src/ios")]
/// extern "Swift" {
///     pub type GeolocationPlugin;
///     pub fn get_position(this: &GeolocationPlugin, high_accuracy: bool) -> Option<String>;
/// }
///
/// #[manganis::ffi("/src/android")]
/// extern "Kotlin" {
///     pub type GeolocationPlugin;
///     pub fn get_position(this: &GeolocationPlugin, high_accuracy: bool) -> Option<String>;
/// }
/// ```
///
/// # Path Parameter
///
/// The path in the attribute specifies the native source folder relative to `CARGO_MANIFEST_DIR`:
/// - For Swift: A SwiftPM package folder containing `Package.swift`
/// - For Kotlin: A Gradle project folder containing `build.gradle.kts`
///
/// # Type Declarations
///
/// Use `type Name;` to declare opaque foreign types. These become Rust structs wrapping
/// the native object handle (GlobalRef for JNI, raw pointer for ObjC).
///
/// # Function Declarations
///
/// Functions can be:
/// - **Instance methods**: First argument is `this: &TypeName`
/// - **Static methods**: No `this` argument
///
/// # Supported Types
///
/// - Primitives: `bool`, `i8`-`i64`, `u8`-`u64`, `f32`, `f64`
/// - Strings: `String`, `&str`
/// - Options: `Option<T>` where T is supported
/// - Opaque refs: `&TypeName` for foreign type references
#[proc_macro_attribute]
pub fn ffi(attr: TokenStream, item: TokenStream) -> TokenStream {
    use ffi::{FfiAttribute, FfiBridgeParser};

    let attr = parse_macro_input!(attr as FfiAttribute);
    let item = parse_macro_input!(item as syn::ItemForeignMod);

    match FfiBridgeParser::parse_with_attr(attr, item) {
        Ok(parser) => parser.generate().into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn resolve_path(raw: &str, span: Span) -> Result<PathBuf, AssetParseError> {
    // Get the location of the root of the crate which is where all assets are relative to
    //
    // IE
    // /users/dioxus/dev/app/
    // is the root of
    // /users/dioxus/dev/app/assets/blah.css
    let manifest_dir = dunce::canonicalize(
        std::env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap(),
    )
    .unwrap();

    // 1. the input file should be a pathbuf
    let input = PathBuf::from(raw);

    let path = if raw.starts_with('.') {
        if let Some(local_folder) = span.local_file().as_ref().and_then(|f| f.parent()) {
            local_folder.join(raw)
        } else {
            // If we are running in rust analyzer, just assume the path is valid and return an error when
            // we compile if it doesn't exist
            if looks_like_rust_analyzer(&span) {
                return Ok(
                    "The asset macro was expanded under Rust Analyzer which doesn't support paths or local assets yet"
                        .into(),
                );
            }

            // Otherwise, return an error about the version of rust required for relative assets
            return Err(AssetParseError::RelativeAssetPath);
        }
    } else {
        manifest_dir.join(raw.trim_start_matches('/'))
    };

    // 2. absolute path to the asset
    let Ok(path) = std::path::absolute(path) else {
        return Err(AssetParseError::InvalidPath {
            path: input.clone(),
        });
    };

    // 3. Ensure the path exists
    let Ok(path) = dunce::canonicalize(path) else {
        return Err(AssetParseError::AssetDoesntExist {
            path: input.clone(),
        });
    };

    // 4. Ensure the path doesn't escape the crate dir
    //
    // - Note: since we called canonicalize on both paths, we can safely compare the parent dirs.
    //   On windows, we can only compare the prefix if both paths are canonicalized (not just absolute)
    //   https://github.com/rust-lang/rust/issues/42869
    if path == manifest_dir || !path.starts_with(manifest_dir) {
        return Err(AssetParseError::InvalidPath { path });
    }

    Ok(path)
}

/// Parse `T`, while also collecting the tokens it was parsed from.
fn parse_with_tokens<T: Parse>(input: ParseStream) -> syn::Result<(T, proc_macro2::TokenStream)> {
    let begin = input.cursor();
    let t: T = input.parse()?;
    let end = input.cursor();

    let mut cursor = begin;
    let mut tokens = proc_macro2::TokenStream::new();
    while cursor != end {
        let (tt, next) = cursor.token_tree().unwrap();
        tokens.extend(std::iter::once(tt));
        cursor = next;
    }

    Ok((t, tokens))
}

#[derive(Debug)]
enum AssetParseError {
    AssetDoesntExist { path: PathBuf },
    InvalidPath { path: PathBuf },
    RelativeAssetPath,
}

impl std::fmt::Display for AssetParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetParseError::AssetDoesntExist { path } => {
                write!(f, "Asset at {} doesn't exist", path.display())
            }
            AssetParseError::InvalidPath { path } => {
                write!(
                    f,
                    "Asset path {} is invalid. Make sure the asset exists within this crate.",
                    path.display()
                )
            }
            AssetParseError::RelativeAssetPath => write!(f, "Failed to resolve relative asset path. Relative assets are only supported in rust 1.88+."),
        }
    }
}

/// Rust analyzer doesn't provide a stable way to detect if macros are running under it.
/// This function uses heuristics to determine if we are running under rust analyzer for better error
/// messages.
fn looks_like_rust_analyzer(span: &Span) -> bool {
    // Rust analyzer spans have a struct debug impl compared to rustcs custom debug impl
    // RA Example: SpanData { range: 45..58, anchor: SpanAnchor(EditionedFileId(0, Edition2024), ErasedFileAstId { kind: Fn, index: 0, hash: 9CD8 }), ctx: SyntaxContext(4294967036) }
    // Rustc Example: #0 bytes(70..83)
    let looks_like_rust_analyzer_span = format!("{:?}", span).contains("ctx:");
    // The rust analyzer macro expander runs under RUST_ANALYZER_INTERNALS_DO_NOT_USE
    let looks_like_rust_analyzer_env = std::env::var("RUST_ANALYZER_INTERNALS_DO_NOT_USE").is_ok();
    // The rust analyzer executable is named rust-analyzer-proc-macro-srv
    let looks_like_rust_analyzer_exe = std::env::current_exe().ok().is_some_and(|p| {
        p.file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.contains("rust-analyzer"))
    });
    looks_like_rust_analyzer_span || looks_like_rust_analyzer_env || looks_like_rust_analyzer_exe
}
