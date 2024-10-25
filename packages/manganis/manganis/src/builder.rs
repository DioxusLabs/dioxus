use dioxus_core_types::DioxusFormattable;
use std::path::PathBuf;

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
