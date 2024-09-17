use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::net::{IpAddr, SocketAddr};
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

    /// Targeting the static generation platform using SSR and Dioxus-Fullstack
    #[cfg_attr(feature = "cli", clap(name = "liveview"))]
    #[serde(rename = "liveview")]
    Liveview,
}

/// An error that occurs when a platform is not recognized
pub struct UnknownPlatformError;

impl std::error::Error for UnknownPlatformError {}

impl std::fmt::Debug for UnknownPlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown platform")
    }
}
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
            Platform::Liveview => "liveview",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DioxusConfig {
    pub application: ApplicationConfig,

    #[serde(default)]
    pub web: WebConfig,

    #[serde(default)]
    pub desktop: DesktopConfig,

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
            desktop: DesktopConfig::default(),
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

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            pre_compress: true_bool(),
            app: Default::default(),
            https: Default::default(),
            wasm_opt: Default::default(),
            proxy: Default::default(),
            watcher: Default::default(),
            resource: Default::default(),
        }
    }
}

/// Represents configuration items for the desktop platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopConfig {
    /// Describes whether a debug-mode desktop app should be always-on-top.
    #[serde(default)]
    pub always_on_top: bool,
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            always_on_top: true,
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BundleConfig {
    pub identifier: Option<String>,
    pub publisher: Option<String>,
    pub icon: Option<Vec<String>>,
    pub resources: Option<Vec<String>>,
    pub copyright: Option<String>,
    pub category: Option<String>,
    pub short_description: Option<String>,
    pub long_description: Option<String>,
    pub external_bin: Option<Vec<String>>,
    pub deb: Option<DebianSettings>,
    pub macos: Option<MacOsSettings>,
    pub windows: Option<WindowsSettings>,
}

impl From<BundleConfig> for tauri_bundler::BundleSettings {
    fn from(val: BundleConfig) -> Self {
        tauri_bundler::BundleSettings {
            identifier: val.identifier,
            publisher: val.publisher,
            icon: val.icon,
            resources: val.resources,
            copyright: val.copyright,
            category: val.category.and_then(|c| c.parse().ok()),
            short_description: val.short_description,
            long_description: val.long_description,
            external_bin: val.external_bin,
            deb: val.deb.map(Into::into).unwrap_or_default(),
            macos: val.macos.map(Into::into).unwrap_or_default(),
            windows: val.windows.map(Into::into).unwrap_or_default(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DebianSettings {
    pub depends: Option<Vec<String>>,
    pub files: HashMap<PathBuf, PathBuf>,
    pub nsis: Option<NsisSettings>,
}

impl From<DebianSettings> for tauri_bundler::DebianSettings {
    fn from(val: DebianSettings) -> Self {
        tauri_bundler::DebianSettings {
            depends: val.depends,
            files: val.files,
            desktop_template: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WixSettings {
    pub language: Vec<(String, Option<PathBuf>)>,
    pub template: Option<PathBuf>,
    pub fragment_paths: Vec<PathBuf>,
    pub component_group_refs: Vec<String>,
    pub component_refs: Vec<String>,
    pub feature_group_refs: Vec<String>,
    pub feature_refs: Vec<String>,
    pub merge_refs: Vec<String>,
    pub skip_webview_install: bool,
    pub license: Option<PathBuf>,
    pub enable_elevated_update_task: bool,
    pub banner_path: Option<PathBuf>,
    pub dialog_image_path: Option<PathBuf>,
    pub fips_compliant: bool,
}

impl From<WixSettings> for tauri_bundler::WixSettings {
    fn from(val: WixSettings) -> Self {
        tauri_bundler::WixSettings {
            language: tauri_bundler::bundle::WixLanguage({
                let mut languages: Vec<_> = val
                    .language
                    .iter()
                    .map(|l| {
                        (
                            l.0.clone(),
                            tauri_bundler::bundle::WixLanguageConfig {
                                locale_path: l.1.clone(),
                            },
                        )
                    })
                    .collect();
                if languages.is_empty() {
                    languages.push(("en-US".into(), Default::default()));
                }
                languages
            }),
            template: val.template,
            fragment_paths: val.fragment_paths,
            component_group_refs: val.component_group_refs,
            component_refs: val.component_refs,
            feature_group_refs: val.feature_group_refs,
            feature_refs: val.feature_refs,
            merge_refs: val.merge_refs,
            skip_webview_install: val.skip_webview_install,
            license: val.license,
            enable_elevated_update_task: val.enable_elevated_update_task,
            banner_path: val.banner_path,
            dialog_image_path: val.dialog_image_path,
            fips_compliant: val.fips_compliant,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacOsSettings {
    pub frameworks: Option<Vec<String>>,
    pub minimum_system_version: Option<String>,
    pub license: Option<String>,
    pub exception_domain: Option<String>,
    pub signing_identity: Option<String>,
    pub provider_short_name: Option<String>,
    pub entitlements: Option<String>,
    pub info_plist_path: Option<PathBuf>,
}

impl From<MacOsSettings> for tauri_bundler::MacOsSettings {
    fn from(val: MacOsSettings) -> Self {
        tauri_bundler::MacOsSettings {
            frameworks: val.frameworks,
            minimum_system_version: val.minimum_system_version,
            license: val.license,
            exception_domain: val.exception_domain,
            signing_identity: val.signing_identity,
            provider_short_name: val.provider_short_name,
            entitlements: val.entitlements,
            info_plist_path: val.info_plist_path,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsSettings {
    pub digest_algorithm: Option<String>,
    pub certificate_thumbprint: Option<String>,
    pub timestamp_url: Option<String>,
    pub tsp: bool,
    pub wix: Option<WixSettings>,
    pub icon_path: Option<PathBuf>,
    pub webview_install_mode: WebviewInstallMode,
    pub webview_fixed_runtime_path: Option<PathBuf>,
    pub allow_downgrades: bool,
    pub nsis: Option<NsisSettings>,
}

impl From<WindowsSettings> for tauri_bundler::WindowsSettings {
    fn from(val: WindowsSettings) -> Self {
        tauri_bundler::WindowsSettings {
            digest_algorithm: val.digest_algorithm,
            certificate_thumbprint: val.certificate_thumbprint,
            timestamp_url: val.timestamp_url,
            tsp: val.tsp,
            wix: val.wix.map(Into::into),
            icon_path: val.icon_path.unwrap_or("icons/icon.ico".into()),
            webview_install_mode: val.webview_install_mode.into(),
            webview_fixed_runtime_path: val.webview_fixed_runtime_path,
            allow_downgrades: val.allow_downgrades,
            nsis: val.nsis.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NsisSettings {
    pub template: Option<PathBuf>,
    pub license: Option<PathBuf>,
    pub header_image: Option<PathBuf>,
    pub sidebar_image: Option<PathBuf>,
    pub installer_icon: Option<PathBuf>,
    pub install_mode: NSISInstallerMode,
    pub languages: Option<Vec<String>>,
    pub custom_language_files: Option<HashMap<String, PathBuf>>,
    pub display_language_selector: bool,
}

impl From<NsisSettings> for tauri_bundler::NsisSettings {
    fn from(val: NsisSettings) -> Self {
        tauri_bundler::NsisSettings {
            license: val.license,
            header_image: val.header_image,
            sidebar_image: val.sidebar_image,
            installer_icon: val.installer_icon,
            install_mode: val.install_mode.into(),
            languages: val.languages,
            display_language_selector: val.display_language_selector,
            custom_language_files: None,
            template: None,
            compression: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NSISInstallerMode {
    CurrentUser,
    PerMachine,
    Both,
}

impl From<NSISInstallerMode> for tauri_utils::config::NSISInstallerMode {
    fn from(val: NSISInstallerMode) -> Self {
        match val {
            NSISInstallerMode::CurrentUser => tauri_utils::config::NSISInstallerMode::CurrentUser,
            NSISInstallerMode::PerMachine => tauri_utils::config::NSISInstallerMode::PerMachine,
            NSISInstallerMode::Both => tauri_utils::config::NSISInstallerMode::Both,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebviewInstallMode {
    Skip,
    DownloadBootstrapper { silent: bool },
    EmbedBootstrapper { silent: bool },
    OfflineInstaller { silent: bool },
    FixedRuntime { path: PathBuf },
}

impl WebviewInstallMode {
    fn into(self) -> tauri_utils::config::WebviewInstallMode {
        match self {
            Self::Skip => tauri_utils::config::WebviewInstallMode::Skip,
            Self::DownloadBootstrapper { silent } => {
                tauri_utils::config::WebviewInstallMode::DownloadBootstrapper { silent }
            }
            Self::EmbedBootstrapper { silent } => {
                tauri_utils::config::WebviewInstallMode::EmbedBootstrapper { silent }
            }
            Self::OfflineInstaller { silent } => {
                tauri_utils::config::WebviewInstallMode::OfflineInstaller { silent }
            }
            Self::FixedRuntime { path } => {
                tauri_utils::config::WebviewInstallMode::FixedRuntime { path }
            }
        }
    }
}

impl Default for WebviewInstallMode {
    fn default() -> Self {
        Self::OfflineInstaller { silent: false }
    }
}

/// The arguments for the address the server will run on

#[derive(Clone, Debug, Parser)]
pub struct AddressArguments {
    /// The port the server will run on
    #[clap(long)]
    #[clap(default_value_t = default_port())]
    pub port: u16,

    /// The address the server will run on
    #[clap(long, default_value_t = default_address())]
    pub addr: std::net::IpAddr,
}

impl Default for AddressArguments {
    fn default() -> Self {
        Self {
            port: default_port(),
            addr: default_address(),
        }
    }
}

impl AddressArguments {
    /// Get the address the server should run on
    pub fn address(&self) -> SocketAddr {
        SocketAddr::new(self.addr, self.port)
    }
}

fn default_port() -> u16 {
    8080
}

fn default_address() -> IpAddr {
    IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))
}
