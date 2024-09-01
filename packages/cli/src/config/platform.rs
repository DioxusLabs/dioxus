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
pub enum Platform {
    /// Targeting the web platform using WASM
    #[clap(name = "web")]
    #[serde(rename = "web")]
    #[default]
    Web,

    /// Targeting the desktop platform using Tao/Wry-based webview
    #[clap(name = "desktop")]
    #[serde(rename = "desktop")]
    Desktop,

    #[clap(name = "mobile")]
    #[serde(rename = "mobile")]
    Mobile,

    /// Targeting the server platform using Axum and Dioxus-Fullstack
    #[clap(name = "fullstack")]
    #[serde(rename = "fullstack")]
    Fullstack,

    /// Targeting the static generation platform using SSR and Dioxus-Fullstack
    #[clap(name = "liveview")]
    #[serde(rename = "liveview")]
    Liveview,
}

/// An error that occurs when a platform is not recognized
pub struct UnknownPlatformError;

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
            "fullstack" => Ok(Self::Fullstack),
            "liveview" => Ok(Self::Liveview),
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
    /// All platforms the dioxus CLI supports
    pub const ALL: &'static [Self] = &[Platform::Web, Platform::Desktop, Platform::Fullstack];

    /// Get the feature name for the platform in the dioxus crate
    pub fn feature_name(&self) -> &str {
        match self {
            Platform::Web => "web",
            Platform::Desktop => "desktop",
            Platform::Fullstack => "fullstack",
            Platform::Liveview => "liveview",
            Platform::Mobile => "mobile",
        }
    }
}
