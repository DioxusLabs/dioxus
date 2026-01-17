use crate::config::component::ComponentConfig;

use super::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

    /// Get the resolved publisher for a specific platform.
    /// Platform-specific publishers override the base bundle publisher.
    pub fn resolved_publisher(&self, platform: BundlePlatform) -> Option<&str> {
        let platform_override = match platform {
            BundlePlatform::Ios => self.ios.publisher.as_deref(),
            BundlePlatform::Android => self.android.publisher.as_deref(),
            BundlePlatform::MacOS => self.macos.publisher.as_deref(),
            BundlePlatform::Windows => self.windows.publisher.as_deref(),
            BundlePlatform::Linux => self.linux.publisher.as_deref(),
            BundlePlatform::Web => None,
        };
        platform_override.or(self.bundle.publisher.as_deref())
    }

    /// Get the resolved icons for a specific platform.
    /// Platform-specific icons override the base bundle icons.
    pub fn resolved_icons(&self, platform: BundlePlatform) -> Option<&[String]> {
        let platform_override = match platform {
            BundlePlatform::Ios => self.ios.icon.as_deref(),
            BundlePlatform::Android => self.android.icon.as_deref(),
            BundlePlatform::MacOS => self.macos.icon.as_deref(),
            BundlePlatform::Windows => self.windows.icon.as_deref(),
            BundlePlatform::Linux => self.linux.icon.as_deref(),
            BundlePlatform::Web => None,
        };
        platform_override.or(self.bundle.icon.as_deref())
    }

    /// Get the resolved resources for a specific platform.
    /// Platform-specific resources override the base bundle resources.
    pub fn resolved_resources(&self, platform: BundlePlatform) -> Option<&[String]> {
        let platform_override = match platform {
            BundlePlatform::Ios => self.ios.resources.as_deref(),
            BundlePlatform::Android => self.android.resources.as_deref(),
            BundlePlatform::MacOS => self.macos.resources.as_deref(),
            BundlePlatform::Windows => self.windows.resources.as_deref(),
            BundlePlatform::Linux => self.linux.resources.as_deref(),
            BundlePlatform::Web => None,
        };
        platform_override.or(self.bundle.resources.as_deref())
    }

    /// Get the resolved copyright for a specific platform.
    pub fn resolved_copyright(&self, platform: BundlePlatform) -> Option<&str> {
        let platform_override = match platform {
            BundlePlatform::Ios => self.ios.copyright.as_deref(),
            BundlePlatform::Android => self.android.copyright.as_deref(),
            BundlePlatform::MacOS => self.macos.copyright.as_deref(),
            BundlePlatform::Windows => self.windows.copyright.as_deref(),
            BundlePlatform::Linux => self.linux.copyright.as_deref(),
            BundlePlatform::Web => None,
        };
        platform_override.or(self.bundle.copyright.as_deref())
    }

    /// Get the resolved category for a specific platform.
    pub fn resolved_category(&self, platform: BundlePlatform) -> Option<&str> {
        let platform_override = match platform {
            BundlePlatform::Ios => self.ios.category.as_deref(),
            BundlePlatform::Android => self.android.category.as_deref(),
            BundlePlatform::MacOS => self.macos.category.as_deref(),
            BundlePlatform::Windows => self.windows.category.as_deref(),
            BundlePlatform::Linux => self.linux.category.as_deref(),
            BundlePlatform::Web => None,
        };
        platform_override.or(self.bundle.category.as_deref())
    }

    /// Get the resolved short description for a specific platform.
    pub fn resolved_short_description(&self, platform: BundlePlatform) -> Option<&str> {
        let platform_override = match platform {
            BundlePlatform::Ios => self.ios.short_description.as_deref(),
            BundlePlatform::Android => self.android.short_description.as_deref(),
            BundlePlatform::MacOS => self.macos.short_description.as_deref(),
            BundlePlatform::Windows => self.windows.short_description.as_deref(),
            BundlePlatform::Linux => self.linux.short_description.as_deref(),
            BundlePlatform::Web => None,
        };
        platform_override.or(self.bundle.short_description.as_deref())
    }

    /// Get the resolved long description for a specific platform.
    pub fn resolved_long_description(&self, platform: BundlePlatform) -> Option<&str> {
        let platform_override = match platform {
            BundlePlatform::Ios => self.ios.long_description.as_deref(),
            BundlePlatform::Android => self.android.long_description.as_deref(),
            BundlePlatform::MacOS => self.macos.long_description.as_deref(),
            BundlePlatform::Windows => self.windows.long_description.as_deref(),
            BundlePlatform::Linux => self.linux.long_description.as_deref(),
            BundlePlatform::Web => None,
        };
        platform_override.or(self.bundle.long_description.as_deref())
    }

    /// Get a resolved BundleConfig for a specific platform.
    /// This merges the base bundle config with platform-specific overrides.
    pub fn resolved_bundle(&self, platform: BundlePlatform) -> BundleConfig {
        let mut resolved = self.bundle.clone();

        // Apply common bundle settings from platform config
        match platform {
            BundlePlatform::Ios => {
                if self.ios.identifier.is_some() {
                    resolved.identifier = self.ios.identifier.clone();
                }
                if self.ios.publisher.is_some() {
                    resolved.publisher = self.ios.publisher.clone();
                }
                if self.ios.icon.is_some() {
                    resolved.icon = self.ios.icon.clone();
                }
                if self.ios.resources.is_some() {
                    resolved.resources = self.ios.resources.clone();
                }
                if self.ios.copyright.is_some() {
                    resolved.copyright = self.ios.copyright.clone();
                }
                if self.ios.category.is_some() {
                    resolved.category = self.ios.category.clone();
                }
                if self.ios.short_description.is_some() {
                    resolved.short_description = self.ios.short_description.clone();
                }
                if self.ios.long_description.is_some() {
                    resolved.long_description = self.ios.long_description.clone();
                }
            }
            BundlePlatform::Android => {
                if self.android.identifier.is_some() {
                    resolved.identifier = self.android.identifier.clone();
                }
                if self.android.publisher.is_some() {
                    resolved.publisher = self.android.publisher.clone();
                }
                if self.android.icon.is_some() {
                    resolved.icon = self.android.icon.clone();
                }
                if self.android.resources.is_some() {
                    resolved.resources = self.android.resources.clone();
                }
                if self.android.copyright.is_some() {
                    resolved.copyright = self.android.copyright.clone();
                }
                if self.android.category.is_some() {
                    resolved.category = self.android.category.clone();
                }
                if self.android.short_description.is_some() {
                    resolved.short_description = self.android.short_description.clone();
                }
                if self.android.long_description.is_some() {
                    resolved.long_description = self.android.long_description.clone();
                }
                // Android signing settings are now in android.signing
                // and merged separately when needed
            }
            BundlePlatform::MacOS => {
                if self.macos.identifier.is_some() {
                    resolved.identifier = self.macos.identifier.clone();
                }
                if self.macos.publisher.is_some() {
                    resolved.publisher = self.macos.publisher.clone();
                }
                if self.macos.icon.is_some() {
                    resolved.icon = self.macos.icon.clone();
                }
                if self.macos.resources.is_some() {
                    resolved.resources = self.macos.resources.clone();
                }
                if self.macos.copyright.is_some() {
                    resolved.copyright = self.macos.copyright.clone();
                }
                if self.macos.category.is_some() {
                    resolved.category = self.macos.category.clone();
                }
                if self.macos.short_description.is_some() {
                    resolved.short_description = self.macos.short_description.clone();
                }
                if self.macos.long_description.is_some() {
                    resolved.long_description = self.macos.long_description.clone();
                }
                // macOS bundle settings are merged into resolved.macos
                self.apply_macos_bundle_settings(&mut resolved);
            }
            BundlePlatform::Windows => {
                if self.windows.identifier.is_some() {
                    resolved.identifier = self.windows.identifier.clone();
                }
                if self.windows.publisher.is_some() {
                    resolved.publisher = self.windows.publisher.clone();
                }
                if self.windows.icon.is_some() {
                    resolved.icon = self.windows.icon.clone();
                }
                if self.windows.resources.is_some() {
                    resolved.resources = self.windows.resources.clone();
                }
                if self.windows.copyright.is_some() {
                    resolved.copyright = self.windows.copyright.clone();
                }
                if self.windows.category.is_some() {
                    resolved.category = self.windows.category.clone();
                }
                if self.windows.short_description.is_some() {
                    resolved.short_description = self.windows.short_description.clone();
                }
                if self.windows.long_description.is_some() {
                    resolved.long_description = self.windows.long_description.clone();
                }
                // Windows bundle settings are merged into resolved.windows
                self.apply_windows_bundle_settings(&mut resolved);
            }
            BundlePlatform::Linux => {
                if self.linux.identifier.is_some() {
                    resolved.identifier = self.linux.identifier.clone();
                }
                if self.linux.publisher.is_some() {
                    resolved.publisher = self.linux.publisher.clone();
                }
                if self.linux.icon.is_some() {
                    resolved.icon = self.linux.icon.clone();
                }
                if self.linux.resources.is_some() {
                    resolved.resources = self.linux.resources.clone();
                }
                if self.linux.copyright.is_some() {
                    resolved.copyright = self.linux.copyright.clone();
                }
                if self.linux.category.is_some() {
                    resolved.category = self.linux.category.clone();
                }
                if self.linux.short_description.is_some() {
                    resolved.short_description = self.linux.short_description.clone();
                }
                if self.linux.long_description.is_some() {
                    resolved.long_description = self.linux.long_description.clone();
                }
                // Linux deb settings are merged into resolved.deb
                self.apply_linux_bundle_settings(&mut resolved);
            }
            BundlePlatform::Web => {
                // Web doesn't have platform-specific bundle settings
            }
        }

        resolved
    }

    /// Apply macOS-specific bundle settings from [macos] to the resolved BundleConfig.
    fn apply_macos_bundle_settings(&self, resolved: &mut BundleConfig) {
        let mac = &self.macos;

        // Create or update MacOsSettings
        let macos_settings = resolved.macos.get_or_insert_with(Default::default);

        // Apply settings from [macos] that override [bundle.macos]
        if mac.bundle_version.is_some() {
            macos_settings.bundle_version = mac.bundle_version.clone();
        }
        if mac.bundle_name.is_some() {
            macos_settings.bundle_name = mac.bundle_name.clone();
        }
        if mac.signing_identity.is_some() {
            macos_settings.signing_identity = mac.signing_identity.clone();
        }
        if mac.provider_short_name.is_some() {
            macos_settings.provider_short_name = mac.provider_short_name.clone();
        }
        if mac.entitlements_file.is_some() {
            macos_settings.entitlements = mac.entitlements_file.clone();
        }
        if mac.exception_domain.is_some() {
            macos_settings.exception_domain = mac.exception_domain.clone();
        }
        if mac.license.is_some() {
            macos_settings.license = mac.license.clone();
        }
        if let Some(hr) = mac.hardened_runtime {
            macos_settings.hardened_runtime = hr;
        }
        if !mac.files.is_empty() {
            macos_settings.files = mac.files.clone();
        }
        // minimum_system_version and frameworks are handled separately since
        // they exist in both places
        if mac.minimum_system_version.is_some() {
            macos_settings.minimum_system_version = mac.minimum_system_version.clone();
        }
        if !mac.frameworks.is_empty() {
            macos_settings.frameworks = Some(mac.frameworks.clone());
        }
        if mac.info_plist.is_some() {
            macos_settings.info_plist_path = mac.info_plist.clone();
        }
    }

    /// Apply Windows-specific bundle settings from [windows] to the resolved BundleConfig.
    fn apply_windows_bundle_settings(&self, resolved: &mut BundleConfig) {
        let win = &self.windows;

        // Create or update WindowsSettings
        let windows_settings = resolved.windows.get_or_insert_with(Default::default);

        // Apply settings from [windows] that override [bundle.windows]
        if win.digest_algorithm.is_some() {
            windows_settings.digest_algorithm = win.digest_algorithm.clone();
        }
        if win.certificate_thumbprint.is_some() {
            windows_settings.certificate_thumbprint = win.certificate_thumbprint.clone();
        }
        if win.timestamp_url.is_some() {
            windows_settings.timestamp_url = win.timestamp_url.clone();
        }
        if let Some(tsp) = win.tsp {
            windows_settings.tsp = tsp;
        }
        if win.icon_path.is_some() {
            windows_settings.icon_path = win.icon_path.clone();
        }
        if let Some(allow) = win.allow_downgrades {
            windows_settings.allow_downgrades = allow;
        }

        // Handle WiX settings
        if let Some(wix) = &win.wix {
            let wix_settings = windows_settings.wix.get_or_insert_with(Default::default);
            if !wix.language.is_empty() {
                wix_settings.language = wix.language.clone();
            }
            if wix.template.is_some() {
                wix_settings.template = wix.template.clone();
            }
            if !wix.fragment_paths.is_empty() {
                wix_settings.fragment_paths = wix.fragment_paths.clone();
            }
            if !wix.component_group_refs.is_empty() {
                wix_settings.component_group_refs = wix.component_group_refs.clone();
            }
            if !wix.component_refs.is_empty() {
                wix_settings.component_refs = wix.component_refs.clone();
            }
            if !wix.feature_group_refs.is_empty() {
                wix_settings.feature_group_refs = wix.feature_group_refs.clone();
            }
            if !wix.feature_refs.is_empty() {
                wix_settings.feature_refs = wix.feature_refs.clone();
            }
            if !wix.merge_refs.is_empty() {
                wix_settings.merge_refs = wix.merge_refs.clone();
            }
            if let Some(skip) = wix.skip_webview_install {
                wix_settings.skip_webview_install = skip;
            }
            if wix.license.is_some() {
                wix_settings.license = wix.license.clone();
            }
            if let Some(elevated) = wix.enable_elevated_update_task {
                wix_settings.enable_elevated_update_task = elevated;
            }
            if wix.banner_path.is_some() {
                wix_settings.banner_path = wix.banner_path.clone();
            }
            if wix.dialog_image_path.is_some() {
                wix_settings.dialog_image_path = wix.dialog_image_path.clone();
            }
            if let Some(fips) = wix.fips_compliant {
                wix_settings.fips_compliant = fips;
            }
            if wix.version.is_some() {
                wix_settings.version = wix.version.clone();
            }
            if wix.upgrade_code.is_some() {
                wix_settings.upgrade_code = wix.upgrade_code;
            }
        }

        // Handle NSIS settings
        if let Some(nsis) = &win.nsis {
            let nsis_settings = windows_settings.nsis.get_or_insert_with(Default::default);
            if nsis.template.is_some() {
                nsis_settings.template = nsis.template.clone();
            }
            if nsis.license.is_some() {
                nsis_settings.license = nsis.license.clone();
            }
            if nsis.header_image.is_some() {
                nsis_settings.header_image = nsis.header_image.clone();
            }
            if nsis.sidebar_image.is_some() {
                nsis_settings.sidebar_image = nsis.sidebar_image.clone();
            }
            if nsis.installer_icon.is_some() {
                nsis_settings.installer_icon = nsis.installer_icon.clone();
            }
            // Install mode needs conversion from string to enum
            if let Some(mode) = &nsis.install_mode {
                nsis_settings.install_mode = match mode.as_str() {
                    "CurrentUser" => crate::config::NSISInstallerMode::CurrentUser,
                    "PerMachine" => crate::config::NSISInstallerMode::PerMachine,
                    "Both" => crate::config::NSISInstallerMode::Both,
                    _ => crate::config::NSISInstallerMode::CurrentUser,
                };
            }
            if nsis.languages.is_some() {
                nsis_settings.languages = nsis.languages.clone();
            }
            if nsis.custom_language_files.is_some() {
                nsis_settings.custom_language_files = nsis.custom_language_files.clone();
            }
            if let Some(display) = nsis.display_language_selector {
                nsis_settings.display_language_selector = display;
            }
            if nsis.start_menu_folder.is_some() {
                nsis_settings.start_menu_folder = nsis.start_menu_folder.clone();
            }
            if nsis.installer_hooks.is_some() {
                nsis_settings.installer_hooks = nsis.installer_hooks.clone();
            }
            if nsis.minimum_webview2_version.is_some() {
                nsis_settings.minimum_webview2_version = nsis.minimum_webview2_version.clone();
            }
        }

        // Handle sign command
        if let Some(sign_cmd) = &win.sign_command {
            windows_settings.sign_command = Some(crate::config::CustomSignCommandSettings {
                cmd: sign_cmd.cmd.clone(),
                args: sign_cmd.args.clone(),
            });
        }
    }

    /// Apply Linux-specific bundle settings from [linux] to the resolved BundleConfig.
    fn apply_linux_bundle_settings(&self, resolved: &mut BundleConfig) {
        let linux = &self.linux;

        // Handle Debian settings
        if let Some(deb) = &linux.deb {
            let deb_settings = resolved.deb.get_or_insert_with(Default::default);
            if deb.depends.is_some() {
                deb_settings.depends = deb.depends.clone();
            }
            if deb.recommends.is_some() {
                deb_settings.recommends = deb.recommends.clone();
            }
            if deb.provides.is_some() {
                deb_settings.provides = deb.provides.clone();
            }
            if deb.conflicts.is_some() {
                deb_settings.conflicts = deb.conflicts.clone();
            }
            if deb.replaces.is_some() {
                deb_settings.replaces = deb.replaces.clone();
            }
            if !deb.files.is_empty() {
                deb_settings.files = deb.files.clone();
            }
            if deb.desktop_template.is_some() {
                deb_settings.desktop_template = deb.desktop_template.clone();
            }
            if deb.section.is_some() {
                deb_settings.section = deb.section.clone();
            }
            if deb.priority.is_some() {
                deb_settings.priority = deb.priority.clone();
            }
            if deb.changelog.is_some() {
                deb_settings.changelog = deb.changelog.clone();
            }
            if deb.pre_install_script.is_some() {
                deb_settings.pre_install_script = deb.pre_install_script.clone();
            }
            if deb.post_install_script.is_some() {
                deb_settings.post_install_script = deb.post_install_script.clone();
            }
            if deb.pre_remove_script.is_some() {
                deb_settings.pre_remove_script = deb.pre_remove_script.clone();
            }
            if deb.post_remove_script.is_some() {
                deb_settings.post_remove_script = deb.post_remove_script.clone();
            }
        }
    }

    /// Get the resolved Android signing configuration.
    /// This prefers [android.signing] over [bundle.android].
    pub fn resolved_android_signing(&self) -> Option<crate::config::AndroidSettings> {
        // First check [android.signing]
        if let Some(signing) = &self.android.signing {
            return Some(crate::config::AndroidSettings {
                jks_file: signing.jks_file.clone(),
                jks_password: signing.jks_password.clone(),
                key_alias: signing.key_alias.clone(),
                key_password: signing.key_password.clone(),
            });
        }
        // Fall back to deprecated [bundle.android]
        self.bundle.android.clone()
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
