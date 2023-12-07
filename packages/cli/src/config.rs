use crate::{cfg::Platform, error::Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DioxusConfig {
    pub application: ApplicationConfig,

    pub web: WebConfig,

    #[serde(default)]
    pub bundle: BundleConfig,

    #[serde(default = "default_plugin")]
    pub plugin: toml::Value,
}

fn default_plugin() -> toml::Value {
    toml::Value::Boolean(true)
}

impl DioxusConfig {
    pub fn load(bin: Option<PathBuf>) -> crate::error::Result<Option<DioxusConfig>> {
        let crate_dir = crate::cargo::crate_root();

        let crate_dir = match crate_dir {
            Ok(dir) => {
                if let Some(bin) = bin {
                    dir.join(bin)
                } else {
                    dir
                }
            }
            Err(_) => return Ok(None),
        };
        let crate_dir = crate_dir.as_path();

        let Some(dioxus_conf_file) = acquire_dioxus_toml(crate_dir) else {
            return Ok(None);
        };

        let dioxus_conf_file = dioxus_conf_file.as_path();
        let cfg = toml::from_str::<DioxusConfig>(&std::fs::read_to_string(dioxus_conf_file)?)
            .map_err(|err| {
                let error_location = dioxus_conf_file
                    .strip_prefix(crate_dir)
                    .unwrap_or(dioxus_conf_file)
                    .display();
                crate::Error::Unique(format!("{error_location} {err}"))
            })
            .map(Some);
        match cfg {
            Ok(Some(mut cfg)) => {
                let name = cfg.application.name.clone();
                if cfg.bundle.identifier.is_none() {
                    cfg.bundle.identifier = Some(format!("io.github.{name}"));
                }
                if cfg.bundle.publisher.is_none() {
                    cfg.bundle.publisher = Some(name);
                }
                Ok(Some(cfg))
            }
            cfg => cfg,
        }
    }
}

fn acquire_dioxus_toml(dir: &Path) -> Option<PathBuf> {
    // prefer uppercase
    let uppercase_conf = dir.join("Dioxus.toml");
    if uppercase_conf.is_file() {
        return Some(uppercase_conf);
    }

    // lowercase is fine too
    let lowercase_conf = dir.join("dioxus.toml");
    if lowercase_conf.is_file() {
        return Some(lowercase_conf);
    }

    None
}

impl Default for DioxusConfig {
    fn default() -> Self {
        let name = "name";
        Self {
            application: ApplicationConfig {
                name: name.into(),
                default_platform: Platform::Web,
                out_dir: Some(PathBuf::from("dist")),
                asset_dir: Some(PathBuf::from("public")),

                tools: None,

                sub_package: None,
            },
            web: WebConfig {
                app: WebAppConfig {
                    title: Some("dioxus | ⛺".into()),
                    base_path: None,
                },
                proxy: Some(vec![]),
                watcher: WebWatcherConfig {
                    watch_path: Some(vec![PathBuf::from("src"), PathBuf::from("examples")]),
                    reload_html: Some(false),
                    index_on_404: Some(true),
                },
                resource: WebResourceConfig {
                    dev: WebDevResourceConfig {
                        style: Some(vec![]),
                        script: Some(vec![]),
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
            },
            bundle: BundleConfig {
                identifier: Some(format!("io.github.{name}")),
                publisher: Some(name.into()),
                ..Default::default()
            },
            plugin: toml::Value::Table(toml::map::Map::new()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    pub name: String,
    pub default_platform: Platform,
    pub out_dir: Option<PathBuf>,
    pub asset_dir: Option<PathBuf>,

    pub tools: Option<HashMap<String, toml::Value>>,

    pub sub_package: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    pub app: WebAppConfig,
    pub proxy: Option<Vec<WebProxyConfig>>,
    pub watcher: WebWatcherConfig,
    pub resource: WebResourceConfig,
    #[serde(default)]
    pub https: WebHttpsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAppConfig {
    pub title: Option<String>,
    pub base_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebProxyConfig {
    pub backend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebWatcherConfig {
    pub watch_path: Option<Vec<PathBuf>>,
    pub reload_html: Option<bool>,
    pub index_on_404: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebResourceConfig {
    pub dev: WebDevResourceConfig,
    pub style: Option<Vec<PathBuf>>,
    pub script: Option<Vec<PathBuf>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDevResourceConfig {
    pub style: Option<Vec<PathBuf>>,
    pub script: Option<Vec<PathBuf>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WebHttpsConfig {
    pub enabled: Option<bool>,
    pub mkcert: Option<bool>,
    pub key_path: Option<String>,
    pub cert_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CrateConfig {
    pub out_dir: PathBuf,
    pub crate_dir: PathBuf,
    pub workspace_dir: PathBuf,
    pub target_dir: PathBuf,
    pub asset_dir: PathBuf,
    pub manifest: cargo_toml::Manifest<cargo_toml::Value>,
    pub executable: ExecutableType,
    pub dioxus_config: DioxusConfig,
    pub release: bool,
    pub hot_reload: bool,
    pub cross_origin_policy: bool,
    pub verbose: bool,
    pub custom_profile: Option<String>,
    pub features: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub enum ExecutableType {
    Binary(String),
    Lib(String),
    Example(String),
}

impl CrateConfig {
    pub fn new(bin: Option<PathBuf>) -> Result<Self> {
        let dioxus_config = DioxusConfig::load(bin.clone())?.unwrap_or_default();

        let crate_root = crate::cargo::crate_root()?;

        let crate_dir = if let Some(package) = &dioxus_config.application.sub_package {
            crate_root.join(package)
        } else if let Some(bin) = bin {
            crate_root.join(bin)
        } else {
            crate_root
        };

        let meta = crate::cargo::Metadata::get()?;
        let workspace_dir = meta.workspace_root;
        let target_dir = meta.target_directory;

        let out_dir = match dioxus_config.application.out_dir {
            Some(ref v) => crate_dir.join(v),
            None => crate_dir.join("dist"),
        };

        let cargo_def = &crate_dir.join("Cargo.toml");

        let asset_dir = match dioxus_config.application.asset_dir {
            Some(ref v) => crate_dir.join(v),
            None => crate_dir.join("public"),
        };

        let manifest = cargo_toml::Manifest::from_path(cargo_def).unwrap();

        let mut output_filename = String::from("dioxus_app");
        if let Some(package) = &manifest.package.as_ref() {
            output_filename = match &package.default_run {
                Some(default_run_target) => default_run_target.to_owned(),
                None => manifest
                    .bin
                    .iter()
                    .find(|b| b.name == manifest.package.as_ref().map(|pkg| pkg.name.clone()))
                    .or(manifest
                        .bin
                        .iter()
                        .find(|b| b.path == Some("src/main.rs".to_owned())))
                    .or(manifest.bin.first())
                    .or(manifest.lib.as_ref())
                    .and_then(|prod| prod.name.clone())
                    .unwrap_or(String::from("dioxus_app")),
            };
        }

        let executable = ExecutableType::Binary(output_filename);

        let release = false;
        let hot_reload = false;
        let verbose = false;
        let custom_profile = None;
        let features = None;

        Ok(Self {
            out_dir,
            crate_dir,
            workspace_dir,
            target_dir,
            asset_dir,
            manifest,
            executable,
            release,
            dioxus_config,
            hot_reload,
            cross_origin_policy: false,
            custom_profile,
            features,
            verbose,
        })
    }

    pub fn as_example(&mut self, example_name: String) -> &mut Self {
        self.executable = ExecutableType::Example(example_name);
        self
    }

    pub fn with_release(&mut self, release: bool) -> &mut Self {
        self.release = release;
        self
    }

    pub fn with_hot_reload(&mut self, hot_reload: bool) -> &mut Self {
        self.hot_reload = hot_reload;
        self
    }

    pub fn with_cross_origin_policy(&mut self, cross_origin_policy: bool) -> &mut Self {
        self.cross_origin_policy = cross_origin_policy;
        self
    }

    pub fn with_verbose(&mut self, verbose: bool) -> &mut Self {
        self.verbose = verbose;
        self
    }

    pub fn set_profile(&mut self, profile: String) -> &mut Self {
        self.custom_profile = Some(profile);
        self
    }

    pub fn set_features(&mut self, features: Vec<String>) -> &mut Self {
        self.features = Some(features);
        self
    }
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
