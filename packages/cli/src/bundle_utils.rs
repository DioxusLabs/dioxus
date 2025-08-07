use crate::{
    config::BundleConfig, CustomSignCommandSettings, DebianSettings, MacOsSettings,
    NSISInstallerMode, NsisSettings, PackageType, WebviewInstallMode, WindowsSettings, WixSettings,
};

impl From<NsisSettings> for tauri_bundler::NsisSettings {
    fn from(val: NsisSettings) -> Self {
        tauri_bundler::NsisSettings {
            header_image: val.header_image,
            sidebar_image: val.sidebar_image,
            installer_icon: val.installer_icon,
            install_mode: val.install_mode.into(),
            languages: val.languages,
            display_language_selector: val.display_language_selector,
            custom_language_files: None,
            template: None,
            compression: tauri_utils::config::NsisCompression::None,
            start_menu_folder: val.start_menu_folder,
            installer_hooks: val.installer_hooks,
            minimum_webview2_version: val.minimum_webview2_version,
        }
    }
}

impl From<BundleConfig> for tauri_bundler::BundleSettings {
    fn from(val: BundleConfig) -> Self {
        tauri_bundler::BundleSettings {
            identifier: val.identifier,
            publisher: val.publisher,
            icon: val.icon,
            resources: val.resources,
            copyright: val.copyright,
            category: val.category.and_then(|c| c.parse().ok()),
            short_description: val.short_description,
            long_description: val.long_description,
            external_bin: val.external_bin,
            deb: val.deb.map(Into::into).unwrap_or_default(),
            macos: val.macos.map(Into::into).unwrap_or_default(),
            windows: val.windows.map(Into::into).unwrap_or_default(),
            ..Default::default()
        }
    }
}

impl From<DebianSettings> for tauri_bundler::DebianSettings {
    fn from(val: DebianSettings) -> Self {
        tauri_bundler::DebianSettings {
            depends: val.depends,
            files: val.files,
            desktop_template: val.desktop_template,
            provides: val.provides,
            conflicts: val.conflicts,
            replaces: val.replaces,
            section: val.section,
            priority: val.priority,
            changelog: val.changelog,
            pre_install_script: val.pre_install_script,
            post_install_script: val.post_install_script,
            pre_remove_script: val.pre_remove_script,
            post_remove_script: val.post_remove_script,
            recommends: val.recommends,
        }
    }
}

impl From<WixSettings> for tauri_bundler::WixSettings {
    fn from(val: WixSettings) -> Self {
        tauri_bundler::WixSettings {
            language: tauri_bundler::bundle::WixLanguage({
                let mut languages: Vec<_> = val
                    .language
                    .iter()
                    .map(|l| {
                        (
                            l.0.clone(),
                            tauri_bundler::bundle::WixLanguageConfig {
                                locale_path: l.1.clone(),
                            },
                        )
                    })
                    .collect();
                if languages.is_empty() {
                    languages.push(("en-US".into(), Default::default()));
                }
                languages
            }),
            template: val.template,
            fragment_paths: val.fragment_paths,
            component_group_refs: val.component_group_refs,
            component_refs: val.component_refs,
            feature_group_refs: val.feature_group_refs,
            feature_refs: val.feature_refs,
            merge_refs: val.merge_refs,
            enable_elevated_update_task: val.enable_elevated_update_task,
            banner_path: val.banner_path,
            dialog_image_path: val.dialog_image_path,
            fips_compliant: val.fips_compliant,
            version: val.version,
            upgrade_code: val.upgrade_code,
        }
    }
}

impl From<MacOsSettings> for tauri_bundler::MacOsSettings {
    fn from(val: MacOsSettings) -> Self {
        tauri_bundler::MacOsSettings {
            frameworks: val.frameworks,
            minimum_system_version: val.minimum_system_version,
            exception_domain: val.exception_domain,
            signing_identity: val.signing_identity,
            provider_short_name: val.provider_short_name,
            entitlements: val.entitlements,
            info_plist_path: val.info_plist_path,
            files: val.files,
            hardened_runtime: val.hardened_runtime,
            bundle_version: val.bundle_version,
            bundle_name: val.bundle_name,
        }
    }
}

#[allow(deprecated)]
impl From<WindowsSettings> for tauri_bundler::WindowsSettings {
    fn from(val: WindowsSettings) -> Self {
        tauri_bundler::WindowsSettings {
            digest_algorithm: val.digest_algorithm,
            certificate_thumbprint: val.certificate_thumbprint,
            timestamp_url: val.timestamp_url,
            tsp: val.tsp,
            wix: val.wix.map(Into::into),
            webview_install_mode: val.webview_install_mode.into(),
            allow_downgrades: val.allow_downgrades,
            nsis: val.nsis.map(Into::into),
            sign_command: val.sign_command.map(Into::into),

            icon_path: val.icon_path.unwrap_or("./icons/icon.ico".into()),
        }
    }
}

impl From<NSISInstallerMode> for tauri_utils::config::NSISInstallerMode {
    fn from(val: NSISInstallerMode) -> Self {
        match val {
            NSISInstallerMode::CurrentUser => tauri_utils::config::NSISInstallerMode::CurrentUser,
            NSISInstallerMode::PerMachine => tauri_utils::config::NSISInstallerMode::PerMachine,
            NSISInstallerMode::Both => tauri_utils::config::NSISInstallerMode::Both,
        }
    }
}

impl From<PackageType> for tauri_bundler::PackageType {
    fn from(value: PackageType) -> Self {
        match value {
            PackageType::MacOsBundle => Self::MacOsBundle,
            PackageType::IosBundle => Self::IosBundle,
            PackageType::WindowsMsi => Self::WindowsMsi,
            PackageType::Deb => Self::Deb,
            PackageType::Rpm => Self::Rpm,
            PackageType::AppImage => Self::AppImage,
            PackageType::Dmg => Self::Dmg,
            PackageType::Updater => Self::Updater,
            PackageType::Nsis => Self::Nsis,
        }
    }
}

impl WebviewInstallMode {
    fn into(self) -> tauri_utils::config::WebviewInstallMode {
        match self {
            Self::Skip => tauri_utils::config::WebviewInstallMode::Skip,
            Self::DownloadBootstrapper { silent } => {
                tauri_utils::config::WebviewInstallMode::DownloadBootstrapper { silent }
            }
            Self::EmbedBootstrapper { silent } => {
                tauri_utils::config::WebviewInstallMode::EmbedBootstrapper { silent }
            }
            Self::OfflineInstaller { silent } => {
                tauri_utils::config::WebviewInstallMode::OfflineInstaller { silent }
            }
            Self::FixedRuntime { path } => {
                tauri_utils::config::WebviewInstallMode::FixedRuntime { path }
            }
        }
    }
}

impl From<CustomSignCommandSettings> for tauri_bundler::CustomSignCommandSettings {
    fn from(val: CustomSignCommandSettings) -> Self {
        tauri_bundler::CustomSignCommandSettings {
            cmd: val.cmd,
            args: val.args,
        }
    }
}
