use anyhow::{Context, Result};
use clap::{arg, ArgMatches, Args, FromArgMatches};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;
use target_lexicon::{Architecture, Environment, OperatingSystem, Triple};

use crate::{triple_is_wasm, Workspace};

#[derive(
    Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug, Default,
)]
#[non_exhaustive]
pub(crate) enum TargetAlias {
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

    /// Targeting the current target
    Host,

    /// An unknown target platform
    #[default]
    Unknown,
}

impl Args for TargetAlias {
    fn augment_args(cmd: clap::Command) -> clap::Command {
        const HELP_HEADING: &str = "Target Alias";
        cmd.arg(arg!(--wasm "Target the wasm triple").help_heading(HELP_HEADING))
            .arg(arg!(--macos "Target the macos triple").help_heading(HELP_HEADING))
            .arg(arg!(--windows "Target the windows triple").help_heading(HELP_HEADING))
            .arg(arg!(--linux "Target the linux triple").help_heading(HELP_HEADING))
            .arg(arg!(--ios "Target the ios triple").help_heading(HELP_HEADING))
            .arg(arg!(--android "Target the android triple").help_heading(HELP_HEADING))
            .arg(arg!(--host "Target the host triple").help_heading(HELP_HEADING))
            .arg(arg!(--desktop "Target the host triple").help_heading(HELP_HEADING))
            .group(
                clap::ArgGroup::new("target_alias")
                    .args([
                        "wasm", "macos", "windows", "linux", "ios", "android", "host", "host",
                    ])
                    .multiple(false)
                    .required(false),
            )
    }

    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        Self::augment_args(cmd)
    }
}

impl FromArgMatches for TargetAlias {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        if let Some(platform) = matches.get_one::<clap::Id>("target_alias") {
            match platform.as_str() {
                "wasm" => Ok(Self::Wasm),
                "macos" => Ok(Self::MacOS),
                "windows" => Ok(Self::Windows),
                "linux" => Ok(Self::Linux),
                "ios" => Ok(Self::Ios),
                "android" => Ok(Self::Android),
                "host" => Ok(Self::Host),
                _ => Err(clap::Error::raw(
                    clap::error::ErrorKind::InvalidValue,
                    format!("Unknown target alias: {platform}"),
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

impl TargetAlias {
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
            Self::MacOS | Self::Windows | Self::Linux | Self::Host | Self::Unknown => {
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

    pub(crate) fn or(self, other: Self) -> Self {
        if self == Self::Unknown {
            other
        } else {
            self
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
        cmd.arg(arg!(--web "Enable the dioxus web renderer").help_heading(HELP_HEADING))
            .arg(arg!(--webview "Enable the dioxus webview renderer").help_heading(HELP_HEADING))
            .arg(arg!(--native "Enable the dioxus native renderer").help_heading(HELP_HEADING))
            .arg(arg!(--server "Enable the dioxus server renderer").help_heading(HELP_HEADING))
            .arg(arg!(--liveview "Enable the dioxus liveview renderer").help_heading(HELP_HEADING))
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
                    format!("Unknown platform: {renderer}"),
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

impl From<RendererArg> for Option<Renderer> {
    fn from(val: RendererArg) -> Self {
        val.renderer
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

    pub(crate) fn default_platform(&self) -> TargetAlias {
        match self {
            Renderer::Webview | Renderer::Native | Renderer::Server | Renderer::Liveview => {
                TargetAlias::TARGET_PLATFORM.unwrap()
            }
            Renderer::Web => TargetAlias::Wasm,
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

    pub(crate) fn compatible_with(
        &self,
        target: &Option<Triple>,
        target_alias: TargetAlias,
    ) -> bool {
        let web_target = match (target, target_alias) {
            (Some(triple), _) if triple_is_wasm(triple) => true,
            (None, TargetAlias::Wasm) => true,
            _ => false,
        };
        // Web builds are only compatible with the web, liveview, and server renderers
        if web_target {
            matches!(self, Self::Web | Self::Liveview | Self::Server)
        } else {
            false
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
    #[cfg_attr(target_os = "macos", serde(alias = "desktop"))]
    #[serde(rename = "macos")]
    MacOS,

    /// Targeting the windows desktop bundle structure
    #[cfg_attr(target_os = "windows", serde(alias = "desktop"))]
    #[serde(rename = "windows")]
    Windows,

    /// Targeting the linux desktop bundle structure
    #[cfg_attr(target_os = "linux", serde(alias = "desktop"))]
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

        format!("{base_profile}-{opt_level}")
    }

    pub(crate) fn expected_name(&self) -> &'static str {
        match self {
            Self::Web => "Web",
            Self::MacOS => "MacOS",
            Self::Windows => "Windows",
            Self::Linux => "Linux",
            Self::Ios => "iOS",
            Self::Android => "Android",
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
    /// Alias for `--wasm --web --bundle-format web`
    #[clap(name = "web")]
    #[serde(rename = "web")]
    #[default]
    Web,

    /// Alias for `--macos --webview --bundle-format macos`
    #[cfg_attr(target_os = "macos", clap(alias = "desktop"))]
    #[clap(name = "macos")]
    #[serde(rename = "macos")]
    MacOS,

    /// Alias for `--windows --webview --bundle-format windows`
    #[cfg_attr(target_os = "windows", clap(alias = "desktop"))]
    #[clap(name = "windows")]
    #[serde(rename = "windows")]
    Windows,

    /// Alias for `--linux --webview --bundle-format linux`
    #[cfg_attr(target_os = "linux", clap(alias = "desktop"))]
    #[clap(name = "linux")]
    #[serde(rename = "linux")]
    Linux,

    /// Alias for `--ios --webview --bundle-format ios`
    #[clap(name = "ios")]
    #[serde(rename = "ios")]
    Ios,

    /// Alias for `--android --webview --bundle-format android`
    #[clap(name = "android")]
    #[serde(rename = "android")]
    Android,

    /// Alias for `--host --server --bundle-format server`
    #[clap(name = "server")]
    #[serde(rename = "server")]
    Server,

    /// Alias for `--host --liveview --bundle-format host`
    #[clap(name = "liveview")]
    #[serde(rename = "liveview")]
    Liveview,
}

impl Platform {
    pub(crate) fn into_triple(self) -> (TargetAlias, Renderer, BundleFormat) {
        match self {
            Platform::Web => (TargetAlias::Wasm, Renderer::Web, BundleFormat::Web),
            Platform::MacOS => (TargetAlias::MacOS, Renderer::Webview, BundleFormat::MacOS),
            Platform::Windows => (
                TargetAlias::Windows,
                Renderer::Webview,
                BundleFormat::Windows,
            ),
            Platform::Linux => (TargetAlias::Linux, Renderer::Webview, BundleFormat::Linux),
            Platform::Ios => (TargetAlias::Ios, Renderer::Webview, BundleFormat::Ios),
            Platform::Android => (
                TargetAlias::Android,
                Renderer::Webview,
                BundleFormat::Android,
            ),
            Platform::Server => (TargetAlias::Host, Renderer::Server, BundleFormat::Server),
            Platform::Liveview => (
                TargetAlias::Host,
                Renderer::Liveview,
                BundleFormat::TARGET_PLATFORM.unwrap(),
            ),
        }
    }
}
