use crate::BundleConfig;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(
    Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug, Default,
)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[non_exhaustive]
pub enum Platform {
    /// Targeting the web platform using WASM
    #[cfg_attr(feature = "cli", clap(name = "web"))]
    #[serde(rename = "web")]
    #[default]
    Web,

    /// Targeting the desktop platform using Tao/Wry-based webview
    #[cfg_attr(feature = "cli", clap(name = "desktop"))]
    #[serde(rename = "desktop")]
    Desktop,

    /// Targeting the server platform using Axum and Dioxus-Fullstack
    #[cfg_attr(feature = "cli", clap(name = "fullstack"))]
    #[serde(rename = "fullstack")]
    Fullstack,

    /// Targeting the static generation platform using SSR and Dioxus-Fullstack
    #[cfg_attr(feature = "cli", clap(name = "static-generation"))]
    #[serde(rename = "static-generation")]
    StaticGeneration,
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
            "static-generation" => Ok(Self::StaticGeneration),
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
    pub const ALL: &'static [Self] = &[
        Platform::Web,
        Platform::Desktop,
        Platform::Fullstack,
        Platform::StaticGeneration,
    ];

    /// Get the feature name for the platform in the dioxus crate
    pub fn feature_name(&self) -> &str {
        match self {
            Platform::Web => "web",
            Platform::Desktop => "desktop",
            Platform::Fullstack => "fullstack",
            Platform::StaticGeneration => "static-generation",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DioxusConfig {
    pub application: ApplicationConfig,

    pub web: WebConfig,

    #[serde(default)]
    pub bundle: BundleConfig,
}

impl Default for DioxusConfig {
    fn default() -> Self {
        let name = default_name();
        Self {
            application: ApplicationConfig {
                name: name.clone(),
                default_platform: default_platform(),
                out_dir: out_dir_default(),
                asset_dir: asset_dir_default(),

                sub_package: None,
            },
            web: WebConfig {
                app: WebAppConfig {
                    title: default_title(),
                    base_path: None,
                },
                proxy: vec![],
                watcher: Default::default(),
                resource: WebResourceConfig {
                    dev: WebDevResourceConfig {
                        style: vec![],
                        script: vec![],
                    },
                    style: Some(vec![]),
                    script: Some(vec![]),
                },
                https: WebHttpsConfig {
                    enabled: None,
                    mkcert: None,
                    key_path: None,
                    cert_path: None,
                },
                pre_compress: true,
                wasm_opt: Default::default(),
            },
            bundle: BundleConfig {
                identifier: Some(format!("io.github.{name}")),
                publisher: Some(name),
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    #[serde(default = "default_name")]
    pub name: String,

    #[serde(default = "default_platform")]
    pub default_platform: Platform,

    #[serde(default = "out_dir_default")]
    pub out_dir: PathBuf,

    #[serde(default = "asset_dir_default")]
    pub asset_dir: PathBuf,

    #[serde(default)]
    pub sub_package: Option<String>,
}

fn default_name() -> String {
    "my-cool-project".into()
}

fn default_platform() -> Platform {
    Platform::Web
}

fn asset_dir_default() -> PathBuf {
    PathBuf::from("public")
}

fn out_dir_default() -> PathBuf {
    PathBuf::from("dist")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    #[serde(default)]
    pub app: WebAppConfig,
    #[serde(default)]
    pub proxy: Vec<WebProxyConfig>,
    #[serde(default)]
    pub watcher: WebWatcherConfig,
    #[serde(default)]
    pub resource: WebResourceConfig,
    #[serde(default)]
    pub https: WebHttpsConfig,
    /// Whether to enable pre-compression of assets and wasm during a web build in release mode
    #[serde(default = "true_bool")]
    pub pre_compress: bool,
    /// The wasm-opt configuration
    #[serde(default)]
    pub wasm_opt: WasmOptConfig,
}

/// The wasm-opt configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WasmOptConfig {
    /// The wasm-opt level to use for release builds [default: s]
    /// Options:
    /// - z: optimize aggressively for size
    /// - s: optimize for size
    /// - 1: optimize for speed
    /// - 2: optimize for more for speed
    /// - 3: optimize for even more for speed
    /// - 4: optimize aggressively for speed
    #[serde(default)]
    pub level: WasmOptLevel,

    /// Keep debug symbols in the wasm file
    #[serde(default = "false_bool")]
    pub debug: bool,
}

/// The wasm-opt level to use for release web builds [default: 4]
#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum WasmOptLevel {
    /// Optimize aggressively for size
    #[serde(rename = "z")]
    Z,
    /// Optimize for size
    #[serde(rename = "s")]
    S,
    /// Don't optimize
    #[serde(rename = "0")]
    Zero,
    /// Optimize for speed
    #[serde(rename = "1")]
    One,
    /// Optimize for more for speed
    #[serde(rename = "2")]
    Two,
    /// Optimize for even more for speed
    #[serde(rename = "3")]
    Three,
    /// Optimize aggressively for speed
    #[serde(rename = "4")]
    #[default]
    Four,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAppConfig {
    #[serde(default = "default_title")]
    pub title: String,
    pub base_path: Option<String>,
}

impl WebAppConfig {
    /// Get the normalized base path for the application with `/` trimmed from both ends. If the base path is not set, this will return `.`.
    pub fn base_path(&self) -> &str {
        match &self.base_path {
            Some(path) => path.trim_matches('/'),
            None => ".",
        }
    }
}

impl Default for WebAppConfig {
    fn default() -> Self {
        Self {
            title: default_title(),
            base_path: None,
        }
    }
}

fn default_title() -> String {
    "dioxus | ⛺".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebProxyConfig {
    pub backend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebWatcherConfig {
    #[serde(default = "watch_path_default")]
    pub watch_path: Vec<PathBuf>,

    #[serde(default)]
    pub reload_html: bool,

    #[serde(default = "true_bool")]
    pub index_on_404: bool,
}

impl Default for WebWatcherConfig {
    fn default() -> Self {
        Self {
            watch_path: watch_path_default(),
            reload_html: false,
            index_on_404: true,
        }
    }
}

fn watch_path_default() -> Vec<PathBuf> {
    vec![PathBuf::from("src"), PathBuf::from("examples")]
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct WebResourceConfig {
    pub dev: WebDevResourceConfig,
    pub style: Option<Vec<PathBuf>>,
    pub script: Option<Vec<PathBuf>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct WebDevResourceConfig {
    #[serde(default)]
    pub style: Vec<PathBuf>,
    #[serde(default)]
    pub script: Vec<PathBuf>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WebHttpsConfig {
    pub enabled: Option<bool>,
    pub mkcert: Option<bool>,
    pub key_path: Option<String>,
    pub cert_path: Option<String>,
}

fn true_bool() -> bool {
    true
}

fn false_bool() -> bool {
    false
}
