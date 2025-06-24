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
pub(crate) enum PlatformArg {
    /// Targeting the web platform using WASM
    #[clap(name = "web")]
    #[default]
    Web,

    /// Targeting macos desktop
    #[clap(name = "macos")]
    MacOS,

    /// Targeting windows desktop
    #[clap(name = "windows")]
    Windows,

    /// Targeting linux desktop
    #[clap(name = "linux")]
    Linux,

    /// Targeting the ios platform
    ///
    /// Can't work properly if you're not building from an Apple device.
    #[clap(name = "ios")]
    Ios,

    /// Targeting the android platform
    #[clap(name = "android")]
    Android,

    /// Targeting the current platform with the "desktop" renderer
    #[clap(name = "desktop")]
    Desktop,

    /// Targeting the current platform with the "native" renderer
    #[clap(name = "native")]
    Native,

    /// Targeting the server platform using Axum and Dioxus-Fullstack
    ///
    /// This is implicitly passed if `fullstack` is enabled as a feature. Using this variant simply
    /// means you're only building the server variant without the `.wasm` to serve.
    #[clap(name = "server")]
    Server,

    /// Targeting the static generation platform using SSR and Dioxus-Fullstack
    #[clap(name = "liveview")]
    Liveview,
}

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
    clap::ValueEnum,
)]
#[non_exhaustive]
pub(crate) enum ClientRenderer {
    /// Targeting webview renderer
    #[serde(rename = "webview")]
    Webview,

    /// Targeting native renderer
    #[serde(rename = "native")]
    Native,
}

#[derive(
    Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug, Default,
)]
#[non_exhaustive]
pub(crate) enum Platform {
    /// Targeting the web platform using WASM
    #[serde(rename = "web")]
    #[default]
    Web,

    /// Targeting macos desktop
    #[serde(rename = "macos")]
    MacOS,

    /// Targeting windows desktop
    #[serde(rename = "windows")]
    Windows,

    /// Targeting linux desktop
    #[serde(rename = "linux")]
    Linux,

    /// Targeting the ios platform
    ///
    /// Can't work properly if you're not building from an Apple device.
    #[serde(rename = "ios")]
    Ios,

    /// Targeting the android platform
    #[serde(rename = "android")]
    Android,

    /// Targeting the server platform using Axum and Dioxus-Fullstack
    ///
    /// This is implicitly passed if `fullstack` is enabled as a feature. Using this variant simply
    /// means you're only building the server variant without the `.wasm` to serve.
    #[serde(rename = "server")]
    Server,

    /// Targeting the static generation platform using SSR and Dioxus-Fullstack
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
            "macos" => Ok(Self::MacOS),
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
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
        f.write_str(match self {
            Platform::Web => "web",
            Platform::MacOS => "macos",
            Platform::Windows => "windows",
            Platform::Linux => "linux",
            Platform::Ios => "ios",
            Platform::Android => "android",
            Platform::Server => "server",
            Platform::Liveview => "liveview",
        })
    }
}

impl From<PlatformArg> for Platform {
    fn from(value: PlatformArg) -> Self {
        match value {
            // Most values map 1:1
            PlatformArg::Web => Platform::Web,
            PlatformArg::MacOS => Platform::MacOS,
            PlatformArg::Windows => Platform::Windows,
            PlatformArg::Linux => Platform::Linux,
            PlatformArg::Ios => Platform::Ios,
            PlatformArg::Android => Platform::Android,
            PlatformArg::Server => Platform::Server,
            PlatformArg::Liveview => Platform::Liveview,

            // The alias arguments
            PlatformArg::Desktop | PlatformArg::Native => {
                Platform::TARGET_PLATFORM.unwrap()
            }
        }
    }
}

impl Platform {
    #[cfg(target_os = "macos")]
    pub(crate) const TARGET_PLATFORM: Option<Self> = Some(Platform::MacOS);
    #[cfg(target_os = "windows")]
    pub(crate) const TARGET_PLATFORM: Option<Self> = Some(Platform::Windows);
    #[cfg(target_os = "linux")]
    pub(crate) const TARGET_PLATFORM: Option<Self> = Some(Platform::Linux);
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    pub(crate) const TARGET_PLATFORM: Option<Self> = None;

    // /// Get the feature name for the platform in the dioxus crate
    pub(crate) fn feature_name(&self, renderer: Option<ClientRenderer>) -> &str {
        match self {
            Platform::Web => "web",
            Platform::MacOS | Platform::Windows | Platform::Linux => match renderer {
                None | Some(ClientRenderer::Webview) => "desktop",
                Some(ClientRenderer::Native) => "native",
            },
            Platform::Ios | Platform::Android => match renderer {
                None | Some(ClientRenderer::Webview) => "mobile",
                Some(ClientRenderer::Native) => "native",
            },
            Platform::Server => "server",
            Platform::Liveview => "liveview",
        }
    }

    /// Get the name of the folder we need to generate for this platform
    ///
    /// Note that web and server share the same platform folder since we'll export the web folder as a bundle on its own
    pub(crate) fn build_folder_name(&self) -> &'static str {
        match self {
            Platform::Web => "web",
            Platform::Server => "web",
            Platform::Liveview => "liveview",
            Platform::Ios => "ios",
            Platform::Android => "android",
            Platform::Windows => "windows",
            Platform::Linux => "linux",
            Platform::MacOS => "macos",
        }
    }

    pub(crate) fn expected_name(&self) -> &'static str {
        match self {
            Platform::Web => "Web",
            Platform::MacOS => "Desktop MacOS",
            Platform::Windows => "Desktop Windows",
            Platform::Linux => "Desktop Linux",
            Platform::Ios => "Mobile iOS",
            Platform::Android => "Mobile Android",
            Platform::Server => "Server",
            Platform::Liveview => "Liveview",
        }
    }

    pub(crate) fn autodetect_from_cargo_feature(feature: &str) -> Option<Self> {
        match feature {
            "web" => Some(Platform::Web),
            "desktop" | "native" => Platform::TARGET_PLATFORM,
            "mobile" => None,
            "liveview" => Some(Platform::Liveview),
            "server" => Some(Platform::Server),
            _ => None,
        }
    }

    pub(crate) fn profile_name(&self, release: bool) -> String {
        let base_profile = match self {
            // TODO: add native profile?
            Platform::MacOS | Platform::Windows | Platform::Linux => "desktop",
            Platform::Web => "web",
            Platform::Ios => "ios",
            Platform::Android => "android",
            Platform::Server => "server",
            Platform::Liveview => "liveview",
        };

        if release {
            format!("{}-release", base_profile)
        } else {
            format!("{}-dev", base_profile)
        }
    }
}
