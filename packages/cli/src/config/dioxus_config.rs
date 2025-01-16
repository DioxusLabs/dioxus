use super::*;
use crate::Result;
use anyhow::Context;
use krates::{Krates, NodeId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DioxusConfig {
    pub(crate) application: ApplicationConfig,

    #[serde(default)]
    pub(crate) web: WebConfig,

    #[serde(default)]
    pub(crate) desktop: DesktopConfig,

    #[serde(default)]
    pub(crate) bundle: BundleConfig,
}

impl Default for DioxusConfig {
    fn default() -> Self {
        Self {
            application: ApplicationConfig {
                asset_dir: None,
                sub_package: None,
                out_dir: None,
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
                pre_compress: true,
                wasm_opt: Default::default(),
            },
            desktop: DesktopConfig::default(),
            bundle: BundleConfig::default(),
        }
    }
}

impl DioxusConfig {
    pub fn load(krates: &Krates, package: NodeId) -> Result<Option<Self>> {
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
