use crate::{build::BuildArgs, AppBundle, Builder, DioxusCrate, Platform};
use anyhow::Context;
use std::collections::HashMap;
use tauri_bundler::{BundleBinary, BundleSettings, PackageSettings, SettingsBuilder};

use super::*;

/// Bundle the Rust desktop app and all of its assets
#[derive(Clone, Debug, Parser)]
#[clap(name = "bundle")]
pub struct Bundle {
    /// The package types to bundle
    ///
    /// Any of:
    /// - macos: The macOS application bundle (.app).
    /// - ios: The iOS app bundle.
    /// - msi: The Windows bundle (.msi).
    /// - nsis: The NSIS bundle (.exe).
    /// - deb: The Linux Debian package bundle (.deb).
    /// - rpm: The Linux RPM bundle (.rpm).
    /// - appimage: The Linux AppImage bundle (.AppImage).
    /// - dmg: The macOS DMG bundle (.dmg).
    /// - updater: The Updater bundle.
    #[clap(long)]
    pub package_types: Option<Vec<crate::PackageType>>,

    /// The directory in which the final bundle will be placed.
    ///
    /// Relative paths will be placed relative to the current working directory.
    ///
    /// We will flatten the artifacts into this directory - there will be no differentiation between
    /// artifacts produced by different platforms.
    #[clap(long)]
    pub outdir: Option<PathBuf>,

    /// The arguments for the dioxus build
    #[clap(flatten)]
    pub(crate) build_arguments: BuildArgs,
}

impl Bundle {
    pub(crate) async fn bundle(mut self) -> Result<StructuredOutput> {
        tracing::info!("Bundling project...");

        let krate = DioxusCrate::new(&self.build_arguments.target_args)
            .context("Failed to load Dioxus workspace")?;

        self.build_arguments.resolve(&krate)?;

        tracing::info!("Building app...");

        let bundle = Builder::start(&krate, self.build_arguments.clone())?
            .finish()
            .await?;

        tracing::info!("Copying app to output directory...");

        // If we're building for iOS, we need to bundle the iOS bundle
        if self.build_arguments.platform() == Platform::Ios && self.package_types.is_none() {
            self.package_types = Some(vec![crate::PackageType::IosBundle]);
        }

        let mut cmd_result = StructuredOutput::GenericSuccess;

        match self.build_arguments.platform() {
            // By default, mac/win/linux work with tauri bundle
            Platform::MacOS | Platform::Linux | Platform::Windows => {
                let bundles = self.bundle_desktop(krate, bundle)?;

                tracing::info!("Bundled app successfully!");
                tracing::info!("App produced {} outputs:", bundles.len());
                tracing::debug!("Bundling produced bundles: {:#?}", bundles);

                // Copy the bundles to the output directory and log their locations
                let mut bundle_paths = vec![];
                for bundle in bundles {
                    for src in bundle.bundle_paths {
                        let src = if let Some(outdir) = &self.outdir {
                            let dest = outdir.join(src.file_name().unwrap());
                            crate::fastfs::copy_asset(&src, &dest)?;
                            dest
                        } else {
                            src.clone()
                        };

                        tracing::info!(
                            "{} - [{}]",
                            bundle.package_type.short_name(),
                            src.display()
                        );

                        bundle_paths.push(src);
                    }
                }

                cmd_result = StructuredOutput::BundleOutput {
                    platform: self.build_arguments.platform(),
                    bundles: bundle_paths,
                };
            }

            Platform::Web => {
                tracing::info!("App available at: {}", bundle.app_dir().display());
            }

            Platform::Ios => {
                tracing::warn!("Signed iOS bundles are not yet supported");
                tracing::info!("The bundle is available at: {}", bundle.app_dir().display());
            }

            Platform::Server => {
                tracing::info!("Server available at: {}", bundle.app_dir().display())
            }
            Platform::Liveview => tracing::info!(
                "Liveview server available at: {}",
                bundle.app_dir().display()
            ),

            Platform::Android => {
                return Err(Error::UnsupportedFeature(
                    "Android bundles are not yet supported".into(),
                ));
            }
        };

        Ok(cmd_result)
    }

    fn bundle_desktop(
        &self,
        krate: DioxusCrate,
        bundle: AppBundle,
    ) -> Result<Vec<tauri_bundler::Bundle>, Error> {
        _ = std::fs::remove_dir_all(krate.bundle_dir(self.build_arguments.platform()));

        let package = krate.package();
        let mut name: PathBuf = krate.executable_name().into();
        if cfg!(windows) {
            name.set_extension("exe");
        }
        std::fs::create_dir_all(krate.bundle_dir(self.build_arguments.platform()))?;
        std::fs::copy(
            &bundle.app.exe,
            krate
                .bundle_dir(self.build_arguments.platform())
                .join(krate.executable_name()),
        )?;

        let binaries = vec![
            // We use the name of the exe but it has to be in the same directory
            BundleBinary::new(name.display().to_string(), true)
                .set_src_path(Some(bundle.app.exe.display().to_string())),
        ];

        let mut bundle_settings: BundleSettings = krate.config.bundle.clone().into();

        if cfg!(windows) {
            let windows_icon_override = krate.config.bundle.windows.as_ref().map(|w| &w.icon_path);
            if windows_icon_override.is_none() {
                let icon_path = bundle_settings
                    .icon
                    .as_ref()
                    .and_then(|icons| icons.first());

                if let Some(icon_path) = icon_path {
                    bundle_settings.icon = Some(vec![icon_path.into()]);
                };
            }
        }

        if bundle_settings.resources_map.is_none() {
            bundle_settings.resources_map = Some(HashMap::new());
        }

        for entry in std::fs::read_dir(bundle.asset_dir())?.flatten() {
            let old = entry.path().canonicalize()?;
            let new = PathBuf::from("/assets").join(old.file_name().unwrap());
            tracing::debug!("Bundled asset: {old:?} -> {new:?}");

            bundle_settings
                .resources_map
                .as_mut()
                .expect("to be set")
                .insert(old.display().to_string(), new.display().to_string());
        }

        for resource_path in bundle_settings.resources.take().into_iter().flatten() {
            bundle_settings
                .resources_map
                .as_mut()
                .expect("to be set")
                .insert(resource_path, "".to_string());
        }

        let mut settings = SettingsBuilder::new()
            .project_out_directory(krate.bundle_dir(self.build_arguments.platform()))
            .package_settings(PackageSettings {
                product_name: krate.executable_name().to_string(),
                version: package.version.to_string(),
                description: package.description.clone().unwrap_or_default(),
                homepage: Some(package.homepage.clone().unwrap_or_default()),
                authors: Some(package.authors.clone()),
                default_run: Some(krate.executable_name().to_string()),
            })
            .log_level(log::Level::Debug)
            .binaries(binaries)
            .bundle_settings(bundle_settings);

        if let Some(packages) = &self.package_types {
            settings = settings.package_types(packages.iter().map(|p| (*p).into()).collect());
        }

        if let Some(target) = self.build_arguments.target_args.target.as_ref() {
            settings = settings.target(target.to_string());
        }

        if self.build_arguments.platform() == Platform::Ios {
            settings = settings.target("aarch64-apple-ios".to_string());
        }

        let settings = settings.build()?;
        tracing::debug!("Bundling project with settings: {:#?}", settings);
        if cfg!(target_os = "macos") {
            std::env::set_var("CI", "true");
        }

        let bundles = tauri_bundler::bundle::bundle_project(&settings).inspect_err(|err| {
            tracing::error!("Failed to bundle project: {:#?}", err);
            if cfg!(target_os = "macos") {
                tracing::error!("Make sure you have automation enabled in your terminal (https://github.com/tauri-apps/tauri/issues/3055#issuecomment-1624389208) and full disk access enabled for your terminal (https://github.com/tauri-apps/tauri/issues/3055#issuecomment-1624389208)");
            }
        })?;

        Ok(bundles)
    }
}
