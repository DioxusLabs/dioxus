use anyhow::Result;
use clap::{arg, Arg, ArgMatches, Args, FromArgMatches};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;
use target_lexicon::{Environment, OperatingSystem, Triple};

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

impl Platform {
    fn from_identifier(identifier: &str) -> std::result::Result<Self, clap::Error> {
        match identifier {
            "web" => Ok(Self::Web),
            "macos" => Ok(Self::MacOS),
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            "ios" => Ok(Self::Ios),
            "android" => Ok(Self::Android),
            "server" => Ok(Self::Server),
            "liveview" => Ok(Self::Liveview),
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
            _ => Err(clap::Error::raw(
                clap::error::ErrorKind::InvalidValue,
                format!("Unknown platform: {identifier}"),
            )),
        }
    }
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
            .arg(
                Arg::new("platform")
                    .long("platform")
                    .value_name("PLATFORM")
                    .help("Manually set the platform (web, macos, windows, linux, ios, android, server, liveview)")
                    .help_heading(HELP_HEADING)
                    .value_parser([
                        "web", "macos", "windows", "linux", "ios", "android", "server", "liveview", "desktop",
                    ])
                    .conflicts_with("target_alias"),
            )
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
        if let Some(identifier) = matches.get_one::<String>("platform") {
            Self::from_identifier(identifier)
        } else if let Some(platform) = matches.get_one::<clap::Id>("target_alias") {
            Self::from_identifier(platform.as_str())
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

impl BundleFormat {
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
