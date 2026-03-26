use crate::config::component::ComponentConfig;

use super::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct DioxusConfig {
    #[serde(default)]
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
    /// configurations. Use `[ios]`, `[android]`, `[macos]` sections for overrides.
    #[serde(default)]
    pub(crate) deep_links: DeepLinkConfig,

    /// Unified background mode configuration.
    /// Background capabilities declared here are mapped to platform-specific
    /// configurations. Use `[ios]`, `[android]` sections for overrides.
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

    /// Custom renderer configuration for projects that use `dioxus-core` with their own renderer.
    ///
    /// When present, this overrides the default renderer autodetection and feature injection.
    /// Existing Dioxus projects (without this section) are unaffected.
    ///
    /// ```toml
    /// [renderer]
    /// name = "my-renderer"
    /// default_platform = "desktop"
    ///
    /// [renderer.features]
    /// desktop = []
    /// web = ["my-web"]
    /// ios = ["my-mobile"]
    /// android = ["my-mobile"]
    /// ```
    #[serde(default)]
    pub(crate) renderer: RendererConfig,
}

/// Configuration for custom (non-dioxus) renderers.
///
/// Projects that use `dioxus-core` directly with their own renderer can use this section
/// to declare platform-to-feature mappings so `dx serve`, `dx build`, and `dx bundle` work
/// without pulling in dioxus's built-in renderers.
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub(crate) struct RendererConfig {
    /// Display name for the renderer (shown in TUI).
    #[serde(default)]
    pub(crate) name: Option<String>,

    /// Default platform when none is specified on the CLI.
    ///
    /// Must be one of: `"web"`, `"macos"`, `"windows"`, `"linux"`, `"ios"`, `"android"`,
    /// `"server"`, `"liveview"`.
    #[serde(default)]
    pub(crate) default_platform: Option<String>,

    /// Map from platform name to cargo features to enable.
    ///
    /// Keys are platform identifiers (e.g., `"desktop"`, `"web"`, `"ios"`).
    /// Values are lists of cargo feature names to pass via `--features`.
    /// An empty list means "build with default features, don't inject any extra".
    #[serde(default)]
    pub(crate) features: HashMap<String, Vec<String>>,
}

impl RendererConfig {
    /// Returns `true` if a custom renderer is configured.
    pub(crate) fn is_custom(&self) -> bool {
        self.name.is_some() || !self.features.is_empty()
    }

    /// Look up custom features for a platform, trying each key in order.
    ///
    /// This allows fallback chains like `["macos", "desktop"]` so platform-specific
    /// keys take priority over generic ones.
    pub(crate) fn features_for_platform(&self, keys: &[&str]) -> Option<Vec<String>> {
        for key in keys {
            if let Some(feats) = self.features.get(*key) {
                return Some(feats.clone());
            }
        }
        None
    }
}

/// Platform identifier for bundle resolution.
/// This is separate from the CLI's Platform enum which includes Server and Unknown variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundlePlatform {
    Ios,
    Android,
    MacOS,
    Windows,
    Linux,
    Web,
}

impl From<crate::BundleFormat> for BundlePlatform {
    fn from(format: crate::BundleFormat) -> Self {
        match format {
            crate::BundleFormat::Ios => BundlePlatform::Ios,
            crate::BundleFormat::Android => BundlePlatform::Android,
            crate::BundleFormat::MacOS => BundlePlatform::MacOS,
            crate::BundleFormat::Windows => BundlePlatform::Windows,
            crate::BundleFormat::Linux => BundlePlatform::Linux,
            crate::BundleFormat::Web | crate::BundleFormat::Server => BundlePlatform::Web,
        }
    }
}

impl DioxusConfig {
    /// Get the resolved bundle identifier for a specific platform.
    /// Platform-specific identifiers override the base bundle identifier.
    pub fn resolved_identifier(&self, platform: BundlePlatform) -> Option<&str> {
        let platform_override = match platform {
            BundlePlatform::Ios => self.ios.identifier.as_deref(),
            BundlePlatform::Android => self.android.identifier.as_deref(),
            BundlePlatform::MacOS => self.macos.identifier.as_deref(),
            BundlePlatform::Windows => self.windows.identifier.as_deref(),
            BundlePlatform::Linux => self.linux.identifier.as_deref(),
            BundlePlatform::Web => None,
        };
        platform_override.or(self.bundle.identifier.as_deref())
    }
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
            renderer: RendererConfig::default(),
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

    #[test]
    fn renderer_config_absent_is_not_custom() {
        let config = DioxusConfig::default();
        assert!(!config.renderer.is_custom());
        assert!(config.renderer.features.is_empty());
    }

    #[test]
    fn renderer_config_parses_from_toml() {
        let source = r#"
            [renderer]
            name = "tanzo"
            default_platform = "desktop"

            [renderer.features]
            desktop = []
            web = ["tanzo-web"]
            ios = ["tanzo-mobile"]
            android = ["tanzo-mobile"]
        "#;

        let config: DioxusConfig = toml::from_str(source).expect("parse config");
        assert!(config.renderer.is_custom());
        assert_eq!(config.renderer.name.as_deref(), Some("tanzo"));
        assert_eq!(config.renderer.default_platform.as_deref(), Some("desktop"));
        assert_eq!(
            config.renderer.features_for_platform(&["desktop"]),
            Some(vec![])
        );
        assert_eq!(
            config.renderer.features_for_platform(&["web"]),
            Some(vec!["tanzo-web".to_string()])
        );
        assert_eq!(
            config.renderer.features_for_platform(&["macos", "desktop"]),
            Some(vec![])
        );
        assert_eq!(
            config.renderer.features_for_platform(&["nonexistent"]),
            None
        );
    }
}
