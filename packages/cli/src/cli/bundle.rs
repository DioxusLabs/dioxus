use crate::{AppBuilder, BuildArgs, BuildId, BuildMode, BuildRequest, BundleFormat};
use anyhow::{bail, Context};
use path_absolutize::Absolutize;
use std::collections::HashMap;
use tauri_bundler::{BundleBinary, BundleSettings, PackageSettings, SettingsBuilder};

use walkdir::WalkDir;

use super::*;

/// Bundle an app and its assets.
///
/// This will produce a client `public` folder and the associated server executable in the output folder.
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
                    bundles.extend(bundle.bundle_paths);
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
        package_types: &Option<Vec<crate::PackageType>>,
    ) -> Result<Vec<tauri_bundler::Bundle>, Error> {
        let krate = &build;
        let exe = build.main_exe();

        _ = std::fs::remove_dir_all(krate.bundle_dir(build.bundle));

        let package = krate.package();
        let mut name: PathBuf = krate.executable_name().into();
        if cfg!(windows) {
            name.set_extension("exe");
        }
        std::fs::create_dir_all(krate.bundle_dir(build.bundle))
            .context("Failed to create bundle directory")?;
        std::fs::copy(&exe, krate.bundle_dir(build.bundle).join(&name))
            .with_context(|| "Failed to copy the output executable into the bundle directory")?;

        let binaries = vec![
            // We use the name of the exe but it has to be in the same directory
            BundleBinary::new(krate.executable_name().to_string(), true)
                .set_src_path(Some(exe.display().to_string())),
        ];

        let mut bundle_settings: BundleSettings = krate.config.bundle.clone().into();

        // Check if required fields are provided instead of failing silently.
        if bundle_settings.identifier.is_none() {
            bail!("\n\nBundle identifier was not provided in `Dioxus.toml`. Add it as:\n\n[bundle]\nidentifier = \"com.mycompany\"\n\n");
        }
        if bundle_settings.publisher.is_none() {
            bail!("\n\nBundle publisher was not provided in `Dioxus.toml`. Add it as:\n\n[bundle]\npublisher = \"MyCompany\"\n\n");
        }

        /// Resolve an icon path relative to the crate dir
        fn canonicalize_icon_path(build: &BuildRequest, icon: &mut String) -> Result<(), Error> {
            let icon_path = build
                .crate_dir()
                .join(&icon)
                .canonicalize()
                .with_context(|| format!("Failed to canonicalize path to icon {icon:?}"))?;
            *icon = icon_path.to_string_lossy().to_string();
            Ok(())
        }

        // Resolve bundle.icon relative to the crate dir
        if let Some(icons) = bundle_settings.icon.as_mut() {
            for icon in icons.iter_mut() {
                canonicalize_icon_path(build, icon)?;
            }
        }

        #[allow(deprecated)]
        if cfg!(windows) {
            // Resolve bundle.windows.icon_path relative to the crate dir
            let mut windows_icon_path = bundle_settings
                .windows
                .icon_path
                .to_string_lossy()
                .to_string();
            canonicalize_icon_path(build, &mut windows_icon_path)?;
            bundle_settings.windows.icon_path = PathBuf::from(&windows_icon_path);

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

        let asset_dir = build.asset_dir();
        if asset_dir.exists() {
            for entry in WalkDir::new(&asset_dir) {
                let entry = entry.unwrap();
                let path = entry.path();

                if path.is_file() {
                    let old = path
                        .canonicalize()
                        .with_context(|| format!("Failed to canonicalize {entry:?}"))?;
                    let new =
                        PathBuf::from("assets").join(path.strip_prefix(&asset_dir).unwrap_or(path));

                    tracing::debug!("Bundled asset: {old:?} -> {new:?}");
                    bundle_settings
                        .resources_map
                        .as_mut()
                        .expect("to be set")
                        .insert(old.display().to_string(), new.display().to_string());
                }
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
            .project_out_directory(krate.bundle_dir(build.bundle))
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

        if let Some(packages) = &package_types {
            settings = settings.package_types(packages.iter().map(|p| (*p).into()).collect());
        }

        settings = settings.target(build.triple.to_string());

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
