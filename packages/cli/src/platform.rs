use anyhow::{Context, Result};
use clap::{arg, ArgMatches, Args, FromArgMatches};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;
use target_lexicon::{Architecture, Environment, OperatingSystem, Triple};

use crate::Workspace;

#[derive(
    Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug, Default,
)]
#[non_exhaustive]
pub(crate) enum PlatformArg {
    /// Targeting the WASM architecture
    Wasm,

    /// Targeting macos desktop
    MacOS,

    /// Targeting windows desktop
    Windows,

    /// Targeting linux desktop
    Linux,

    /// Targeting the ios platform
    ///
    /// Can't work properly if you're not building from an Apple device.
    Ios,

    /// Targeting the android platform
    Android,

    /// Targeting the current platform with the "desktop" renderer
    Desktop,

    /// An unknown target platform
    #[default]
    Unknown,
}

impl Args for PlatformArg {
    fn augment_args(cmd: clap::Command) -> clap::Command {
        const HELP_HEADING: &str = "Platform";
        cmd.arg(arg!(--wasm "The wasm target platform").help_heading(HELP_HEADING))
            .arg(arg!(--macos "The macos target platform").help_heading(HELP_HEADING))
            .arg(arg!(--windows "The windows target platform").help_heading(HELP_HEADING))
            .arg(arg!(--linux "The linux target platform").help_heading(HELP_HEADING))
            .arg(arg!(--ios "The ios target platform").help_heading(HELP_HEADING))
            .arg(arg!(--android "The android target platform").help_heading(HELP_HEADING))
            .arg(arg!(--desktop "The desktop target platform").help_heading(HELP_HEADING))
            .group(
                clap::ArgGroup::new("platform")
                    .args([
                        "wasm", "macos", "windows", "linux", "ios", "android", "desktop",
                    ])
                    .multiple(false)
                    .required(false),
            )
    }

    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        Self::augment_args(cmd)
    }
}

impl FromArgMatches for PlatformArg {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        if let Some(platform) = matches.get_one::<clap::Id>("platform") {
            match platform.as_str() {
                "wasm" => Ok(Self::Wasm),
                "macos" => Ok(Self::MacOS),
                "windows" => Ok(Self::Windows),
                "linux" => Ok(Self::Linux),
                "ios" => Ok(Self::Ios),
                "android" => Ok(Self::Android),
                "desktop" => Ok(Self::Desktop),
                _ => Err(clap::Error::raw(
                    clap::error::ErrorKind::InvalidValue,
                    format!("Unknown platform: {}", platform),
                )),
            }
        } else {
            Ok(Self::Unknown)
        }
    }
    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        *self = Self::from_arg_matches(matches)?;
        Ok(())
    }
}

impl PlatformArg {
    #[cfg(target_os = "macos")]
    pub(crate) const TARGET_PLATFORM: Option<Self> = Some(Self::MacOS);
    #[cfg(target_os = "windows")]
    pub(crate) const TARGET_PLATFORM: Option<Self> = Some(Self::Windows);
    #[cfg(target_os = "linux")]
    pub(crate) const TARGET_PLATFORM: Option<Self> = Some(Self::Linux);
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    pub(crate) const TARGET_PLATFORM: Option<Self> = None;

    pub(crate) async fn into_target(self, device: bool, workspace: &Workspace) -> Result<Triple> {
        match self {
            // Generally just use the host's triple for native executables unless specified otherwise
            Self::MacOS | Self::Windows | Self::Linux | Self::Desktop | Self::Unknown => {
                Ok(Triple::host())
            }

            // We currently assume unknown-unknown for web, but we might want to eventually
            // support emscripten
            Self::Wasm => Ok("wasm32-unknown-unknown".parse()?),

            // For iOS we should prefer the actual architecture for the simulator, but in lieu of actually
            // figuring that out, we'll assume aarch64 on m-series and x86_64 otherwise
            Self::Ios => {
                // use the host's architecture and sim if --device is passed
                use target_lexicon::{Architecture, HOST};
                let triple_str = match HOST.architecture {
                    Architecture::Aarch64(_) if device => "aarch64-apple-ios",
                    Architecture::Aarch64(_) => "aarch64-apple-ios-sim",
                    _ if device => "x86_64-apple-ios",
                    _ => "x86_64-apple-ios",
                };
                Ok(triple_str.parse()?)
            }

            // Same idea with android but we figure out the connected device using adb
            Self::Android => Ok(workspace
                .android_tools()?
                .autodetect_android_device_triple()
                .await),
        }
    }
}

#[derive(
    Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug, Default,
)]
pub(crate) struct RendererArg {
    pub(crate) renderer: Option<Renderer>,
}

impl Args for RendererArg {
    fn augment_args(cmd: clap::Command) -> clap::Command {
        const HELP_HEADING: &str = "Renderer";
        cmd.arg(arg!(--web "Targeting the web renderer").help_heading(HELP_HEADING))
            .arg(arg!(--webview "Targeting the webview renderer").help_heading(HELP_HEADING))
            .arg(arg!(--native "Targeting the native renderer").help_heading(HELP_HEADING))
            .arg(arg!(--server "Targeting the server renderer").help_heading(HELP_HEADING))
            .arg(arg!(--liveview "Targeting the liveview renderer").help_heading(HELP_HEADING))
            .group(
                clap::ArgGroup::new("renderer")
                    .args(["web", "webview", "native", "server", "liveview"])
                    .multiple(false)
                    .required(false),
            )
    }

    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        Self::augment_args(cmd)
    }
}

impl FromArgMatches for RendererArg {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        if let Some(renderer) = matches.get_one::<clap::Id>("renderer") {
            match renderer.as_str() {
                "web" => Ok(Self {
                    renderer: Some(Renderer::Web),
                }),
                "webview" => Ok(Self {
                    renderer: Some(Renderer::Webview),
                }),
                "native" => Ok(Self {
                    renderer: Some(Renderer::Native),
                }),
                "server" => Ok(Self {
                    renderer: Some(Renderer::Server),
                }),
                "liveview" => Ok(Self {
                    renderer: Some(Renderer::Liveview),
                }),
                _ => Err(clap::Error::raw(
                    clap::error::ErrorKind::InvalidValue,
                    format!("Unknown platform: {}", renderer),
                )),
            }
        } else {
            Ok(Self { renderer: None })
        }
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        *self = Self::from_arg_matches(matches)?;
        Ok(())
    }
}

impl Into<Option<Renderer>> for RendererArg {
    fn into(self) -> Option<Renderer> {
        self.renderer
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
#[non_exhaustive]
pub(crate) enum Renderer {
    /// Targeting webview renderer
    Webview,

    /// Targeting native renderer
    Native,

    /// Targeting the server platform using Axum and Dioxus-Fullstack
    ///
    /// This is implicitly passed if `fullstack` is enabled as a feature. Using this variant simply
    /// means you're only building the server variant without the `.wasm` to serve.
    Server,

    /// Targeting the static generation platform using SSR and Dioxus-Fullstack
    Liveview,

    /// Targeting the web renderer
    Web,
}

impl Renderer {
    /// Get the feature name for the platform in the dioxus crate
    pub(crate) fn feature_name(&self, target: &Triple) -> &str {
        match self {
            Renderer::Webview => match (target.environment, target.operating_system) {
                (Environment::Android, _) | (_, OperatingSystem::IOS(_)) => "mobile",
                _ => "desktop",
            },
            Renderer::Native => "native",
            Renderer::Server => "server",
            Renderer::Liveview => "liveview",
            Renderer::Web => "web",
        }
    }

    pub(crate) fn autodetect_from_cargo_feature(feature: &str) -> Option<Self> {
        match feature {
            "web" => Some(Self::Web),
            "desktop" | "mobile" => Some(Self::Webview),
            "native" => Some(Self::Native),
            "liveview" => Some(Self::Liveview),
            "server" => Some(Self::Server),
            _ => None,
        }
    }

    pub(crate) fn default_platform(&self) -> PlatformArg {
        match self {
            Renderer::Webview | Renderer::Native | Renderer::Server | Renderer::Liveview => {
                PlatformArg::TARGET_PLATFORM.unwrap()
            }
            Renderer::Web => PlatformArg::Wasm,
        }
    }

    pub(crate) fn from_target(triple: &Triple) -> Self {
        match triple.architecture {
            // Assume any wasm32 or wasm64 target is a web target
            Architecture::Wasm32 | Architecture::Wasm64 => Self::Web,
            // Otherwise, assume webview for native targets
            _ => Self::Webview,
        }
    }
}

#[derive(Debug)]
pub(crate) struct UnknownRendererError;

impl std::error::Error for UnknownRendererError {}

impl std::fmt::Display for UnknownRendererError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown renderer")
    }
}

impl FromStr for Renderer {
    type Err = UnknownRendererError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "webview" => Ok(Self::Webview),
            "native" => Ok(Self::Native),
            "server" => Ok(Self::Server),
            "liveview" => Ok(Self::Liveview),
            "web" => Ok(Self::Web),
            _ => Err(UnknownRendererError),
        }
    }
}

impl Display for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Renderer::Webview => "webview",
            Renderer::Native => "native",
            Renderer::Server => "server",
            Renderer::Liveview => "liveview",
            Renderer::Web => "web",
        })
    }
}
#[derive(
    Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug, Default,
)]
#[non_exhaustive]
pub(crate) enum BundleFormat {
    /// Targeting the web bundle structure
    #[serde(rename = "web")]
    #[default]
    Web,

    /// Targeting the macos desktop bundle structure
    #[serde(rename = "macos")]
    MacOS,

    /// Targeting the windows desktop bundle structure
    #[serde(rename = "windows")]
    Windows,

    /// Targeting the linux desktop bundle structure
    #[serde(rename = "linux")]
    Linux,

    /// Targeting the server bundle structure (a single binary placed next to the web build)
    #[serde(rename = "server")]
    Server,

    /// Targeting the ios bundle structure
    ///
    /// Can't work properly if you're not building from an Apple device.
    #[serde(rename = "ios")]
    Ios,

    /// Targeting the android bundle structure
    #[serde(rename = "android")]
    Android,
}

#[derive(Debug)]
pub(crate) struct UnknownBundleFormatError;

impl std::error::Error for UnknownBundleFormatError {}

impl std::fmt::Display for UnknownBundleFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown bundle format")
    }
}

impl FromStr for BundleFormat {
    type Err = UnknownBundleFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "web" => Ok(Self::Web),
            "macos" => Ok(Self::MacOS),
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            "server" => Ok(Self::Server),
            "ios" => Ok(Self::Ios),
            "android" => Ok(Self::Android),
            _ => Err(UnknownBundleFormatError),
        }
    }
}

impl Display for BundleFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            BundleFormat::Web => "web",
            BundleFormat::MacOS => "macos",
            BundleFormat::Windows => "windows",
            BundleFormat::Linux => "linux",
            BundleFormat::Server => "server",
            BundleFormat::Ios => "ios",
            BundleFormat::Android => "android",
        })
    }
}

impl BundleFormat {
    #[cfg(target_os = "macos")]
    pub(crate) const TARGET_PLATFORM: Option<Self> = Some(Self::MacOS);
    #[cfg(target_os = "windows")]
    pub(crate) const TARGET_PLATFORM: Option<Self> = Some(Self::Windows);
    #[cfg(target_os = "linux")]
    pub(crate) const TARGET_PLATFORM: Option<Self> = Some(Self::Linux);
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    pub(crate) const TARGET_PLATFORM: Option<Self> = None;

    /// Get the name of the folder we need to generate for this platform
    ///
    /// Note that web and server share the same platform folder since we'll export the web folder as a bundle on its own
    pub(crate) fn build_folder_name(&self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Server => "web",
            Self::Ios => "ios",
            Self::Android => "android",
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::MacOS => "macos",
        }
    }

    pub(crate) fn profile_name(&self, release: bool) -> String {
        let base_profile = match self {
            Self::MacOS | Self::Windows | Self::Linux => "desktop",
            Self::Web => "wasm",
            Self::Ios => "ios",
            Self::Android => "android",
            Self::Server => "server",
        };

        let opt_level = if release { "release" } else { "dev" };

        format!("{}-{}", base_profile, opt_level)
    }

    pub(crate) fn expected_name(&self) -> &'static str {
        match self {
            Self::Web => "Web",
            Self::MacOS => "Desktop MacOS",
            Self::Windows => "Desktop Windows",
            Self::Linux => "Desktop Linux",
            Self::Ios => "Mobile iOS",
            Self::Android => "Mobile Android",
            Self::Server => "Server",
        }
    }

    pub(crate) fn from_target(target: &Triple, renderer: Option<Renderer>) -> Result<BundleFormat> {
        match (
            renderer,
            target.architecture,
            target.environment,
            target.operating_system,
        ) {
            // The server always uses the server bundle format
            (Some(Renderer::Server), _, _, _) => Ok(BundleFormat::Server),
            // The web renderer always uses the web bundle format
            (Some(Renderer::Web), _, _, _) => Ok(BundleFormat::Web),
            // Otherwise, guess it based on the target
            // Assume any wasm32 or wasm64 target is a web target
            (_, Architecture::Wasm32 | Architecture::Wasm64, _, _) => Ok(BundleFormat::Web),
            // For native targets, we need to determine the bundle format based on the OS
            (_, _, Environment::Android, _) => Ok(BundleFormat::Android),
            (_, _, _, OperatingSystem::IOS(_)) => Ok(BundleFormat::Ios),
            (_, _, _, OperatingSystem::MacOSX(_) | OperatingSystem::Darwin(_)) => {
                Ok(BundleFormat::MacOS)
            }
            (_, _, _, OperatingSystem::Linux) => Ok(BundleFormat::Linux),
            (_, _, _, OperatingSystem::Windows) => Ok(BundleFormat::Windows),
            // If we don't recognize the target, default to desktop
            _ => BundleFormat::TARGET_PLATFORM.context(
                "failed to determine bundle format. Try setting the `--bundle` flag manually",
            ),
        }
    }
}
