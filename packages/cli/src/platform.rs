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

    /// Targeting macos desktop
    /// When running on macos, you can also use `--platform desktop` to build for the desktop
    #[cfg_attr(target_os = "macos", clap(alias = "desktop"))]
    #[clap(name = "macos")]
    #[serde(rename = "macos")]
    MacOS,

    /// Targeting windows desktop
    /// When running on windows, you can also use `--platform desktop` to build for the desktop
    #[cfg_attr(target_os = "windows", clap(alias = "desktop"))]
    #[clap(name = "windows")]
    #[serde(rename = "windows")]
    Windows,

    /// Targeting linux desktop
    /// When running on linux, you can also use `--platform desktop` to build for the desktop
    #[cfg_attr(target_os = "linux", clap(alias = "desktop"))]
    #[clap(name = "linux")]
    #[serde(rename = "linux")]
    Linux,

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

    /// Targeting the server platform using Axum and Dioxus-Fullstack
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

impl Platform {
    /// Get the feature name for the platform in the dioxus crate
    pub(crate) fn feature_name(&self) -> &str {
        match self {
            Platform::Web => "web",
            Platform::MacOS => "desktop",
            Platform::Windows => "desktop",
            Platform::Linux => "desktop",
            Platform::Server => "server",
            Platform::Liveview => "liveview",
            Platform::Ios => "mobile",
            Platform::Android => "mobile",
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
            "desktop" => {
                #[cfg(target_os = "macos")]
                {
                    Some(Platform::MacOS)
                }
                #[cfg(target_os = "windows")]
                {
                    Some(Platform::Windows)
                }
                #[cfg(target_os = "linux")]
                {
                    Some(Platform::Linux)
                }
                // Possibly need a something for freebsd? Maybe default to Linux?
                #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
                {
                    None
                }
            }
            "mobile" => None,
            "liveview" => Some(Platform::Liveview),
            "server" => Some(Platform::Server),
            _ => None,
        }
    }
}
