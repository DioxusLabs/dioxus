use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DebianSettings {
    pub depends: Option<Vec<String>>,
    pub files: HashMap<PathBuf, PathBuf>,
    pub nsis: Option<NsisSettings>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NSISInstallerMode {
    CurrentUser,
    PerMachine,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebviewInstallMode {
    Skip,
    DownloadBootstrapper { silent: bool },
    EmbedBootstrapper { silent: bool },
    OfflineInstaller { silent: bool },
    FixedRuntime { path: PathBuf },
}

impl Default for WebviewInstallMode {
    fn default() -> Self {
        Self::OfflineInstaller { silent: false }
    }
}
