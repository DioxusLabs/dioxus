use crate::{
    build::TargetArgs,
    config::{DioxusConfig, Platform},
};
use krates::cm::Target;
use krates::{cm::TargetKind, Cmd, Krates, NodeId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{
    fmt::{Display, Formatter},
    path::PathBuf,
};

use crate::metadata::CargoError;

/// Load the dioxus config from a path
fn load_dioxus_config(
    krates: &Krates,
    package: NodeId,
) -> Result<Option<DioxusConfig>, CrateConfigError> {
    fn acquire_dioxus_toml(dir: &std::path::Path) -> Option<PathBuf> {
        ["Dioxus.toml", "dioxus.toml"]
            .into_iter()
            .map(|file| dir.join(file))
            .find(|path| path.is_file())
    }

    // Walk up from the cargo.toml to the root of the workspace looking for Dioxus.toml
    let mut current_dir = krates[package]
        .manifest_path
        .parent()
        .unwrap()
        .as_std_path()
        .to_path_buf()
        .canonicalize()?;

    let workspace_path = krates
        .workspace_root()
        .as_std_path()
        .to_path_buf()
        .canonicalize()?;

    let mut dioxus_conf_file = None;
    while current_dir.starts_with(&workspace_path) {
        // Try to find Dioxus.toml in the current directory
        if let Some(new_config) = acquire_dioxus_toml(&current_dir) {
            dioxus_conf_file = Some(new_config.as_path().to_path_buf());
            break;
        }
        // If we can't find it, go up a directory
        current_dir = current_dir
            .parent()
            .ok_or(CrateConfigError::CurrentPackageNotFound)?
            .to_path_buf();
    }

    let Some(dioxus_conf_file) = dioxus_conf_file else {
        return Ok(None);
    };

    let cfg = toml::from_str::<DioxusConfig>(&std::fs::read_to_string(&dioxus_conf_file)?)
        .map_err(|err| {
            CrateConfigError::LoadDioxusConfig(LoadDioxusConfigError {
                location: dioxus_conf_file.display().to_string(),
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

// Find the main package in the workspace
fn find_main_package(package: Option<String>, krates: &Krates) -> Result<NodeId, CrateConfigError> {
    let kid = match package {
        Some(package) => {
            let mut workspace_members = krates.workspace_members();
            workspace_members
                .find_map(|node| {
                    if let krates::Node::Krate { id, krate, .. } = node {
                        if krate.name == package {
                            return Some(id);
                        }
                    }
                    None
                })
                .ok_or_else(|| CrateConfigError::PackageNotFound(package.clone()))?
        }
        None => {
            // Otherwise find the package that is the closest parent of the current directory
            let current_dir = std::env::current_dir()?;
            let current_dir = current_dir.as_path();
            // Go through each member and find the path that is a parent of the current directory
            let mut closest_parent = None;
            for member in krates.workspace_members() {
                if let krates::Node::Krate { id, krate, .. } = member {
                    let member_path = krate.manifest_path.parent().unwrap();
                    if let Ok(path) = current_dir.strip_prefix(member_path.as_std_path()) {
                        let len = path.components().count();
                        match closest_parent {
                            Some((_, closest_parent_len)) => {
                                if len < closest_parent_len {
                                    closest_parent = Some((id, len));
                                }
                            }
                            None => {
                                closest_parent = Some((id, len));
                            }
                        }
                    }
                }
            }
            closest_parent
                .map(|(id, _)| id)
                .ok_or(CrateConfigError::CurrentPackageNotFound)?
        }
    };

    let package = krates.nid_for_kid(kid).unwrap();
    Ok(package)
}

// Contains information about the crate we are currently in and the dioxus config for that crate
#[derive(Clone)]
pub struct DioxusCrate {
    pub krates: Arc<Krates>,
    pub package: NodeId,
    pub dioxus_config: DioxusConfig,
    pub target: Target,
}

impl DioxusCrate {
    pub fn new(target: &TargetArgs) -> Result<Self, CrateConfigError> {
        let mut cmd = Cmd::new();
        cmd.features(target.features.clone());
        let builder = krates::Builder::new();
        let krates = builder.build(cmd, |_| {})?;
        let package = find_main_package(target.package.clone(), &krates)?;

        let dioxus_config = load_dioxus_config(&krates, package)?.unwrap_or_default();

        let package_name = krates[package].name.clone();
        let target_kind = if target.example.is_some() {
            TargetKind::Example
        } else {
            TargetKind::Bin
        };
        let target_name = target
            .example
            .clone()
            .or(target.bin.clone())
            .unwrap_or(package_name);
        let main_package = &krates[package];
        let target = main_package
            .targets
            .iter()
            .find(|target| {
                target_name == target.name.as_str() && target.kind.contains(&target_kind)
            })
            .ok_or(CrateConfigError::TargetNotFound(target_name))?
            .clone();

        Ok(Self {
            krates: Arc::new(krates),
            package,
            dioxus_config,
            target,
        })
    }

    /// Compose an asset directory. Represents the typical "public" directory
    /// with publicly available resources (configurable in the `Dioxus.toml`).
    pub fn asset_dir(&self) -> PathBuf {
        self.crate_dir()
            .join(&self.dioxus_config.application.asset_dir)
    }

    /// Compose an out directory. Represents the typical "dist" directory that
    /// is "distributed" after building an application (configurable in the
    /// `Dioxus.toml`).
    pub fn out_dir(&self) -> PathBuf {
        self.workspace_dir()
            .join(&self.dioxus_config.application.out_dir)
    }

    /// Get the workspace directory for the crate
    pub fn workspace_dir(&self) -> PathBuf {
        self.krates.workspace_root().as_std_path().to_path_buf()
    }

    /// Get the directory of the crate
    pub fn crate_dir(&self) -> PathBuf {
        self.package()
            .manifest_path
            .parent()
            .unwrap()
            .as_std_path()
            .to_path_buf()
    }

    /// Get the main source file of the target
    pub fn main_source_file(&self) -> PathBuf {
        self.target.src_path.as_std_path().to_path_buf()
    }

    /// Get the package we are currently in
    pub fn package(&self) -> &krates::cm::Package {
        &self.krates[self.package]
    }

    /// Get the name of the package we are compiling
    pub fn executable_name(&self) -> &str {
        &self.target.name
    }

    /// Get the type of executable we are compiling
    pub fn executable_type(&self) -> krates::cm::TargetKind {
        self.target.kind[0].clone()
    }

    pub fn features_for_platform(&mut self, platform: Platform) -> Vec<String> {
        let package = self.package();
        // Try to find the feature that activates the dioxus feature for the given platform
        let dioxus_feature = platform.feature_name();
        let feature = package.features.iter().find_map(|(key, features)| {
            // Find a feature that starts with dioxus/ or dioxus?/
            for feature in features {
                if let Some((_, after_dioxus)) = feature.split_once("dioxus") {
                    if let Some(dioxus_feature_enabled) =
                        after_dioxus.trim_start_matches('?').strip_prefix('/')
                    {
                        // If that enables the feature we are looking for, return that feature
                        if dioxus_feature_enabled == dioxus_feature {
                            return Some(key.clone());
                        }
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
pub struct Executable {
    pub name: String,
    pub ty: ExecutableType,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum ExecutableType {
    Binary,
    Lib,
    Example,
}

impl ExecutableType {
    /// Get the name of the executable if it is a binary or an example.
    pub fn executable(&self) -> bool {
        matches!(self, Self::Binary | Self::Example)
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
    TargetNotFound(String),
    Krates(krates::Error),
    PackageNotFound(String),
    CurrentPackageNotFound,
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

impl From<krates::Error> for CrateConfigError {
    fn from(err: krates::Error) -> Self {
        Self::Krates(err)
    }
}

impl Display for CrateConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cargo(err) => write!(f, "{}", err),
            Self::Io(err) => write!(f, "{}", err),
            Self::Toml(err) => write!(f, "{}", err),
            Self::LoadDioxusConfig(err) => write!(f, "{}", err),
            Self::TargetNotFound(target) => {
                write!(f, "Failed to find target with name: {}", target)
            }
            Self::Krates(err) => write!(f, "{}", err),
            Self::PackageNotFound(package) => write!(f, "Package not found: {}", package),
            Self::CurrentPackageNotFound => write!(f, "Failed to find current package"),
        }
    }
}

impl std::error::Error for CrateConfigError {}
