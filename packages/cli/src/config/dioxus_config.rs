use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DioxusConfig {
    pub(crate) application: ApplicationConfig,

    #[serde(default)]
    pub(crate) web: WebConfig,

    #[serde(default)]
    pub(crate) bundle: BundleConfig,
}

impl Default for DioxusConfig {
    fn default() -> Self {
        Self {
            application: ApplicationConfig {
                out_dir: None,
                public_dir: Some("public".into()),
                tailwind_input: None,
                tailwind_output: None,
                ios_info_plist: None,
                android_manifest: None,
                android_main_activity: None,
                android_min_sdk_version: None,
                macos_info_plist: None,
                ios_entitlements: None,
                macos_entitlements: None,
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
                pre_compress: false,
                wasm_opt: Default::default(),
            },
            bundle: BundleConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_dir_defaults_to_public() {
        let config = DioxusConfig::default();
        assert_eq!(
            config.application.public_dir,
            Some(std::path::PathBuf::from("public"))
        );
    }

    #[test]
    fn static_dir_can_be_overridden() {
        let source = r#"
            [application]
            static_dir = "static"
        "#;

        let config: DioxusConfig = toml::from_str(source).expect("parse config");
        assert_eq!(
            config.application.public_dir.as_deref(),
            Some(std::path::Path::new("static"))
        );
    }
}
