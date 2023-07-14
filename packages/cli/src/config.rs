use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DioxusConfig {
    pub application: ApplicationConfig,

    pub web: WebConfig,

    #[serde(default = "default_plugin")]
    pub plugin: toml::Value,
}

fn default_plugin() -> toml::Value {
    toml::Value::Boolean(true)
}

impl DioxusConfig {
    pub fn load() -> crate::error::Result<Option<DioxusConfig>> {
        let Ok(crate_dir) = crate::cargo::crate_root() else { return Ok(None); };

        // we support either `Dioxus.toml` or `Cargo.toml`
        let Some(dioxus_conf_file) = acquire_dioxus_toml(crate_dir) else {
            return Ok(None);
        };

        toml::from_str::<DioxusConfig>(&std::fs::read_to_string(dioxus_conf_file)?)
            .map_err(|_| crate::Error::Unique("Dioxus.toml parse failed".into()))
            .map(Some)
    }
}

fn acquire_dioxus_toml(dir: PathBuf) -> Option<PathBuf> {
    // prefer uppercase
    if dir.join("Dioxus.toml").is_file() {
        return Some(dir.join("Dioxus.toml"));
    }

    // lowercase is fine too
    if dir.join("dioxus.toml").is_file() {
        return Some(dir.join("Dioxus.toml"));
    }

    None
}

impl Default for DioxusConfig {
    fn default() -> Self {
        Self {
            application: ApplicationConfig {
                name: "dioxus".into(),
                default_platform: "web".to_string(),
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
                    watch_path: Some(vec![PathBuf::from("src")]),
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
            plugin: toml::Value::Table(toml::map::Map::new()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    pub name: String,
    pub default_platform: String,
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
    pub fn new() -> Result<Self> {
        let dioxus_config = DioxusConfig::load()?.unwrap_or_default();

        let crate_dir = if let Some(package) = &dioxus_config.application.sub_package {
            crate::cargo::crate_root()?.join(package)
        } else {
            crate::cargo::crate_root()?
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

        let output_filename = {
            match &manifest.package.as_ref().unwrap().default_run {
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
                    .expect("No executable or library found from cargo metadata."),
            }
        };
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
