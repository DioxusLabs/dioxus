use dioxus_cli_config::{DioxusConfig, Platform};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    path::PathBuf,
};

use crate::metadata::{crate_root, CargoError, Metadata};

// Contains information about the crate we are currently in and the dioxus config for that crate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DioxusCrate {
    pub crate_dir: PathBuf,
    pub workspace_dir: PathBuf,
    pub target_dir: PathBuf,
    pub manifest: cargo_toml::Manifest<cargo_toml::Value>,
    pub executable: ExecutableType,
    pub dioxus_config: DioxusConfig,
}

impl DioxusCrate {
    pub fn new(bin: Option<PathBuf>) -> Result<Self, CrateConfigError> {
        let dioxus_config = load_dioxus_config(bin.clone())?.unwrap_or_default();

        let crate_root = crate_root()?;

        let crate_dir = if let Some(package) = &dioxus_config.application.sub_package {
            crate_root.join(package)
        } else if let Some(bin) = bin {
            crate_root.join(bin)
        } else {
            crate_root
        };

        let meta = Metadata::get()?;
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

        Ok(Self {
            crate_dir,
            workspace_dir,
            target_dir,
            manifest,
            executable,
            dioxus_config,
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

    pub fn features_for_platform(&mut self, platform: Platform) -> Vec<String> {
        let manifest = &self.manifest;
        // Try to find the feature that activates the dioxus feature for the given platform
        let dioxus_feature = platform.feature_name();
        let feature = manifest.features.iter().find_map(|(key, features)| {
            // Find a key that starts with dioxus/ or dioxus?/
            if let Some((_, after_dioxus)) = key.split_once("dioxus") {
                if let Some(dioxus_feature_enabled) =
                    after_dioxus.trim_start_matches("?").strip_prefix("/")
                {
                    // If that enables the feature we are looking for, return that feature
                    if dioxus_feature_enabled == dioxus_feature {
                        return Some(key.clone());
                    }
                }
            }
            None
        });

        feature.into_iter().collect()
    }

    /// Check if assets should be pre_compressed. This will only be true in release mode if the user has enabled pre_compress in the web config.
    pub fn should_pre_compress_web_assets(&self, release: bool) -> bool {
        self.dioxus_config.web.pre_compress && release
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutableType {
    Binary(String),
    Lib(String),
    Example(String),
}

impl ExecutableType {
    /// Get the name of the executable if it is a binary or an example.
    pub fn executable(&self) -> Option<&str> {
        match self {
            Self::Binary(bin) | Self::Example(bin) => Some(bin),
            _ => None,
        }
    }
}

/// Load the dioxus config from a path
#[tracing::instrument]
fn load_dioxus_config(bin: Option<PathBuf>) -> Result<Option<DioxusConfig>, CrateConfigError> {
    #[tracing::instrument]
    fn acquire_dioxus_toml(dir: &std::path::Path) -> Option<PathBuf> {
        use tracing::trace;

        ["Dioxus.toml", "dioxus.toml"]
            .into_iter()
            .map(|file| dir.join(file))
            .inspect(|path| trace!("checking [{path:?}]"))
            .find(|path| path.is_file())
    }

    let crate_dir = crate_root();

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
        tracing::warn!(?crate_dir, "no dioxus config found for");
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
#[non_exhaustive]
pub enum CrateConfigError {
    Cargo(CargoError),
    Io(std::io::Error),
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
            Self::Toml(err) => write!(f, "{}", err),
            Self::LoadDioxusConfig(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for CrateConfigError {}
