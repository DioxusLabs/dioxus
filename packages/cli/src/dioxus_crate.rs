use dioxus_cli_config::{crate_root, CrateConfigError, DioxusConfig, Metadata, Platform};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
        let dioxus_config = DioxusConfig::load(bin.clone())?.unwrap_or_default();

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
