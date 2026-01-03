use crate::AssetOptions;
use const_serialize_07 as const_serialize;
use const_serialize_08::{deserialize_const, ConstStr, SerializeConst};
use std::{fmt::Debug, hash::Hash, path::PathBuf};

/// An asset that should be copied by the bundler with some options. This type will be
/// serialized into the binary.
/// CLIs that support manganis, should pull out the assets from the link section, optimize,
/// and write them to the filesystem at [`BundledAsset::bundled_path`] for the application
/// to use.
#[derive(
    Debug,
    Eq,
    Clone,
    Copy,
    SerializeConst,
    const_serialize::SerializeConst,
    serde::Serialize,
    serde::Deserialize,
)]
#[const_serialize(crate = const_serialize_08)]
pub struct BundledAsset {
    /// The absolute path of the asset
    absolute_source_path: ConstStr,
    /// The bundled path of the asset
    bundled_path: ConstStr,
    /// The options for the asset
    options: AssetOptions,
}

impl PartialEq for BundledAsset {
    fn eq(&self, other: &Self) -> bool {
        self.absolute_source_path == other.absolute_source_path
            && self.bundled_path == other.bundled_path
            && self.options == other.options
    }
}

impl PartialOrd for BundledAsset {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self
            .absolute_source_path
            .partial_cmp(&other.absolute_source_path)
        {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.bundled_path.partial_cmp(&other.bundled_path) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.options.partial_cmp(&other.options)
    }
}

impl Hash for BundledAsset {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.absolute_source_path.hash(state);
        self.bundled_path.hash(state);
        self.options.hash(state);
    }
}

impl BundledAsset {
    pub const PLACEHOLDER_HASH: &str = "This should be replaced by dx as part of the build process. If you see this error, make sure you are using a matching version of dx and dioxus and you are not stripping symbols from your binary.";

    #[doc(hidden)]
    /// This should only be called from the macro
    /// Create a new asset
    pub const fn new(
        absolute_source_path: &str,
        bundled_path: &str,
        options: AssetOptions,
    ) -> Self {
        Self {
            absolute_source_path: ConstStr::new(absolute_source_path),
            bundled_path: ConstStr::new(bundled_path),
            options,
        }
    }

    /// Get the bundled name of the asset. This identifier cannot be used to read the asset directly
    pub fn bundled_path(&self) -> &str {
        self.bundled_path.as_str()
    }

    /// Get the absolute path of the asset source. This path will not be available when the asset is bundled
    pub fn absolute_source_path(&self) -> &str {
        self.absolute_source_path.as_str()
    }

    /// Get the options for the asset
    pub const fn options(&self) -> &AssetOptions {
        &self.options
    }
}

/// A bundled asset with some options. The asset can be used in rsx! to reference the asset.
/// It should not be read directly with [`std::fs::read`] because the path needs to be resolved
/// relative to the bundle
///
/// ```rust, ignore
/// # use manganis::{asset, Asset};
/// # use dioxus::prelude::*;
/// const ASSET: Asset = asset!("/assets/image.png");
/// rsx! {
///     img { src: ASSET }
/// };
/// ```
#[allow(unpredictable_function_pointer_comparisons)]
#[derive(PartialEq, Clone, Copy)]
pub struct Asset {
    /// A function that returns a pointer to the bundled asset. This will be resolved after the linker has run and
    /// put into the lazy asset. We use a function instead of using the pointer directly to force the compiler to
    /// read the static __LINK_SECTION at runtime which will be offset by the hot reloading engine instead
    /// of at compile time which can't be offset
    ///
    /// WARNING: Don't read this directly. Reads can get optimized away at compile time before
    /// the data for this is filled in by the CLI after the binary is built. Instead, use
    /// [`std::ptr::read_volatile`] to read the data.
    bundled: fn() -> &'static [u8],
    /// The legacy version of [`Self::bundled`]. This is only used for backwards compatibility with older versions of the CLI
    legacy: fn() -> &'static [u8],
}

impl Debug for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.resolve().fmt(f)
    }
}

unsafe impl Send for Asset {}
unsafe impl Sync for Asset {}

impl Asset {
    #[doc(hidden)]
    /// This should only be called from the macro
    /// Create a new asset from the bundled form of the asset and the link section
    pub const fn new(
        bundled: extern "Rust" fn() -> &'static [u8],
        legacy: extern "Rust" fn() -> &'static [u8],
    ) -> Self {
        Self { bundled, legacy }
    }

    /// Get the bundled asset
    pub fn bundled(&self) -> BundledAsset {
        // Read the slice using volatile reads to prevent the compiler from optimizing
        // away the read at compile time
        fn read_slice_volatile(bundled: &'static [u8]) -> const_serialize_07::ConstVec<u8> {
            let ptr = bundled as *const [u8] as *const u8;
            let len = bundled.len();
            if ptr.is_null() {
                panic!("Tried to use an asset that was not bundled. Make sure you are compiling dx as the linker");
            }
            let mut bytes = const_serialize_07::ConstVec::new();
            for byte in 0..len {
                // SAFETY: We checked that the pointer was not null above. The pointer is valid for reads and
                // since we are reading a u8 there are no alignment requirements
                let byte = unsafe { std::ptr::read_volatile(ptr.add(byte)) };
                bytes = bytes.push(byte);
            }
            bytes
        }

        let bundled = (self.bundled)();
        let bytes = read_slice_volatile(bundled);
        let read = bytes.as_ref();
        let asset = deserialize_const!(BundledAsset, read).expect("Failed to deserialize asset. Make sure you built with the matching version of the Dioxus CLI").1;

        // If the asset wasn't bundled with the newer format, try the legacy format
        if asset.bundled_path() == BundledAsset::PLACEHOLDER_HASH {
            let bundled = (self.legacy)();
            let bytes = read_slice_volatile(bundled);
            let read = bytes.read();
            let asset = const_serialize_07::deserialize_const!(BundledAsset, read).expect("Failed to deserialize asset. Make sure you built with the matching version of the Dioxus CLI").1;
            asset
        } else {
            asset
        }
    }

    /// Return a canonicalized path to the asset
    ///
    /// Attempts to resolve it against an `assets` folder in the current directory.
    /// If that doesn't exist, it will resolve against the cargo manifest dir
    pub fn resolve(&self) -> PathBuf {
        #[cfg(feature = "dioxus")]
        // If the asset is relative, we resolve the asset at the current directory
        if !dioxus_core_types::is_bundled_app() {
            return PathBuf::from(self.bundled().absolute_source_path.as_str());
        }

        #[cfg(feature = "dioxus")]
        let bundle_root = {
            let base_path = dioxus_cli_config::base_path();
            let base_path = base_path
                .as_deref()
                .map(|base_path| {
                    let trimmed = base_path.trim_matches('/');
                    format!("/{trimmed}")
                })
                .unwrap_or_default();
            PathBuf::from(format!("{base_path}/assets/"))
        };
        #[cfg(not(feature = "dioxus"))]
        let bundle_root = PathBuf::from("/assets/");

        // Otherwise presumably we're bundled and we can use the bundled path
        bundle_root.join(PathBuf::from(
            self.bundled().bundled_path.as_str().trim_start_matches('/'),
        ))
    }
}

impl From<Asset> for String {
    fn from(value: Asset) -> Self {
        value.to_string()
    }
}
impl From<Asset> for Option<String> {
    fn from(value: Asset) -> Self {
        Some(value.to_string())
    }
}

impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.resolve().display())
    }
}

#[cfg(feature = "dioxus")]
impl dioxus_core_types::DioxusFormattable for Asset {
    fn format(&self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Owned(self.to_string())
    }
}
