use crate::{AppBundle, BuildArgs, Builder, DioxusCrate, Platform};
use anyhow::Context;
use path_absolutize::Absolutize;
use std::collections::HashMap;
use tauri_bundler::{BundleBinary, BundleSettings, PackageSettings, SettingsBuilder};

use super::*;

/// Bundle the Rust desktop app and all of its assets
#[derive(Clone, Debug, Parser)]
pub struct Bundle {
    /// The package types to bundle
    #[clap(long)]
    pub package_types: Option<Vec<crate::PackageType>>,

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
    pub(crate) build_arguments: BuildArgs,
}

impl Bundle {
    pub(crate) async fn bundle(mut self) -> Result<StructuredOutput> {
        tracing::info!("Bundling project...");

        let krate = DioxusCrate::new(&self.build_arguments.target_args)
            .context("Failed to load Dioxus workspace")?;

        // We always use `release` mode for bundling
        self.build_arguments.release = true;
        self.build_arguments.resolve(&krate).await?;

        tracing::info!("Building app...");

        let bundle = Builder::start(&krate, self.build_arguments.clone())?
            .finish()
            .await?;

        // If we're building for iOS, we need to bundle the iOS bundle
        if self.build_arguments.platform() == Platform::Ios && self.package_types.is_none() {
            self.package_types = Some(vec![crate::PackageType::IosBundle]);
        }

        let mut bundles = vec![];

        // Copy the server over if it exists
        if bundle.build.build.fullstack {
            bundles.push(bundle.server_exe().unwrap());
        }

        // Create a list of bundles that we might need to copy
        match self.build_arguments.platform() {
            // By default, mac/win/linux work with tauri bundle
            Platform::MacOS | Platform::Linux | Platform::Windows => {
                tracing::info!("Running desktop bundler...");
                for bundle in self.bundle_desktop(&krate, &bundle)? {
                    bundles.extend(bundle.bundle_paths);
                }
            }

            // Web/ios can just use their root_dir
            Platform::Web => bundles.push(bundle.build.root_dir()),
            Platform::Ios => {
                tracing::warn!("iOS bundles are not currently codesigned! You will need to codesign the app before distributing.");
                bundles.push(bundle.build.root_dir())
            }
            Platform::Server => bundles.push(bundle.build.root_dir()),
            Platform::Liveview => bundles.push(bundle.build.root_dir()),

            Platform::Android => {
                let aab = bundle
                    .android_gradle_bundle()
                    .await
                    .context("Failed to run gradle bundleRelease")?;
                bundles.push(aab);
            }
        };

        // Copy the bundles to the output directory if one was specified
        let crate_outdir = bundle.build.krate.crate_out_dir();
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

        Ok(StructuredOutput::BundleOutput { bundles })
    }

    fn bundle_desktop(
        &self,
        krate: &DioxusCrate,
        bundle: &AppBundle,
    ) -> Result<Vec<tauri_bundler::Bundle>, Error> {
        _ = std::fs::remove_dir_all(krate.bundle_dir(self.build_arguments.platform()));

        let package = krate.package();
        let mut name: PathBuf = krate.executable_name().into();
        if cfg!(windows) {
            name.set_extension("exe");
        }
        std::fs::create_dir_all(krate.bundle_dir(self.build_arguments.platform()))
            .context("Failed to create bundle directory")?;
        std::fs::copy(
            &bundle.app.exe,
            krate
                .bundle_dir(self.build_arguments.platform())
                .join(&name),
        )
        .with_context(|| "Failed to copy the output executable into the bundle directory")?;

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

        let asset_dir = bundle.build.asset_dir();
        if asset_dir.exists() {
            let asset_dir_entries = std::fs::read_dir(&asset_dir)
                .with_context(|| format!("failed to read asset directory {:?}", asset_dir))?;
            for entry in asset_dir_entries.flatten() {
                let old = entry
                    .path()
                    .canonicalize()
                    .with_context(|| format!("Failed to canonicalize {entry:?}"))?;
                let new = PathBuf::from("assets").join(old.file_name().expect("Filename to exist"));
                tracing::debug!("Bundled asset: {old:?} -> {new:?}");

                bundle_settings
                    .resources_map
                    .as_mut()
                    .expect("to be set")
                    .insert(old.display().to_string(), new.display().to_string());
            }
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
                product_name: krate.bundled_app_name(),
                version: package.version.to_string(),
                description: package.description.clone().unwrap_or_default(),
                homepage: Some(package.homepage.clone().unwrap_or_default()),
                authors: Some(package.authors.clone()),
                default_run: Some(name.display().to_string()),
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

        let settings = settings
            .build()
            .context("failed to bundle tauri bundle settings")?;
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
