use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;

#[derive(
    Copy,
    Clone,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Debug,
    Default,
    clap::ValueEnum,
)]
#[non_exhaustive]
pub(crate) enum Platform {
    /// Targeting the web platform using WASM
    #[clap(name = "web")]
    #[serde(rename = "web")]
    #[default]
    Web,

    /// Targeting the desktop platform using Tao/Wry-based webview
    ///
    /// Will only build for your native architecture - to do cross builds you need to use a VM.
    /// Read more about cross-builds on the Dioxus Website.
    #[clap(name = "desktop")]
    #[serde(rename = "desktop")]
    Desktop,

    /// Targeting the ios platform
    ///
    /// Can't work properly if you're not building from an Apple device.
    #[clap(name = "ios")]
    #[serde(rename = "ios")]
    Ios,

    /// Targeting the android platform
    #[clap(name = "android")]
    #[serde(rename = "android")]
    Android,

    /// Targetting the server platform using Axum and Dioxus-Fullstack
    ///
    /// This is implicitly passed if `fullstack` is enabled as a feature. Using this variant simply
    /// means you're only building the server variant without the `.wasm` to serve.
    #[clap(name = "server")]
    #[serde(rename = "server")]
    Server,

    /// Targeting the static generation platform using SSR and Dioxus-Fullstack
    #[clap(name = "liveview")]
    #[serde(rename = "liveview")]
    Liveview,
}

/// An error that occurs when a platform is not recognized
pub(crate) struct UnknownPlatformError;

impl std::fmt::Display for UnknownPlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown platform")
    }
}

impl FromStr for Platform {
    type Err = UnknownPlatformError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "web" => Ok(Self::Web),
            "desktop" => Ok(Self::Desktop),
            "liveview" => Ok(Self::Liveview),
            "server" => Ok(Self::Server),
            "ios" => Ok(Self::Ios),
            "android" => Ok(Self::Android),
            _ => Err(UnknownPlatformError),
        }
    }
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let feature = self.feature_name();
        f.write_str(feature)
    }
}

impl Platform {
    /// Get the feature name for the platform in the dioxus crate
    pub(crate) fn feature_name(&self) -> &str {
        match self {
            Platform::Web => "web",
            Platform::Desktop => "desktop",
            Platform::Liveview => "liveview",
            Platform::Ios => "ios",
            Platform::Android => "android",
            Platform::Server => "server",
        }
    }
}
