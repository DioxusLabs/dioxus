use crate::AssetOptions;
use const_serialize::{ConstStr, SerializeConst, deserialize_const};
use std::{
    collections::HashMap,
    fmt::Debug,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    sync::{LazyLock, RwLock},
};

static EMBEDDED_ASSETS: LazyLock<RwLock<HashMap<String, &'static [u8]>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Get the bytes of an embedded asset by its resolved path.
pub fn embedded_asset(path: &str) -> Option<&'static [u8]> {
    EMBEDDED_ASSETS.read().unwrap().get(path).copied()
}

/// An asset that should be copied by the bundler with some options. This type will be
/// serialized into the binary.
/// CLIs that support manganis, should pull out the assets from the link section, optimize,
/// and write them to the filesystem at [`BundledAsset::bundled_path`] for the application
/// to use.
#[derive(Debug, Eq, Clone, Copy, SerializeConst, serde::Serialize, serde::Deserialize)]
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
    embedded: Option<&'static [u8]>,
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
    pub const fn new(bundled: extern "Rust" fn() -> &'static [u8]) -> Self {
        Self::new_with_embedded(bundled, None)
    }

    #[doc(hidden)]
    /// Create an asset with optional embedded bytes.
    pub const fn new_with_embedded(
        bundled: extern "Rust" fn() -> &'static [u8],
        embedded: Option<&'static [u8]>,
    ) -> Self {
        Self { bundled, embedded }
    }

    /// Get the bundled asset
    pub fn bundled(&self) -> BundledAsset {
        // Read the slice using volatile reads to prevent the compiler from optimizing
        // away the read at compile time
        let bundled = (self.bundled)();
        let ptr = bundled as *const [u8] as *const u8;
        let len = bundled.len();
        if ptr.is_null() {
            panic!(
                "Tried to use an asset that was not bundled. Make sure you are compiling dx as the linker"
            );
        }
        let mut bytes = Vec::with_capacity(len);
        for byte in 0..len {
            // SAFETY: We checked that the pointer was not null above. The pointer is valid for reads and
            // since we are reading a u8 there are no alignment requirements
            let byte = unsafe { std::ptr::read_volatile(ptr.add(byte)) };
            bytes.push(byte);
        }

        deserialize_const!(BundledAsset, bytes.as_slice()).expect("Failed to deserialize asset. Make sure you built with the matching version of the Dioxus CLI").1
    }

    /// Return a canonicalized path to the asset
    ///
    /// Attempts to resolve it against an `assets` folder in the current directory.
    /// If that doesn't exist, it will resolve against the cargo manifest dir
    pub fn resolve(&self) -> PathBuf {
        let bundled = self.bundled();
        if bundled.bundled_path() == BundledAsset::PLACEHOLDER_HASH {
            if let Some(bytes) = self.embedded {
                let mut hasher = DefaultHasher::new();
                bundled.hash(&mut hasher);
                let name = Path::new(bundled.absolute_source_path())
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                let path = bundle_root().join(format!("{:016x}-{name}", hasher.finish()));
                let path = path.to_string_lossy().into_owned();
                EMBEDDED_ASSETS.write().unwrap().insert(path.clone(), bytes);
                return PathBuf::from(path);
            }
            return PathBuf::from(bundled.absolute_source_path());
        }

        #[cfg(feature = "dioxus")]
        if !dioxus_base_types::is_bundled_app() {
            return PathBuf::from(bundled.absolute_source_path());
        }

        // Otherwise presumably we're bundled and we can use the bundled path
        bundle_root().join(PathBuf::from(
            bundled.bundled_path().trim_start_matches('/'),
        ))
    }
}

fn bundle_root() -> PathBuf {
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

    bundle_root
}

#[cfg(test)]
mod tests {
    use super::*;
    use const_serialize::{ConstVec, serialize_const};

    const OPTIONS: AssetOptions = AssetOptions::builder().into_asset_options();
    static UNPROCESSED: ConstVec<u8> = serialize_const(
        &BundledAsset::new("/source/style.css", BundledAsset::PLACEHOLDER_HASH, OPTIONS),
        ConstVec::new(),
    );
    static PROCESSED: ConstVec<u8> = serialize_const(
        &BundledAsset::new("/source/style.css", "/style-hash.css", OPTIONS),
        ConstVec::new(),
    );

    #[test]
    fn resolve_unprocessed_asset() {
        let asset = Asset::new(|| UNPROCESSED.as_slice());
        assert_eq!(asset.resolve(), PathBuf::from("/source/style.css"));
    }

    #[test]
    fn resolve_embedded_asset() {
        let asset = Asset::new_with_embedded(|| UNPROCESSED.as_slice(), Some(b"embedded"));
        let path = asset.resolve();
        assert!(path.to_string_lossy().ends_with("-style.css"));
        assert_eq!(
            embedded_asset(path.to_str().unwrap()),
            Some(&b"embedded"[..])
        );
        assert_eq!(embedded_asset("/missing"), None);
    }

    #[test]
    fn resolve_processed_asset() {
        let asset = Asset::new(|| PROCESSED.as_slice());
        #[cfg(feature = "dioxus")]
        if !dioxus_base_types::is_bundled_app() {
            assert_eq!(asset.resolve(), PathBuf::from("/source/style.css"));
            return;
        }
        #[cfg(feature = "dioxus")]
        let root = dioxus_cli_config::base_path()
            .map(|path| format!("/{}/assets", path.trim_matches('/')))
            .unwrap_or_else(|| "/assets".to_string());
        #[cfg(not(feature = "dioxus"))]
        let root = "/assets";
        assert_eq!(asset.resolve(), PathBuf::from(root).join("style-hash.css"));
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
impl dioxus_base_types::DioxusFormattable for Asset {
    fn format(&self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Owned(self.to_string())
    }
}
