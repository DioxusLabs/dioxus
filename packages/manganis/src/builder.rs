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
    /// `blah123.css``
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
        PathBuf::from(self.input.trim_start_matches("/").to_string())
    }

    /// Return a canonicalized path to the asset
    pub fn resolve(&self) -> PathBuf {
        // if we're running with cargo in the loop, we can use the absolute path.
        // this is non-bundled situations
        if let Ok(_manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            return PathBuf::from(self.local);
        }

        // todo: actually properly resolve this
        base_path()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or("/".into()))
            .join(PathBuf::from(self.bundled.trim_start_matches('/')))
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

#[allow(unreachable_code)]
fn base_path() -> Option<PathBuf> {
    // Use the prescence of the bundle to determine if we're in dev mode
    // todo: for other platforms, we should check their bundles too. This currently only works for macOS and iOS
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        // usually the bundle is
        // .app
        //   Contents
        //     Resources
        //       some_asset
        //     macOS
        //       somebinary
        //
        // but not always!
        //
        // we fallback to using the .app's directory itself if it doesn't exist - which is inline
        // with how tauri-bundle works
        //
        // we would normally just want to use core-foundation, but it's much faster for compile times
        // to not pull in CF in a build/proc-macro, so it's a teeny bit hand-rolled
        let cur_exe = std::env::current_exe().ok()?;
        let mut resources_dir = cur_exe.parent()?.parent()?.join("Resources");
        if !resources_dir.exists() {
            resources_dir = cur_exe.parent()?.to_path_buf();
        }

        // Note that this will return `target/debug` if you're in debug mode - not reliable check if we're in dev mode
        return dunce::canonicalize(resources_dir).ok();
    }

    // web-wasm
    #[cfg(target_os = "wasm32-unknown-unknown")]
    {
        return = Some(PathBuf::from("/"))
    }

    None
}
