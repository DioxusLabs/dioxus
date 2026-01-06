use const_serialize_07 as const_serialize;
use const_serialize_08::{ConstStr, SerializeConst};

use crate::{AssetOptions, AssetOptionsBuilder};

/// Options for a Rust binary sidecar asset.
///
/// Use this to compile a separate Rust crate and bundle the resulting binary
/// with your application.
///
/// # Example
///
/// ```rust,ignore
/// use manganis::{asset, Asset};
///
/// // Compile and bundle an LSP server from a workspace crate
/// static LSP_SERVER: Asset = asset!(
///     "/crates/lsp-server",
///     RustBinaryOptions::new()
///         .bin("my-lsp")
///         .release(true)
/// );
///
/// // At runtime, start the LSP server
/// fn start_lsp() {
///     let lsp_path = LSP_SERVER.resolve();
///     std::process::Command::new(lsp_path).spawn();
/// }
/// ```
#[derive(
    Debug,
    Eq,
    PartialEq,
    PartialOrd,
    Clone,
    Copy,
    Hash,
    SerializeConst,
    const_serialize::SerializeConst,
    serde::Serialize,
    serde::Deserialize,
)]
#[const_serialize(crate = const_serialize_08)]
pub struct RustBinaryOptions {
    /// The name of the binary to build (if the crate has multiple binaries)
    bin_name: ConstStr,
    /// Whether to build in release mode
    release: bool,
    /// Comma-separated list of features to enable
    features: ConstStr,
}

impl Default for RustBinaryOptions {
    fn default() -> Self {
        Self::default()
    }
}

impl RustBinaryOptions {
    /// Create a new Rust binary asset builder
    pub const fn new() -> AssetOptionsBuilder<RustBinaryOptions> {
        AssetOptions::rust_binary()
    }

    /// Create default Rust binary options
    pub const fn default() -> Self {
        Self {
            bin_name: ConstStr::new(""),
            release: false,
            features: ConstStr::new(""),
        }
    }

    /// Get the binary name to build
    pub fn bin_name(&self) -> &str {
        self.bin_name.as_str()
    }

    /// Check if the binary should be built in release mode
    pub const fn is_release(&self) -> bool {
        self.release
    }

    /// Get the features to enable (comma-separated)
    pub fn features(&self) -> &str {
        self.features.as_str()
    }
}

impl AssetOptions {
    /// Create a new Rust binary asset builder
    ///
    /// ```rust,ignore
    /// # use manganis::{asset, Asset, AssetOptions};
    /// const _: Asset = asset!("/crates/my-tool", AssetOptions::rust_binary().release(true));
    /// ```
    pub const fn rust_binary() -> AssetOptionsBuilder<RustBinaryOptions> {
        AssetOptionsBuilder::variant(RustBinaryOptions::default())
    }
}

impl AssetOptionsBuilder<RustBinaryOptions> {
    /// Set the binary name to build
    ///
    /// If the crate has multiple binaries, use this to specify which one to build.
    /// If not specified, the default binary will be built.
    ///
    /// ```rust,ignore
    /// # use manganis::{asset, Asset, RustBinaryOptions};
    /// const _: Asset = asset!("/crates/multi-bin", RustBinaryOptions::new().bin("my-tool"));
    /// ```
    pub const fn bin(mut self, name: &'static str) -> Self {
        self.variant.bin_name = ConstStr::new(name);
        self
    }

    /// Set whether to build in release mode
    ///
    /// Release builds are optimized but take longer to compile.
    ///
    /// ```rust,ignore
    /// # use manganis::{asset, Asset, RustBinaryOptions};
    /// const _: Asset = asset!("/crates/tool", RustBinaryOptions::new().release(true));
    /// ```
    pub const fn release(mut self, release: bool) -> Self {
        self.variant.release = release;
        self
    }

    /// Set the features to enable (comma-separated)
    ///
    /// ```rust,ignore
    /// # use manganis::{asset, Asset, RustBinaryOptions};
    /// const _: Asset = asset!("/crates/tool", RustBinaryOptions::new().features("feature1,feature2"));
    /// ```
    pub const fn features(mut self, features: &'static str) -> Self {
        self.variant.features = ConstStr::new(features);
        self
    }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            add_hash: false, // Binaries don't need hash suffixes
            variant: crate::AssetVariant::RustBinary(self.variant),
        }
    }
}
