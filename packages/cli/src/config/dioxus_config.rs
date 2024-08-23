use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DioxusConfig {
    pub application: ApplicationConfig,

    #[serde(default)]
    pub web: WebConfig,

    #[serde(default)]
    pub desktop: DesktopConfig,

    #[serde(default)]
    pub bundle: BundleConfig,
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
                pre_compress: true,
                wasm_opt: Default::default(),
            },
            desktop: DesktopConfig::default(),
            bundle: BundleConfig {
                identifier: Some(format!("io.github.{name}")),
                publisher: Some(name),
                ..Default::default()
            },
        }
    }
}
