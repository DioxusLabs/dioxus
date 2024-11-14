#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use std::path::PathBuf;

pub use const_serialize;

use dioxus_core_types::DioxusFormattable;

/// Asset
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Hash)]
pub struct Asset {
    /// The input URI given to the macro
    pub input: &'static str,
}

impl Asset {
    /// Create a new asset
    pub const fn new(self) -> Self {
        self
    }

    /// Get the path to the asset
    pub fn path(&self) -> PathBuf {
        PathBuf::from(self.input.to_string())
    }

    /// Get the path to the asset
    pub fn relative_path(&self) -> PathBuf {
        PathBuf::from(self.input.trim_start_matches('/').to_string())
    }

    /// Return a canonicalized path to the asset
    ///
    /// Attempts to resolve it against an `assets` folder in the current directory.
    /// If that doesn't exist, it will resolve against the cargo manifest dir
    pub fn resolve(&self) -> PathBuf {
        // If the asset is relative, we resolve the asset at the current directory
        if !dioxus_core_types::is_bundled_app() {
            return PathBuf::from(self.local);
        }

        // Otherwise presumably we're bundled and we can use the bundled path
        PathBuf::from("/assets/").join(PathBuf::from(self.bundled.trim_start_matches('/')))
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

impl DioxusFormattable for Asset {
    fn format(&self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Owned(self.to_string())
    }
}

/// The mg macro collects assets that will be included in the final binary
///
/// # Files
///
/// The file builder collects an arbitrary file. Relative paths are resolved relative to the package root
/// ```rust
/// # use manganis::asset;
/// const _: &str = asset!("src/asset.txt");
/// ```
/// Or you can use URLs to read the asset at build time from a remote location
/// ```rust
/// # use manganis::asset;
/// const _: &str = asset!("https://rustacean.net/assets/rustacean-flat-happy.png");
/// ```
///
/// # Images
///
/// You can collect images which will be automatically optimized with the image builder:
/// ```rust
/// # use manganis::asset;
/// const _: manganis::ImageAsset = asset!(image("rustacean-flat-gesture.png"));
/// ```
/// Resize the image at compile time to make the assets file size smaller:
/// ```rust
/// # use manganis::asset;
/// const _: manganis::ImageAsset = asset!(image("rustacean-flat-gesture.png").size(52, 52));
/// ```
/// Or convert the image at compile time to a web friendly format:
/// ```rust
/// # use manganis::asset;
/// const _: manganis::ImageAsset = asset!(image("rustacean-flat-gesture.png").format(ImageFormat::Avif).size(52, 52));
/// ```
/// You can mark images as preloaded to make them load faster in your app
/// ```rust
/// # use manganis::asset;
/// const _: manganis::ImageAsset = asset!(image("rustacean-flat-gesture.png").preload());
/// ```
#[macro_export]
macro_rules! asset {
    ($asset:literal $($tokens:tt)*) => {{
        const ASSET: $crate::AssetBuilder = $crate::AssetBuilder::new($asset) $($tokens)*;
        const BUFFER: $crate::const_serialize::ConstWriteBuffer = {
            let write = $crate::const_serialize::ConstWriteBuffer::new();
            $crate::const_serialize::serialize_const(&ASSET, write)
        };
        const BYTES: &[u8] = BUFFER.as_ref();
        const LEN: usize = BYTES.len();

        #[link_section = $crate::__current_link_section!()]
        #[used]
        static LINK_SECTION: [u8; LEN] = {
            let mut bytes = [0; LEN];
            let mut i = 0;
            while i < LEN {
                bytes[i] = BYTES[i];
                i += 1;
            }
            bytes
        };

        ASSET.build()
    }};
}
