use crate::BundleConfig;
use crate::CargoError;
use core::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum Platform {
    #[cfg_attr(feature = "cli", clap(name = "web"))]
    #[serde(rename = "web")]
    Web,
    #[cfg_attr(feature = "cli", clap(name = "desktop"))]
    #[serde(rename = "desktop")]
    Desktop,
    #[cfg_attr(feature = "cli", clap(name = "fullstack"))]
    #[serde(rename = "fullstack")]
    Fullstack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DioxusConfig {
    pub application: ApplicationConfig,

    pub web: WebConfig,

    #[serde(default)]
    pub bundle: BundleConfig,

    #[cfg(feature = "cli")]
    #[serde(default = "default_plugin")]
    pub plugin: toml::Value,
}

#[cfg(feature = "cli")]
fn default_plugin() -> toml::Value {
    toml::Value::Boolean(true)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadDioxusConfigError {
    location: String,
    error: String,
}

impl std::fmt::Display for LoadDioxusConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.location, self.error)
    }
}

impl std::error::Error for LoadDioxusConfigError {}

#[derive(Debug)]
pub enum CrateConfigError {
    Cargo(CargoError),
    Io(std::io::Error),
    #[cfg(feature = "cli")]
    Toml(toml::de::Error),
    LoadDioxusConfig(LoadDioxusConfigError),
}

impl From<CargoError> for CrateConfigError {
    fn from(err: CargoError) -> Self {
        Self::Cargo(err)
    }
}

impl From<std::io::Error> for CrateConfigError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

#[cfg(feature = "cli")]
impl From<toml::de::Error> for CrateConfigError {
    fn from(err: toml::de::Error) -> Self {
        Self::Toml(err)
    }
}

impl From<LoadDioxusConfigError> for CrateConfigError {
    fn from(err: LoadDioxusConfigError) -> Self {
        Self::LoadDioxusConfig(err)
    }
}

impl Display for CrateConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cargo(err) => write!(f, "{}", err),
            Self::Io(err) => write!(f, "{}", err),
            #[cfg(feature = "cli")]
            Self::Toml(err) => write!(f, "{}", err),
            Self::LoadDioxusConfig(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for CrateConfigError {}

impl DioxusConfig {
    #[cfg(feature = "cli")]
    /// Load the dioxus config from a path
    pub fn load(bin: Option<PathBuf>) -> Result<Option<DioxusConfig>, CrateConfigError> {
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
                CrateConfigError::LoadDioxusConfig(LoadDioxusConfigError {
                    location: error_location.to_string(),
                    error: err.to_string(),
                })
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

#[cfg(feature = "cli")]
fn acquire_dioxus_toml(dir: &std::path::Path) -> Option<PathBuf> {
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
        let name = default_name();
        Self {
            application: ApplicationConfig {
                name: name.clone(),
                default_platform: default_platform(),
                out_dir: out_dir_default(),
                asset_dir: asset_dir_default(),

                #[cfg(feature = "cli")]
                tools: Default::default(),

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
            },
            bundle: BundleConfig {
                identifier: Some(format!("io.github.{name}")),
                publisher: Some(name),
                ..Default::default()
            },
            #[cfg(feature = "cli")]
            plugin: toml::Value::Table(toml::map::Map::new()),
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

    #[cfg(feature = "cli")]
    #[serde(default)]
    pub tools: std::collections::HashMap<String, toml::Value>,

    #[serde(default)]
    pub sub_package: Option<String>,
}

fn default_name() -> String {
    "name".into()
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAppConfig {
    #[serde(default = "default_title")]
    pub title: String,
    pub base_path: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateConfig {
    pub crate_dir: PathBuf,
    pub workspace_dir: PathBuf,
    pub target_dir: PathBuf,
    #[cfg(feature = "cli")]
    pub manifest: cargo_toml::Manifest<cargo_toml::Value>,
    pub executable: ExecutableType,
    pub dioxus_config: DioxusConfig,
    pub release: bool,
    pub hot_reload: bool,
    pub cross_origin_policy: bool,
    pub verbose: bool,
    pub custom_profile: Option<String>,
    pub features: Option<Vec<String>>,
    pub target: Option<String>,
    pub cargo_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutableType {
    Binary(String),
    Lib(String),
    Example(String),
}

impl CrateConfig {
    #[cfg(feature = "cli")]
    pub fn new(bin: Option<PathBuf>) -> Result<Self, CrateConfigError> {
        let dioxus_config = DioxusConfig::load(bin.clone())?.unwrap_or_default();

        let crate_root = crate::crate_root()?;

        let crate_dir = if let Some(package) = &dioxus_config.application.sub_package {
            crate_root.join(package)
        } else if let Some(bin) = bin {
            crate_root.join(bin)
        } else {
            crate_root
        };

        let meta = crate::Metadata::get()?;
        let workspace_dir = meta.workspace_root;
        let target_dir = meta.target_directory;

        let cargo_def = &crate_dir.join("Cargo.toml");

        let manifest = cargo_toml::Manifest::from_path(cargo_def).unwrap();

        let mut output_filename = String::from("dioxus_app");
        if let Some(package) = &manifest.package.as_ref() {
            output_filename = match &package.default_run {
                Some(default_run_target) => default_run_target.to_owned(),
                None => manifest
                    .bin
                    .iter()
                    .find(|b| {
                        #[allow(clippy::useless_asref)]
                        let matching_bin =
                            b.name == manifest.package.as_ref().map(|pkg| pkg.name.clone());
                        matching_bin
                    })
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
        let cross_origin_policy = false;
        let verbose = false;
        let custom_profile = None;
        let features = None;
        let target = None;
        let cargo_args = vec![];

        Ok(Self {
            crate_dir,
            workspace_dir,
            target_dir,
            #[cfg(feature = "cli")]
            manifest,
            executable,
            dioxus_config,
            release,
            hot_reload,
            cross_origin_policy,
            verbose,
            custom_profile,
            features,
            target,
            cargo_args,
        })
    }

    /// Compose an asset directory. Represents the typical "public" directory
    /// with publicly available resources (configurable in the `Dioxus.toml`).
    pub fn asset_dir(&self) -> PathBuf {
        self.crate_dir
            .join(&self.dioxus_config.application.asset_dir)
    }

    /// Compose an out directory. Represents the typical "dist" directory that
    /// is "distributed" after building an application (configurable in the
    /// `Dioxus.toml`).
    pub fn out_dir(&self) -> PathBuf {
        self.crate_dir.join(&self.dioxus_config.application.out_dir)
    }

    /// Compose an out directory for the fullstack platform. See `out_dir()`
    /// method.
    pub fn fullstack_out_dir(&self) -> PathBuf {
        self.crate_dir.join(".dioxus")
    }

    /// Compose a target directory for the server (fullstack-only?).
    pub fn server_target_dir(&self) -> PathBuf {
        self.fullstack_out_dir().join("ssr")
    }

    /// Compose a target directory for the client (fullstack-only?).
    pub fn client_target_dir(&self) -> PathBuf {
        self.fullstack_out_dir().join("web")
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

    pub fn set_target(&mut self, target: String) -> &mut Self {
        self.target = Some(target);
        self
    }

    pub fn set_cargo_args(&mut self, cargo_args: Vec<String>) -> &mut Self {
        self.cargo_args = cargo_args;
        self
    }
}

fn true_bool() -> bool {
    true
}
