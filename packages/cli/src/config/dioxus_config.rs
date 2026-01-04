use crate::config::component::ComponentConfig;

use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DioxusConfig {
    pub(crate) application: ApplicationConfig,

    #[serde(default)]
    pub(crate) web: WebConfig,

    #[serde(default)]
    pub(crate) bundle: BundleConfig,

    #[serde(default)]
    pub(crate) components: ComponentConfig,

    /// Unified permissions configuration.
    /// Permissions declared here are automatically mapped to platform-specific
    /// identifiers (AndroidManifest.xml, Info.plist, etc.)
    #[serde(default)]
    pub(crate) permissions: PermissionsConfig,

    /// Unified deep linking configuration.
    /// URL schemes and universal links declared here are mapped to platform-specific
    /// configurations. Use [ios], [android], [macos] sections for overrides.
    #[serde(default)]
    pub(crate) deep_links: DeepLinkConfig,

    /// Unified background mode configuration.
    /// Background capabilities declared here are mapped to platform-specific
    /// configurations. Use [ios], [android] sections for overrides.
    #[serde(default)]
    pub(crate) background: BackgroundConfig,

    /// iOS-specific configuration.
    #[serde(default)]
    pub(crate) ios: IosConfig,

    /// Android-specific configuration.
    #[serde(default)]
    pub(crate) android: AndroidConfig,

    /// macOS-specific configuration.
    #[serde(default)]
    pub(crate) macos: MacosConfig,

    /// Windows-specific configuration.
    #[serde(default)]
    pub(crate) windows: WindowsConfig,

    /// Linux-specific configuration.
    #[serde(default)]
    pub(crate) linux: LinuxConfig,
}

impl Default for DioxusConfig {
    fn default() -> Self {
        Self {
            application: ApplicationConfig {
                asset_dir: None,
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
            components: ComponentConfig::default(),
            permissions: PermissionsConfig::default(),
            deep_links: DeepLinkConfig::default(),
            background: BackgroundConfig::default(),
            ios: IosConfig::default(),
            android: AndroidConfig::default(),
            macos: MacosConfig::default(),
            windows: WindowsConfig::default(),
            linux: LinuxConfig::default(),
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
            public_dir = "public2"
        "#;

        let config: DioxusConfig = toml::from_str(source).expect("parse config");
        assert_eq!(
            config.application.public_dir.as_deref(),
            Some(std::path::Path::new("public2"))
        );
    }

    #[test]
    fn static_dir_can_be_disabled() {
        let source = r#"
            [application]
            public_dir = ""
        "#;

        let config: DioxusConfig = toml::from_str(source).expect("parse config");
        assert_eq!(config.application.public_dir.as_deref(), None);
    }
}
