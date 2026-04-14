#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use std::path::{Component, PathBuf};

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::{
    ItemStruct,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

pub(crate) mod asset;
pub(crate) mod css_module;
pub(crate) mod ffi;
pub(crate) mod linker;

use crate::css_module::{CssModuleAttribute, expand_css_module_struct};

/// The asset macro collects assets that will be included in the final binary
///
/// # Files
///
/// The file builder collects an arbitrary file.
/// ```rust
/// # use manganis::{asset, Asset};
/// const _: Asset = asset!("/assets/asset.txt");
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
///
/// # Path resolution
///
/// Paths are resolved relative to the current file if they begin with either `.` or `..`.
/// ```rust
/// # use manganis::{asset, Asset};
/// const _: Asset = asset!("./asset.txt");
/// const _: Asset = asset!("../assets/asset.txt");
/// ```
/// If a path points to within the output directory, the full path is used.
/// The output directory is the value of the `OUT_DIR` environment variable.
/// ```rust, ignore
/// const _: Asset = asset!(concat!(env!("OUT_DIR"), "/generated-asset.txt"));
/// ```
/// Otherwise, paths are resolved relative to the crate root.
/// Leading `/` characters are ignored.
/// ```rust
/// # use manganis::{asset, Asset};
/// const _: Asset = asset!("assets/asset.txt");
/// const _: Asset = asset!("/assets/asset.txt");
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
/// - The asset path. Uses the same rules for resolution as `asset!`.
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

struct PathResolver {
    src: PathBuf,
    manifest_dir: PathBuf,
    out_dir: Option<PathBuf>,
    file_path: Option<PathBuf>,
    looks_like_rust_analyzer: bool,
}

impl PathResolver {
    fn new(src: impl Into<PathBuf>, span: &Span) -> Self {
        let manifest_dir = Self::get_manifest_dir();

        // The output directory is only available when a build script is present
        let out_dir = std::env::var("OUT_DIR")
            .ok()
            .map(dunce::canonicalize)
            .transpose()
            .unwrap();

        Self {
            src: src.into(),
            manifest_dir,
            out_dir,
            file_path: span.local_file(),
            looks_like_rust_analyzer: looks_like_rust_analyzer(span),
        }
    }

    fn get_manifest_dir() -> PathBuf {
        dunce::canonicalize(std::env::var("CARGO_MANIFEST_DIR").unwrap()).unwrap()
    }

    fn resolve(self) -> Result<PathBuf, AssetParseError> {
        // 1. Resolve path
        let path = if self
            .out_dir
            .as_ref()
            .is_some_and(|out_dir| self.src.starts_with(out_dir))
        {
            self.src
        } else if self.src.components().next().is_some_and(|component| {
            component == Component::CurDir || component == Component::ParentDir
        }) {
            if let Some(parent) = self.file_path.as_ref().and_then(|path| path.parent()) {
                parent.join(self.src)
            } else {
                // If we are running in rust analyzer, just assume the path is valid and return an error when
                // we compile if it doesn't exist
                if self.looks_like_rust_analyzer {
                    let message = concat!(
                        "The asset macro was expanded under Rust Analyzer ",
                        "which doesn't support paths or local assets yet."
                    );

                    return Ok(message.into());
                }

                // Otherwise, return an error about the version of rust required for relative assets
                return Err(AssetParseError::FileBaseUnavailable);
            }
        } else {
            self.manifest_dir
                .join(self.src.strip_prefix("/").unwrap_or(self.src.as_path()))
        };

        // 2. Convert to absolute path
        let Ok(path) = std::path::absolute(&path) else {
            return Err(AssetParseError::DoesNotExist { path });
        };

        // 3. Ensure the path exists
        let Ok(path) = dunce::canonicalize(&path) else {
            return Err(AssetParseError::DoesNotExist { path });
        };

        let in_manifest_dir = path != self.manifest_dir && path.starts_with(&self.manifest_dir);

        let in_out_dir = self
            .out_dir
            .as_ref()
            .is_some_and(|dir| path != *dir && path.starts_with(dir));

        // 4. Ensure the path doesn't escape the crate or output directories
        //
        // On windows, we can only compare the prefix if both paths are canonicalized (not just absolute)
        //
        // See: https://github.com/rust-lang/rust/issues/42869
        if !in_manifest_dir && !in_out_dir {
            return Err(AssetParseError::Outside { path });
        }

        Ok(path)
    }
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

#[derive(Debug, PartialEq, Eq)]
enum AssetParseError {
    FileBaseUnavailable,
    DoesNotExist { path: PathBuf },
    Outside { path: PathBuf },
}

impl std::fmt::Display for AssetParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetParseError::FileBaseUnavailable => {
                write!(f, "Relative paths are only supported in Rust 1.88+.")
            }
            AssetParseError::DoesNotExist { path } => {
                write!(f, "Path {} not found.", path.display())
            }
            AssetParseError::Outside { path } => {
                write!(f, "Path {} is outside of allowed folders.", path.display())
            }
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{AssetParseError, PathResolver};
    use tempfile::TempDir;

    struct Ctx {
        crate_root: PathBuf,
        #[expect(unused)]
        out_dir: TempDir,
        out_path: PathBuf,
    }

    impl Ctx {
        fn init() -> Self {
            let out_dir = tempfile::tempdir().unwrap();
            let out_path = dunce::canonicalize(out_dir.path()).unwrap();

            std::fs::write(out_path.join("generated-asset.txt"), b"").unwrap();

            Self {
                crate_root: PathResolver::get_manifest_dir(),
                out_dir,
                out_path,
            }
        }

        fn create_resolver(&self, src: impl Into<PathBuf>) -> PathResolver {
            PathResolver {
                src: src.into(),
                manifest_dir: self.crate_root.clone(),
                out_dir: Some(self.out_path.clone()),
                file_path: Some(self.crate_root.join("src/lib.rs")),
                looks_like_rust_analyzer: false,
            }
        }
    }

    #[test]
    fn resolve_crate_path() {
        let ctx = Ctx::init();

        assert_eq!(
            ctx.create_resolver("assets/asset.txt").resolve(),
            Ok(ctx.crate_root.join("assets/asset.txt")),
        );

        assert_eq!(
            ctx.create_resolver("/assets/asset.txt").resolve(),
            Ok(ctx.crate_root.join("assets/asset.txt")),
        );
    }

    #[test]
    fn resolve_missing_crate_path() {
        let ctx = Ctx::init();

        assert_eq!(
            ctx.create_resolver("assets/does-not-exist.txt").resolve(),
            Err(AssetParseError::DoesNotExist {
                path: ctx.crate_root.join("assets/does-not-exist.txt"),
            }),
        );

        assert_eq!(
            ctx.create_resolver("/assets/does-not-exist.txt").resolve(),
            Err(AssetParseError::DoesNotExist {
                path: ctx.crate_root.join("assets/does-not-exist.txt"),
            }),
        );
    }

    #[test]
    fn resolve_outside_crate_path() {
        let ctx = Ctx::init();

        assert_eq!(
            ctx.create_resolver("/").resolve(),
            Err(AssetParseError::Outside {
                path: ctx.crate_root,
            }),
        );
    }

    #[test]
    fn resolve_file_path() {
        let ctx = Ctx::init();

        assert_eq!(
            ctx.create_resolver("./asset.txt").resolve(),
            Ok(ctx.crate_root.join("src/asset.txt")),
        );

        assert_eq!(
            ctx.create_resolver("../assets/asset.txt").resolve(),
            Ok(ctx.crate_root.join("assets/asset.txt")),
        );
    }

    #[test]
    fn resolve_missing_file_path() {
        let ctx = Ctx::init();

        assert_eq!(
            ctx.create_resolver("./does-not-exist.txt").resolve(),
            Err(AssetParseError::DoesNotExist {
                path: ctx.crate_root.join("src/does-not-exist.txt"),
            }),
        );

        assert_eq!(
            ctx.create_resolver("../assets/does-not-exist.txt")
                .resolve(),
            Err(AssetParseError::DoesNotExist {
                path: ctx.crate_root.join("src/../assets/does-not-exist.txt"),
            }),
        );
    }

    #[test]
    fn resolve_outside_file_path() {
        let ctx = Ctx::init();

        assert_eq!(
            ctx.create_resolver("./..").resolve(),
            Err(AssetParseError::Outside {
                path: ctx.crate_root.clone(),
            }),
        );

        assert_eq!(
            ctx.create_resolver("..").resolve(),
            Err(AssetParseError::Outside {
                path: ctx.crate_root.clone(),
            }),
        );
    }

    #[test]
    fn resolve_out_path() {
        let ctx = Ctx::init();

        let path = ctx.out_path.join("generated-asset.txt");

        assert_eq!(ctx.create_resolver(&path).resolve(), Ok(path));
    }

    #[test]
    fn resolve_missing_out_path() {
        let ctx = Ctx::init();

        let path = ctx.out_path.join("does-not-exist.txt");

        assert_eq!(
            ctx.create_resolver(&path).resolve(),
            Err(AssetParseError::DoesNotExist { path }),
        );
    }

    #[test]
    fn resolve_outside_out_path() {
        let ctx = Ctx::init();

        assert_eq!(
            ctx.create_resolver(&ctx.out_path).resolve(),
            Err(AssetParseError::Outside { path: ctx.out_path }),
        );
    }
}
