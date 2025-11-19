use crate::{AppBuilder, BuildArgs, BuildId, BuildMode, BuildRequest, BundleFormat, PackageType};
use anyhow::{bail, Context};
use path_absolutize::Absolutize;
use std::collections::HashMap;
// use tauri_bundler::{BundleBinary, BundleSettings, PackageSettings, SettingsBuilder};
use walkdir::WalkDir;

use super::*;

/// Bundle an app and its assets.
///
/// This will produce a client `public` folder and the associated server executable in the output folder.
#[derive(Clone, Debug, Parser)]
pub struct Bundle {
    /// The package types to bundle
    #[clap(long)]
    pub package_types: Option<Vec<PackageType>>,

    /// The directory in which the final bundle will be placed.
    ///
    /// Relative paths will be placed relative to the current working directory if specified.
    /// Otherwise, the out_dir path specified in Dioxus.toml will be used (relative to the crate root).
    ///
    /// We will flatten the artifacts into this directory - there will be no differentiation between
    /// artifacts produced by different platforms.
    #[clap(long)]
    pub out_dir: Option<PathBuf>,

    /// The arguments for the dioxus build
    #[clap(flatten)]
    pub(crate) args: CommandWithPlatformOverrides<BuildArgs>,
}

impl Bundle {
    // todo: make sure to run pre-render static routes! we removed this from the other bundling step
    pub(crate) async fn bundle(mut self) -> Result<StructuredOutput> {
        tracing::info!("Bundling project...");

        let BuildTargets { client, server } = self.args.into_targets().await?;

        let mut server_artifacts = None;
        let client_artifacts =
            AppBuilder::started(&client, BuildMode::Base { run: false }, BuildId::PRIMARY)?
                .finish_build()
                .await?;

        tracing::info!(path = ?client.root_dir(), "Client build completed successfully! 🚀");

        if let Some(server) = server.as_ref() {
            // If the server is present, we need to build it as well
            server_artifacts = Some(
                AppBuilder::started(server, BuildMode::Base { run: false }, BuildId::SECONDARY)?
                    .finish_build()
                    .await?,
            );

            tracing::info!(path = ?client.root_dir(), "Server build completed successfully! 🚀");
        }

        // If we're building for iOS, we need to bundle the iOS bundle
        if client.bundle == BundleFormat::Ios && self.package_types.is_none() {
            self.package_types = Some(vec![crate::PackageType::IosBundle]);
        }

        let mut bundles = vec![];

        // Copy the server over if it exists
        if let Some(server) = server.as_ref() {
            bundles.push(server.main_exe());
        }

        // Create a list of bundles that we might need to copy
        match client.bundle {
            // By default, mac/win/linux work with tauri bundle
            BundleFormat::MacOS | BundleFormat::Linux | BundleFormat::Windows => {
                tracing::info!("Running desktop bundler...");
                for bundle in Self::bundle_desktop(&client, &self.package_types)? {
                    bundles.push(bundle);
                }
            }

            // Web/ios can just use their root_dir
            BundleFormat::Web => bundles.push(client.root_dir()),
            BundleFormat::Ios => {
                tracing::warn!("iOS bundles are not currently codesigned! You will need to codesign the app before distributing.");
                bundles.push(client.root_dir())
            }
            BundleFormat::Server => bundles.push(client.root_dir()),
            BundleFormat::Android => {
                let aab = client
                    .android_gradle_bundle()
                    .await
                    .context("Failed to run gradle bundleRelease")?;
                bundles.push(aab);
            }
        };

        // Copy the bundles to the output directory if one was specified
        let crate_outdir = client.crate_out_dir();
        if let Some(outdir) = self.out_dir.clone().or(crate_outdir) {
            let outdir = outdir
                .absolutize()
                .context("Failed to absolutize output directory")?;

            tracing::info!("Copying bundles to output directory: {}", outdir.display());

            std::fs::create_dir_all(&outdir)?;

            for bundle_path in bundles.iter_mut() {
                let destination = outdir.join(bundle_path.file_name().unwrap());

                tracing::debug!(
                    "Copying from {} to {}",
                    bundle_path.display(),
                    destination.display()
                );

                if bundle_path.is_dir() {
                    dircpy::CopyBuilder::new(&bundle_path, &destination)
                        .overwrite(true)
                        .run_par()
                        .context("Failed to copy the app to output directory")?;
                } else {
                    std::fs::copy(&bundle_path, &destination)
                        .context("Failed to copy the app to output directory")?;
                }

                *bundle_path = destination;
            }
        }

        for bundle_path in bundles.iter() {
            tracing::info!(
                "Bundled app at: {}",
                bundle_path.absolutize().unwrap().display()
            );
        }

        let client = client_artifacts.into_structured_output();
        let server = server_artifacts.map(|s| s.into_structured_output());

        Ok(StructuredOutput::BundleOutput {
            bundles,
            client,
            server,
        })
    }

    fn bundle_desktop(
        build: &BuildRequest,
        package_types: &Option<Vec<PackageType>>,
    ) -> Result<Vec<PathBuf>, Error> {
        todo!()
        // let krate = &build;
        // let exe = build.main_exe();

        // _ = std::fs::remove_dir_all(krate.bundle_dir(build.bundle));

        // let package = krate.package();
        // let mut name: PathBuf = krate.executable_name().into();
        // if cfg!(windows) {
        //     name.set_extension("exe");
        // }
        // std::fs::create_dir_all(krate.bundle_dir(build.bundle))
        //     .context("Failed to create bundle directory")?;
        // std::fs::copy(&exe, krate.bundle_dir(build.bundle).join(&name))
        //     .with_context(|| "Failed to copy the output executable into the bundle directory")?;

        // let binaries = vec![
        //     // We use the name of the exe but it has to be in the same directory
        //     BundleBinary::new(krate.executable_name().to_string(), true)
        //         .set_src_path(Some(exe.display().to_string())),
        // ];

        // let mut bundle_settings: BundleSettings = krate.config.bundle.clone().into();

        // // Check if required fields are provided instead of failing silently.
        // if bundle_settings.identifier.is_none() {
        //     bail!("\n\nBundle identifier was not provided in `Dioxus.toml`. Add it as:\n\n[bundle]\nidentifier = \"com.mycompany\"\n\n");
        // }
        // if bundle_settings.publisher.is_none() {
        //     bail!("\n\nBundle publisher was not provided in `Dioxus.toml`. Add it as:\n\n[bundle]\npublisher = \"MyCompany\"\n\n");
        // }

        // if cfg!(windows) {
        //     let windows_icon_override = krate.config.bundle.windows.as_ref().map(|w| &w.icon_path);
        //     if windows_icon_override.is_none() {
        //         let icon_path = bundle_settings
        //             .icon
        //             .as_ref()
        //             .and_then(|icons| icons.first());

        //         if let Some(icon_path) = icon_path {
        //             bundle_settings.icon = Some(vec![icon_path.into()]);
        //         };
        //     }
        // }

        // if bundle_settings.resources_map.is_none() {
        //     bundle_settings.resources_map = Some(HashMap::new());
        // }

        // let asset_dir = build.asset_dir();
        // if asset_dir.exists() {
        //     for entry in WalkDir::new(&asset_dir) {
        //         let entry = entry.unwrap();
        //         let path = entry.path();

        //         if path.is_file() {
        //             let old = path
        //                 .canonicalize()
        //                 .with_context(|| format!("Failed to canonicalize {entry:?}"))?;
        //             let new =
        //                 PathBuf::from("assets").join(path.strip_prefix(&asset_dir).unwrap_or(path));

        //             tracing::debug!("Bundled asset: {old:?} -> {new:?}");
        //             bundle_settings
        //                 .resources_map
        //                 .as_mut()
        //                 .expect("to be set")
        //                 .insert(old.display().to_string(), new.display().to_string());
        //         }
        //     }
        // }

        // for resource_path in bundle_settings.resources.take().into_iter().flatten() {
        //     bundle_settings
        //         .resources_map
        //         .as_mut()
        //         .expect("to be set")
        //         .insert(resource_path, "".to_string());
        // }

        // let mut settings = SettingsBuilder::new()
        //     .project_out_directory(krate.bundle_dir(build.bundle))
        //     .package_settings(PackageSettings {
        //         product_name: krate.bundled_app_name(),
        //         version: package.version.to_string(),
        //         description: package.description.clone().unwrap_or_default(),
        //         homepage: Some(package.homepage.clone().unwrap_or_default()),
        //         authors: Some(package.authors.clone()),
        //         default_run: Some(name.display().to_string()),
        //     })
        //     .log_level(log::Level::Debug)
        //     .binaries(binaries)
        //     .bundle_settings(bundle_settings);

        // if let Some(packages) = &package_types {
        //     settings = settings.package_types(packages.iter().map(|p| (*p).into()).collect());
        // }

        // settings = settings.target(build.triple.to_string());

        // let settings = settings
        //     .build()
        //     .context("failed to bundle tauri bundle settings")?;
        // tracing::debug!("Bundling project with settings: {:#?}", settings);
        // // if cfg!(target_os = "macos") {
        // //     std::env::set_var("CI", "true");
        // // }

        // // let bundles = tauri_bundler::bundle::bundle_project(&settings).inspect_err(|err| {
        // //     tracing::error!("Failed to bundle project: {:#?}", err);
        // //     if cfg!(target_os = "macos") {
        // //         tracing::error!("Make sure you have automation enabled in your terminal (https://github.com/tauri-apps/tauri/issues/3055#issuecomment-1624389208) and full disk access enabled for your terminal (https://github.com/tauri-apps/tauri/issues/3055#issuecomment-1624389208)");
        // //     }
        // // })?;

        // Ok(bundles)
    }
}

// use crate::{
//     config::BundleConfig, CustomSignCommandSettings, DebianSettings, MacOsSettings,
//     NSISInstallerMode, NsisSettings, PackageType, WebviewInstallMode, WindowsSettings, WixSettings,
// };

// impl From<NsisSettings> for tauri_bundler::NsisSettings {
//     fn from(val: NsisSettings) -> Self {
//         tauri_bundler::NsisSettings {
//             header_image: val.header_image,
//             sidebar_image: val.sidebar_image,
//             installer_icon: val.installer_icon,
//             install_mode: val.install_mode.into(),
//             languages: val.languages,
//             display_language_selector: val.display_language_selector,
//             custom_language_files: None,
//             template: None,
//             compression: tauri_utils::config::NsisCompression::None,
//             start_menu_folder: val.start_menu_folder,
//             installer_hooks: val.installer_hooks,
//             minimum_webview2_version: val.minimum_webview2_version,
//         }
//     }
// }

// impl From<BundleConfig> for tauri_bundler::BundleSettings {
//     fn from(val: BundleConfig) -> Self {
//         tauri_bundler::BundleSettings {
//             identifier: val.identifier,
//             publisher: val.publisher,
//             icon: val.icon,
//             resources: val.resources,
//             copyright: val.copyright,
//             category: val.category.and_then(|c| c.parse().ok()),
//             short_description: val.short_description,
//             long_description: val.long_description,
//             external_bin: val.external_bin,
//             deb: val.deb.map(Into::into).unwrap_or_default(),
//             macos: val.macos.map(Into::into).unwrap_or_default(),
//             windows: val.windows.map(Into::into).unwrap_or_default(),
//             ..Default::default()
//         }
//     }
// }

// impl From<DebianSettings> for tauri_bundler::DebianSettings {
//     fn from(val: DebianSettings) -> Self {
//         tauri_bundler::DebianSettings {
//             depends: val.depends,
//             files: val.files,
//             desktop_template: val.desktop_template,
//             provides: val.provides,
//             conflicts: val.conflicts,
//             replaces: val.replaces,
//             section: val.section,
//             priority: val.priority,
//             changelog: val.changelog,
//             pre_install_script: val.pre_install_script,
//             post_install_script: val.post_install_script,
//             pre_remove_script: val.pre_remove_script,
//             post_remove_script: val.post_remove_script,
//             recommends: val.recommends,
//         }
//     }
// }

// impl From<WixSettings> for tauri_bundler::WixSettings {
//     fn from(val: WixSettings) -> Self {
//         tauri_bundler::WixSettings {
//             language: tauri_bundler::bundle::WixLanguage({
//                 let mut languages: Vec<_> = val
//                     .language
//                     .iter()
//                     .map(|l| {
//                         (
//                             l.0.clone(),
//                             tauri_bundler::bundle::WixLanguageConfig {
//                                 locale_path: l.1.clone(),
//                             },
//                         )
//                     })
//                     .collect();
//                 if languages.is_empty() {
//                     languages.push(("en-US".into(), Default::default()));
//                 }
//                 languages
//             }),
//             template: val.template,
//             fragment_paths: val.fragment_paths,
//             component_group_refs: val.component_group_refs,
//             component_refs: val.component_refs,
//             feature_group_refs: val.feature_group_refs,
//             feature_refs: val.feature_refs,
//             merge_refs: val.merge_refs,
//             enable_elevated_update_task: val.enable_elevated_update_task,
//             banner_path: val.banner_path,
//             dialog_image_path: val.dialog_image_path,
//             fips_compliant: val.fips_compliant,
//             version: val.version,
//             upgrade_code: val.upgrade_code,
//         }
//     }
// }

// impl From<MacOsSettings> for tauri_bundler::MacOsSettings {
//     fn from(val: MacOsSettings) -> Self {
//         tauri_bundler::MacOsSettings {
//             frameworks: val.frameworks,
//             minimum_system_version: val.minimum_system_version,
//             exception_domain: val.exception_domain,
//             signing_identity: val.signing_identity,
//             provider_short_name: val.provider_short_name,
//             entitlements: val.entitlements,
//             info_plist_path: val.info_plist_path,
//             files: val.files,
//             hardened_runtime: val.hardened_runtime,
//             bundle_version: val.bundle_version,
//             bundle_name: val.bundle_name,
//         }
//     }
// }

// #[allow(deprecated)]
// impl From<WindowsSettings> for tauri_bundler::WindowsSettings {
//     fn from(val: WindowsSettings) -> Self {
//         tauri_bundler::WindowsSettings {
//             digest_algorithm: val.digest_algorithm,
//             certificate_thumbprint: val.certificate_thumbprint,
//             timestamp_url: val.timestamp_url,
//             tsp: val.tsp,
//             wix: val.wix.map(Into::into),
//             webview_install_mode: val.webview_install_mode.into(),
//             allow_downgrades: val.allow_downgrades,
//             nsis: val.nsis.map(Into::into),
//             sign_command: val.sign_command.map(Into::into),

//             icon_path: val.icon_path.unwrap_or("./icons/icon.ico".into()),
//         }
//     }
// }

// impl From<NSISInstallerMode> for tauri_utils::config::NSISInstallerMode {
//     fn from(val: NSISInstallerMode) -> Self {
//         match val {
//             NSISInstallerMode::CurrentUser => tauri_utils::config::NSISInstallerMode::CurrentUser,
//             NSISInstallerMode::PerMachine => tauri_utils::config::NSISInstallerMode::PerMachine,
//             NSISInstallerMode::Both => tauri_utils::config::NSISInstallerMode::Both,
//         }
//     }
// }

// impl From<PackageType> for tauri_bundler::PackageType {
//     fn from(value: PackageType) -> Self {
//         match value {
//             PackageType::MacOsBundle => Self::MacOsBundle,
//             PackageType::IosBundle => Self::IosBundle,
//             PackageType::WindowsMsi => Self::WindowsMsi,
//             PackageType::Deb => Self::Deb,
//             PackageType::Rpm => Self::Rpm,
//             PackageType::AppImage => Self::AppImage,
//             PackageType::Dmg => Self::Dmg,
//             PackageType::Updater => Self::Updater,
//             PackageType::Nsis => Self::Nsis,
//         }
//     }
// }

// impl WebviewInstallMode {
//     fn into(self) -> tauri_utils::config::WebviewInstallMode {
//         match self {
//             Self::Skip => tauri_utils::config::WebviewInstallMode::Skip,
//             Self::DownloadBootstrapper { silent } => {
//                 tauri_utils::config::WebviewInstallMode::DownloadBootstrapper { silent }
//             }
//             Self::EmbedBootstrapper { silent } => {
//                 tauri_utils::config::WebviewInstallMode::EmbedBootstrapper { silent }
//             }
//             Self::OfflineInstaller { silent } => {
//                 tauri_utils::config::WebviewInstallMode::OfflineInstaller { silent }
//             }
//             Self::FixedRuntime { path } => {
//                 tauri_utils::config::WebviewInstallMode::FixedRuntime { path }
//             }
//         }
//     }
// }

// impl From<CustomSignCommandSettings> for tauri_bundler::CustomSignCommandSettings {
//     fn from(val: CustomSignCommandSettings) -> Self {
//         tauri_bundler::CustomSignCommandSettings {
//             cmd: val.cmd,
//             args: val.args,
//         }
//     }
// }
