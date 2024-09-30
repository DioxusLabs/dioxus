use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct BundleConfig {
    pub(crate) identifier: Option<String>,
    pub(crate) publisher: Option<String>,
    pub(crate) icon: Option<Vec<String>>,
    pub(crate) resources: Option<Vec<String>>,
    pub(crate) copyright: Option<String>,
    pub(crate) category: Option<String>,
    pub(crate) short_description: Option<String>,
    pub(crate) long_description: Option<String>,
    pub(crate) external_bin: Option<Vec<String>>,
    pub(crate) deb: Option<DebianSettings>,
    pub(crate) macos: Option<MacOsSettings>,
    pub(crate) windows: Option<WindowsSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct DebianSettings {
    pub(crate) depends: Option<Vec<String>>,
    pub(crate) files: HashMap<PathBuf, PathBuf>,
    pub(crate) nsis: Option<NsisSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct WixSettings {
    pub(crate) language: Vec<(String, Option<PathBuf>)>,
    pub(crate) template: Option<PathBuf>,
    pub(crate) fragment_paths: Vec<PathBuf>,
    pub(crate) component_group_refs: Vec<String>,
    pub(crate) component_refs: Vec<String>,
    pub(crate) feature_group_refs: Vec<String>,
    pub(crate) feature_refs: Vec<String>,
    pub(crate) merge_refs: Vec<String>,
    pub(crate) skip_webview_install: bool,
    pub(crate) license: Option<PathBuf>,
    pub(crate) enable_elevated_update_task: bool,
    pub(crate) banner_path: Option<PathBuf>,
    pub(crate) dialog_image_path: Option<PathBuf>,
    pub(crate) fips_compliant: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct MacOsSettings {
    pub(crate) frameworks: Option<Vec<String>>,
    pub(crate) minimum_system_version: Option<String>,
    pub(crate) license: Option<String>,
    pub(crate) exception_domain: Option<String>,
    pub(crate) signing_identity: Option<String>,
    pub(crate) provider_short_name: Option<String>,
    pub(crate) entitlements: Option<String>,
    pub(crate) info_plist_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WindowsSettings {
    pub(crate) digest_algorithm: Option<String>,
    pub(crate) certificate_thumbprint: Option<String>,
    pub(crate) timestamp_url: Option<String>,
    pub(crate) tsp: bool,
    pub(crate) wix: Option<WixSettings>,
    pub(crate) icon_path: Option<PathBuf>,
    pub(crate) webview_install_mode: WebviewInstallMode,
    pub(crate) webview_fixed_runtime_path: Option<PathBuf>,
    pub(crate) allow_downgrades: bool,
    pub(crate) nsis: Option<NsisSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct NsisSettings {
    pub(crate) template: Option<PathBuf>,
    pub(crate) license: Option<PathBuf>,
    pub(crate) header_image: Option<PathBuf>,
    pub(crate) sidebar_image: Option<PathBuf>,
    pub(crate) installer_icon: Option<PathBuf>,
    pub(crate) install_mode: NSISInstallerMode,
    pub(crate) languages: Option<Vec<String>>,
    pub(crate) custom_language_files: Option<HashMap<String, PathBuf>>,
    pub(crate) display_language_selector: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum NSISInstallerMode {
    CurrentUser,
    PerMachine,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum WebviewInstallMode {
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
