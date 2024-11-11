use const_serialize::{ConstStr, SerializeConst};
use dioxus_core_types::DioxusFormattable;
use std::path::PathBuf;

/// A builder for a generic asset. For configuration options specific to the asset type, see [`image`], [`folder`], [`css`], and [`js`]
#[derive(SerializeConst)]
pub struct AssetBuilder {
    local_path: ConstStr,
    preload: bool,
}

impl AssetBuilder {
    /// Create a new asset builder
    pub const fn new(local_path: &str) -> Self {
        Self {
            local_path: ConstStr::new(local_path),
            preload: false,
        }
    }

    /// Make the asset preloaded
    ///
    /// Preloading an image will make the image start to load as soon as possible. This is useful for images that will be displayed soon after the page loads or images that may not be visible immediately, but should start loading sooner
    ///
    /// ```rust
    /// const _: manganis::ImageAsset = manganis::mg!(image("https://avatars.githubusercontent.com/u/79236386?s=48&v=4").preload());
    /// ```
    #[allow(unused)]
    pub const fn preload(self) -> Self {
        Self {
            preload: true,
            ..self
        }
    }

    /// Finalize the asset builder and return the asset
    pub const fn build(self) -> Asset {
        Asset {
            input: "",
            local: "",
            bundled: "",
        }
    }
}

/// Asset
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Hash)]
pub struct Asset {
    /// The input URI given to the macro
    pub input: &'static str,

    /// The absolute path to the asset on the filesystem
    pub local: &'static str,

    /// The asset location after its been bundled
    ///
    /// `blah-123.css``
    pub bundled: &'static str,
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
