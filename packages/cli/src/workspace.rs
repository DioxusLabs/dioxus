use crate::config::DioxusConfig;
use crate::CliSettings;
use crate::{Platform, Result};
use anyhow::Context;
use itertools::Itertools;
use krates::{cm::Target, KrateDetails};
use krates::{cm::TargetKind, Cmd, Krates, NodeId};
use once_cell::sync::OnceCell;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use target_lexicon::Triple;
use tokio::process::Command;
use toml_edit::Item;

pub struct Workspace {
    pub(crate) krates: Krates,
    pub(crate) settings: CliSettings,
    pub(crate) wasm_opt: Option<PathBuf>,
    pub(crate) sysroot: PathBuf,
    pub(crate) rustc_version: String,
}

static WS: OnceCell<Arc<Workspace>> = OnceCell::new();

impl Workspace {
    pub async fn current() -> Result<Arc<Workspace>> {
        tracing::debug!("Loading workspace");
        let cmd = Cmd::new();
        let builder = krates::Builder::new();
        let krates = builder
            .build(cmd, |_| {})
            .context("Failed to run cargo metadata")?;

        let settings = CliSettings::global_or_default();
        let sysroot = Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
            .await
            .map(|out| String::from_utf8(out.stdout))?
            .context("Failed to extract rustc sysroot output")?;

        let rustc_version = Command::new("rustc")
            .args(["--version"])
            .output()
            .await
            .map(|out| String::from_utf8(out.stdout))?
            .context("Failed to extract rustc version output")?;

        let wasm_opt = which::which("wasm-opt").ok();

        Ok(Arc::new(Self {
            krates,
            settings,
            wasm_opt,
            sysroot: sysroot.trim().into(),
            rustc_version: rustc_version.trim().into(),
        }))
    }

    pub fn wasm_ld(&self) -> PathBuf {
        self.sysroot
            .join("lib")
            .join("rustlib")
            .join(Triple::host().to_string())
            .join("bin")
            .join("gcc-ld")
            .join("wasm-ld")
    }

    pub fn has_wasm32_unknown_unknown(&self) -> bool {
        self.sysroot
            .join("lib/rustlib/wasm32-unknown-unknown")
            .exists()
    }

    /// Find the main package in the workspace
    pub fn find_main_package(&self, package: Option<String>) -> Result<NodeId> {
        if let Some(package) = package {
            let mut workspace_members = self.krates.workspace_members();
            let found = workspace_members.find_map(|node| {
                if let krates::Node::Krate { id, krate, .. } = node {
                    if krate.name == package {
                        return Some(id);
                    }
                }
                None
            });

            if found.is_none() {
                tracing::error!("Could not find package {package} in the workspace. Did you forget to add it to the workspace?");
                tracing::error!("Packages in the workspace:");
                for package in self.krates.workspace_members() {
                    if let krates::Node::Krate { krate, .. } = package {
                        tracing::error!("{}", krate.name());
                    }
                }
            }

            let kid = found.ok_or_else(|| anyhow::anyhow!("Failed to find package {package}"))?;

            return Ok(self.krates.nid_for_kid(kid).unwrap());
        };

        // Otherwise find the package that is the closest parent of the current directory
        let current_dir = std::env::current_dir()?;
        let current_dir = current_dir.as_path();

        // Go through each member and find the path that is a parent of the current directory
        let mut closest_parent = None;
        for member in self.krates.workspace_members() {
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

        let kid = closest_parent
        .map(|(id, _)| id)
        .with_context(|| {
            let bin_targets = self.krates.workspace_members().filter_map(|krate|match krate {
                krates::Node::Krate { krate, .. } if krate.targets.iter().any(|t| t.kind.contains(&krates::cm::TargetKind::Bin))=> {
                    Some(format!("- {}", krate.name))
                }
                _ => None
            }).collect::<Vec<_>>();
            format!("Failed to find binary package to build.\nYou need to either run dx from inside a binary crate or specify a binary package to build with the `--package` flag. Try building again with one of the binary packages in the workspace:\n{}", bin_targets.join("\n"))
        })?;

        let package = self.krates.nid_for_kid(kid).unwrap();
        Ok(package)
    }

    pub fn load_dioxus_config(&self, package: NodeId) -> Result<Option<DioxusConfig>> {
        // Walk up from the cargo.toml to the root of the workspace looking for Dioxus.toml
        let mut current_dir = self.krates[package]
            .manifest_path
            .parent()
            .unwrap()
            .as_std_path()
            .to_path_buf()
            .canonicalize()?;

        let workspace_path = self
            .krates
            .workspace_root()
            .as_std_path()
            .to_path_buf()
            .canonicalize()?;

        let mut dioxus_conf_file = None;
        while current_dir.starts_with(&workspace_path) {
            let config = ["Dioxus.toml", "dioxus.toml"]
                .into_iter()
                .map(|file| current_dir.join(file))
                .find(|path| path.is_file());

            // Try to find Dioxus.toml in the current directory
            if let Some(new_config) = config {
                dioxus_conf_file = Some(new_config.as_path().to_path_buf());
                break;
            }
            // If we can't find it, go up a directory
            current_dir = current_dir
                .parent()
                .context("Failed to find Dioxus.toml")?
                .to_path_buf();
        }

        let Some(dioxus_conf_file) = dioxus_conf_file else {
            return Ok(None);
        };

        toml::from_str::<DioxusConfig>(&std::fs::read_to_string(&dioxus_conf_file)?)
            .map_err(|err| {
                anyhow::anyhow!("Failed to parse Dioxus.toml at {dioxus_conf_file:?}: {err}").into()
            })
            .map(Some)
    }
}

impl std::fmt::Debug for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Workspace")
            .field("krates", &"..")
            .field("settings", &self.settings)
            .field("rustc_version", &self.rustc_version)
            .field("sysroot", &self.sysroot)
            .field("wasm_opt", &self.wasm_opt)
            .finish()
    }
}
