use anyhow::{Context, Result};
use clap::{arg, ArgMatches, Args, FromArgMatches};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;
use target_lexicon::{Architecture, Environment, OperatingSystem, Triple};

#[derive(
    Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug, Default,
)]
#[non_exhaustive]
pub(crate) enum Platform {
    /// Alias for `--target wasm32-unknown-unknown --renderer websys --bundle-format web`
    #[serde(rename = "web")]
    Web,

    /// Alias for `--target <host> --renderer webview --bundle-format macos`
    #[serde(rename = "macos")]
    MacOS,

    /// Alias for `--target <host> --renderer webview --bundle-format windows`
    #[serde(rename = "windows")]
    Windows,

    /// Alias for `--target <host> --renderer webview --bundle-format linux`
    #[serde(rename = "linux")]
    Linux,

    /// Alias for `--target <aarch64-apple-ios/sim> --renderer webview --bundle-format ios`
    #[serde(rename = "ios")]
    Ios,

    /// Alias for `--target <device-triple> --renderer webview --bundle-format android`
    #[serde(rename = "android")]
    Android,

    /// Alias for `--target <host> --renderer ssr --bundle-format server`
    #[serde(rename = "server")]
    Server,

    /// Alias for `--target <host> --renderer liveview --bundle-format server`
    #[serde(rename = "liveview")]
    Liveview,

    /// No platform was specified, so the CLI is free to choose the best one.
    #[default]
    Unknown,
}

impl Args for Platform {
    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        Self::augment_args(cmd)
    }

    fn augment_args(cmd: clap::Command) -> clap::Command {
        const HELP_HEADING: &str = "Platform";
        cmd.arg(arg!(--web "Target a web app").help_heading(HELP_HEADING))
            .arg(arg!(--desktop "Target a desktop app").help_heading(HELP_HEADING))
            .arg(arg!(--macos "Target a macos desktop app").help_heading(HELP_HEADING))
            .arg(arg!(--windows "Target a windows desktop app").help_heading(HELP_HEADING))
            .arg(arg!(--linux "Target a linux desktop app").help_heading(HELP_HEADING))
            .arg(arg!(--ios "Target an ios app").help_heading(HELP_HEADING))
            .arg(arg!(--android "Target an android app").help_heading(HELP_HEADING))
            .arg(arg!(--server "Target a server build").help_heading(HELP_HEADING))
            .arg(arg!(--liveview "Target a liveview build").help_heading(HELP_HEADING))
            .group(
                clap::ArgGroup::new("target_alias")
                    .args([
                        "web", "desktop", "macos", "windows", "linux", "ios", "android", "server",
                        "liveview",
                    ])
                    .multiple(false)
                    .required(false),
            )
    }
}

impl FromArgMatches for Platform {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        if let Some(platform) = matches.get_one::<clap::Id>("target_alias") {
            match platform.as_str() {
                "web" => Ok(Self::Web),
                "desktop" => {
                    if cfg!(target_os = "macos") {
                        Ok(Self::MacOS)
                    } else if cfg!(target_os = "windows") {
                        Ok(Self::Windows)
                    } else if cfg!(unix) {
                        Ok(Self::Linux)
                    } else {
                        Err(clap::Error::raw(
                            clap::error::ErrorKind::InvalidValue,
                            "Desktop alias is not supported on this platform",
                        ))
                    }
                }
                "macos" => Ok(Self::MacOS),
                "windows" => Ok(Self::Windows),
                "linux" => Ok(Self::Linux),
                "ios" => Ok(Self::Ios),
                "android" => Ok(Self::Android),
                "liveview" => Ok(Self::Liveview),
                "server" => Ok(Self::Server),
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

    /// Targeting the web-sys renderer
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

    pub(crate) fn default_triple(&self) -> Triple {
        match self {
            Self::Webview => Triple::host(),
            Self::Native => Triple::host(),
            Self::Server => Triple::host(),
            Self::Liveview => Triple::host(),
            Self::Web => "wasm32-unknown-unknown".parse().unwrap(),
            // Self::Custom => Triple::host(),
        }
    }

    pub(crate) fn default_bundle_format(&self) -> BundleFormat {
        match self {
            Self::Webview | Self::Native => {
                if cfg!(target_os = "macos") {
                    BundleFormat::MacOS
                } else if cfg!(target_os = "windows") {
                    BundleFormat::Windows
                } else if cfg!(unix) {
                    BundleFormat::Linux
                } else {
                    BundleFormat::Linux
                }
            }
            Self::Server => BundleFormat::Server,
            Self::Liveview => BundleFormat::Server,
            Self::Web => BundleFormat::Web,
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
            Self::Webview => todo!(),
            Self::Native => todo!(),
            Self::Server => todo!(),
            Self::Liveview => todo!(),
            Self::Web => todo!(),
            // Self::Custom => todo!(),
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

impl BundleFormat {
    #[cfg(target_os = "macos")]
    pub(crate) const TARGET_PLATFORM: Option<Self> = Some(Self::MacOS);
    #[cfg(target_os = "windows")]
    pub(crate) const TARGET_PLATFORM: Option<Self> = Some(Self::Windows);
    #[cfg(target_os = "linux")]
    pub(crate) const TARGET_PLATFORM: Option<Self> = Some(Self::Linux);
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    pub(crate) const TARGET_PLATFORM: Option<Self> = None;

    /// The native "desktop" host app format.
    pub(crate) fn host() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOS
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Web
        }
    }

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

// #[derive(
//     Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug, Default,
// )]
// #[non_exhaustive]
// pub(crate) enum PlatformAlias {
//     /// Targeting a WASM app
//     Wasm,

//     /// Targeting macos desktop
//     MacOS,

//     /// Targeting windows desktop
//     Windows,

//     /// Targeting linux desktop
//     Linux,

//     /// Targeting the ios platform
//     ///
//     /// Can't work properly if you're not building from an Apple device.
//     Ios,

//     /// Targeting the android platform
//     Android,

//     /// An unknown target platform
//     #[default]
//     Unknown,
// }

// impl TargetAlias {
//     #[cfg(target_os = "macos")]
//     pub(crate) const TARGET_PLATFORM: Option<Self> = Some(Self::MacOS);
//     #[cfg(target_os = "windows")]
//     pub(crate) const TARGET_PLATFORM: Option<Self> = Some(Self::Windows);
//     #[cfg(target_os = "linux")]
//     pub(crate) const TARGET_PLATFORM: Option<Self> = Some(Self::Linux);
//     #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
//     pub(crate) const TARGET_PLATFORM: Option<Self> = None;

//     pub(crate) async fn into_target(self, device: bool, workspace: &Workspace) -> Result<Triple> {
//         match self {
//             // Generally just use the host's triple for native executables unless specified otherwise
//             Self::MacOS | Self::Windows | Self::Linux | Self::Host | Self::Unknown => {
//                 Ok(Triple::host())
//             }

//             // We currently assume unknown-unknown for web, but we might want to eventually
//             // support emscripten
//             Self::Wasm => Ok("wasm32-unknown-unknown".parse()?),

//             // For iOS we should prefer the actual architecture for the simulator, but in lieu of actually
//             // figuring that out, we'll assume aarch64 on m-series and x86_64 otherwise
//             Self::Ios => {
//                 // use the host's architecture and sim if --device is passed
//                 use target_lexicon::{Architecture, HOST};
//                 let triple_str = match HOST.architecture {
//                     Architecture::Aarch64(_) if device => "aarch64-apple-ios",
//                     Architecture::Aarch64(_) => "aarch64-apple-ios-sim",
//                     _ if device => "x86_64-apple-ios",
//                     _ => "x86_64-apple-ios",
//                 };
//                 Ok(triple_str.parse()?)
//             }

//             // Same idea with android but we figure out the connected device using adb
//             Self::Android => Ok(workspace
//                 .android_tools()?
//                 .autodetect_android_device_triple()
//                 .await),
//         }
//     }

//     pub(crate) fn or(self, other: Self) -> Self {
//         if self == Self::Unknown {
//             other
//         } else {
//             self
//         }
//     }
// }

// #[derive(
//     Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug, Default,
// )]
// pub(crate) struct RendererArg {
//     pub(crate) renderer: Option<Renderer>,
// }

// impl Args for RendererArg {
//     fn augment_args(cmd: clap::Command) -> clap::Command {
//         const HELP_HEADING: &str = "Renderer";
//         cmd.arg(arg!(--web "Enable the dioxus web renderer").help_heading(HELP_HEADING))
//             .arg(arg!(--webview "Enable the dioxus webview renderer").help_heading(HELP_HEADING))
//             .arg(arg!(--native "Enable the dioxus native renderer").help_heading(HELP_HEADING))
//             .arg(arg!(--server "Enable the dioxus server renderer").help_heading(HELP_HEADING))
//             .arg(arg!(--liveview "Enable the dioxus liveview renderer").help_heading(HELP_HEADING))
//             .group(
//                 clap::ArgGroup::new("renderer")
//                     .args(["web", "webview", "native", "server", "liveview"])
//                     .multiple(false)
//                     .required(false),
//             )
//     }

//     fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
//         Self::augment_args(cmd)
//     }
// }

// impl FromArgMatches for RendererArg {
//     fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
//         if let Some(renderer) = matches.get_one::<clap::Id>("renderer") {
//             match renderer.as_str() {
//                 "web" => Ok(Self {
//                     renderer: Some(Renderer::Web),
//                 }),
//                 "webview" => Ok(Self {
//                     renderer: Some(Renderer::Webview),
//                 }),
//                 "native" => Ok(Self {
//                     renderer: Some(Renderer::Native),
//                 }),
//                 "server" => Ok(Self {
//                     renderer: Some(Renderer::Server),
//                 }),
//                 "liveview" => Ok(Self {
//                     renderer: Some(Renderer::Liveview),
//                 }),
//                 _ => Err(clap::Error::raw(
//                     clap::error::ErrorKind::InvalidValue,
//                     format!("Unknown platform: {renderer}"),
//                 )),
//             }
//         } else {
//             Ok(Self { renderer: None })
//         }
//     }

//     fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
//         *self = Self::from_arg_matches(matches)?;
//         Ok(())
//     }
// }

// impl From<RendererArg> for Option<Renderer> {
//     fn from(val: RendererArg) -> Self {
//         val.renderer
//     }
// }

impl Platform {
    // pub(crate) fn into_triple(self) -> (TargetAlias, Renderer, BundleFormat) {
    //     match self {
    //         Platform::Web => (TargetAlias::Wasm, Renderer::Web, BundleFormat::Web),
    //         Platform::MacOS => (TargetAlias::MacOS, Renderer::Webview, BundleFormat::MacOS),
    //         Platform::Windows => (
    //             TargetAlias::Windows,
    //             Renderer::Webview,
    //             BundleFormat::Windows,
    //         ),
    //         Platform::Linux => (TargetAlias::Linux, Renderer::Webview, BundleFormat::Linux),
    //         Platform::Ios => (TargetAlias::Ios, Renderer::Webview, BundleFormat::Ios),
    //         Platform::Android => (
    //             TargetAlias::Android,
    //             Renderer::Webview,
    //             BundleFormat::Android,
    //         ),
    //         Platform::Server => (TargetAlias::Host, Renderer::Server, BundleFormat::Server),
    //         Platform::Liveview => (
    //             TargetAlias::Host,
    //             Renderer::Liveview,
    //             BundleFormat::TARGET_PLATFORM.unwrap(),
    //         ),
    //     }
    // }
}
