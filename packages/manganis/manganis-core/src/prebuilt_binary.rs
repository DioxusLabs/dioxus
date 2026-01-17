use const_serialize_07 as const_serialize;
use const_serialize_08::SerializeConst;

use crate::{AssetOptions, AssetOptionsBuilder};

/// Options for a prebuilt binary asset.
///
/// Use this to bundle an existing binary file with your application.
/// The binary will be copied to the bundle and can optionally be marked as executable.
///
/// # Example
///
/// ```rust,ignore
/// use manganis::{asset, Asset};
///
/// // Bundle a prebuilt ffmpeg binary
/// static FFMPEG: Asset = asset!(
///     "/tools/ffmpeg-darwin-arm64",
///     PrebuiltBinaryOptions::new().executable(true)
/// );
///
/// // At runtime, get the path to the bundled binary
/// fn transcode() {
///     let ffmpeg_path = FFMPEG.resolve();
///     std::process::Command::new(ffmpeg_path).arg("-version").spawn();
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
pub struct PrebuiltBinaryOptions {
    /// Whether to set the executable permission on the binary
    executable: bool,
}

impl Default for PrebuiltBinaryOptions {
    fn default() -> Self {
        Self::default()
    }
}

impl PrebuiltBinaryOptions {
    /// Create a new prebuilt binary asset builder
    pub const fn new() -> AssetOptionsBuilder<PrebuiltBinaryOptions> {
        AssetOptions::prebuilt_binary()
    }

    /// Create default prebuilt binary options
    pub const fn default() -> Self {
        Self { executable: false }
    }

    /// Check if the binary should be marked as executable
    pub const fn is_executable(&self) -> bool {
        self.executable
    }
}

impl AssetOptions {
    /// Create a new prebuilt binary asset builder
    ///
    /// ```rust,ignore
    /// # use manganis::{asset, Asset, AssetOptions};
    /// const _: Asset = asset!("/tools/ffmpeg", AssetOptions::prebuilt_binary().executable(true));
    /// ```
    pub const fn prebuilt_binary() -> AssetOptionsBuilder<PrebuiltBinaryOptions> {
        AssetOptionsBuilder::variant(PrebuiltBinaryOptions::default())
    }
}

impl AssetOptionsBuilder<PrebuiltBinaryOptions> {
    /// Set whether the binary should be marked as executable
    ///
    /// On Unix systems, this will set the executable permission bit.
    /// On Windows, this has no effect.
    ///
    /// ```rust,ignore
    /// # use manganis::{asset, Asset, PrebuiltBinaryOptions};
    /// const _: Asset = asset!("/tools/ffmpeg", PrebuiltBinaryOptions::new().executable(true));
    /// ```
    pub const fn executable(mut self, executable: bool) -> Self {
        self.variant.executable = executable;
        self
    }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            add_hash: false, // Binaries typically don't need hash suffixes
            variant: crate::AssetVariant::PrebuiltBinary(self.variant),
        }
    }
}
