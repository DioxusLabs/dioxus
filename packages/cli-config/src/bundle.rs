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

#[cfg(feature = "cli")]
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

#[cfg(feature = "cli")]
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

#[cfg(feature = "cli")]
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

#[cfg(feature = "cli")]
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

#[cfg(feature = "cli")]
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

#[cfg(feature = "cli")]
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

#[cfg(feature = "cli")]
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

#[cfg(feature = "cli")]
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
