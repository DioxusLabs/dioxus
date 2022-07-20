use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, io::Read, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DioxusConfig {
    pub application: ApplicationConfig,
    pub web: WebConfig,
}

impl DioxusConfig {
    pub fn load() -> crate::error::Result<DioxusConfig> {
        let crate_dir = crate::cargo::crate_root()?;

        if !crate_dir.join("Dioxus.toml").is_file() {
            log::warn!("Config file: `Dioxus.toml` not found; using default config.");
            return Ok(DioxusConfig::default());
        }

        let mut dioxus_conf_file = File::open(crate_dir.join("Dioxus.toml"))?;
        let mut meta_str = String::new();
        dioxus_conf_file.read_to_string(&mut meta_str)?;

        toml::from_str::<DioxusConfig>(&meta_str)
            .map_err(|_| crate::Error::Unique("Dioxus.toml parse failed".into()))
    }
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
            },
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
    pub watcher: WebWatcherConfig,
    pub resource: WebResourceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAppConfig {
    pub title: Option<String>,
    pub base_path: Option<String>,
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
        let dioxus_config = DioxusConfig::load()?;

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

        let manifest = cargo_toml::Manifest::from_path(&cargo_def).unwrap();

        // We just assume they're using a 'main.rs'
        // Anyway, we've already parsed the manifest, so it should be easy to change the type
        let output_filename = manifest
            .bin
            .first()
            .or(manifest.lib.as_ref())
            .and_then(|product| product.name.clone())
            .or_else(|| manifest.package.as_ref().map(|pkg| pkg.name.clone()))
            .expect("No lib found from cargo metadata");
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

    // pub fn with_build_options(&mut self, options: &BuildOptions) {
    //     if let Some(name) = &options.example {
    //         self.as_example(name.clone());
    //     }
    //     self.release = options.release;
    //     self.out_dir = options.outdir.clone().into();
    // }

    // pub fn with_develop_options(&mut self, options: &DevelopOptions) {
    //     if let Some(name) = &options.example {
    //         self.as_example(name.clone());
    //     }
    //     self.release = options.release;
    //     self.out_dir = tempfile::Builder::new().tempdir().expect("").into_path();
    // }
}
