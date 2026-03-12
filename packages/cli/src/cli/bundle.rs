use crate::{AppBuilder, BuildArgs, BuildId, BuildMode, BuildRequest, BundleFormat, PackageType};
use anyhow::Context;
use path_absolutize::Absolutize;
use std::path::PathBuf;

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

        let client_artifacts =
            AppBuilder::started(&client, BuildMode::Base { run: false }, BuildId::PRIMARY)?
                .finish_build()
                .await?;
        let mut server_artifacts = None;

        if let Some(server) = server.as_ref() {
            // If the server is present, we need to build it as well
            server_artifacts = Some(
                AppBuilder::started(server, BuildMode::Base { run: false }, BuildId::SECONDARY)?
                    .finish_build()
                    .await?,
            );

            tracing::info!(path = ?client.root_dir(), "Server build completed successfully! 🚀");
        }

        // Fill platform-specific defaults for package types when omitted.
        if self.package_types.is_none() {
            match client.bundle {
                BundleFormat::Ios => {
                    self.package_types = Some(vec![crate::PackageType::IosBundle]);
                }
                BundleFormat::Android => {
                    self.package_types = Some(vec![crate::PackageType::Aab]);
                }
                _ => {}
            }
        }

        Self::validate_package_types_for_bundle(client.bundle, self.package_types.as_deref())?;

        let mut bundles = vec![];

        // Copy the server over if it exists
        if let Some(server) = server.as_ref() {
            bundles.push(server.main_exe());
        }

        // Create a list of bundles that we might need to copy.
        // Package-type based bundling is handled by the bundler module.
        match client.bundle {
            // Desktop and Android platforms use package-type dispatch in the bundler module.
            BundleFormat::MacOS
            | BundleFormat::Linux
            | BundleFormat::Windows
            | BundleFormat::Android => {
                tracing::info!("Running package bundler...");
                for bundle in Self::bundle_with_package_types(&client, &self.package_types).await? {
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

    async fn bundle_with_package_types(
        build: &BuildRequest,
        package_types: &Option<Vec<PackageType>>,
    ) -> Result<Vec<crate::bundler::Bundle>, Error> {
        use anyhow::bail;

        if matches!(
            build.bundle,
            BundleFormat::MacOS | BundleFormat::Linux | BundleFormat::Windows
        ) {
            let krate = &build;
            let exe = build.main_exe();

            _ = std::fs::remove_dir_all(krate.bundle_dir(build.bundle));

            let mut name: PathBuf = krate.executable_name().into();
            if cfg!(windows) {
                name.set_extension("exe");
            }
            std::fs::create_dir_all(krate.bundle_dir(build.bundle))
                .context("Failed to create bundle directory")?;
            std::fs::copy(&exe, krate.bundle_dir(build.bundle).join(&name)).with_context(|| {
                "Failed to copy the output executable into the bundle directory"
            })?;

            // Check if required fields are provided instead of failing silently.
            if build.config.bundle.identifier.is_none() {
                bail!("\n\nBundle identifier was not provided in `Dioxus.toml`. Add it as:\n\n[bundle]\nidentifier = \"com.mycompany\"\n\n");
            }
            if build.config.bundle.publisher.is_none() {
                bail!("\n\nBundle publisher was not provided in `Dioxus.toml`. Add it as:\n\n[bundle]\npublisher = \"MyCompany\"\n\n");
            }
        }

        let ctx = crate::bundler::BundleContext::new(build, package_types).await?;

        tracing::debug!("Bundling project for {:?}", ctx.package_types());

        let bundles = ctx.bundle_project().await.inspect_err(|err| {
            tracing::error!("Failed to bundle project: {:#?}", err);
            if cfg!(target_os = "macos") {
                tracing::error!("Make sure you have automation enabled in your terminal (https://github.com/tauri-apps/tauri/issues/3055#issuecomment-1624389208) and full disk access enabled for your terminal (https://github.com/tauri-apps/tauri/issues/3055#issuecomment-1624389208)");
            }
        })?;

        Ok(bundles)
    }

    fn validate_package_types_for_bundle(
        bundle: BundleFormat,
        package_types: Option<&[PackageType]>,
    ) -> Result<(), Error> {
        let Some(package_types) = package_types else {
            return Ok(());
        };

        let is_package_supported = |package_type: &PackageType| -> bool {
            match bundle {
                BundleFormat::MacOS => matches!(
                    package_type,
                    PackageType::MacOsBundle | PackageType::Dmg | PackageType::Updater
                ),
                BundleFormat::Linux => matches!(
                    package_type,
                    PackageType::Deb
                        | PackageType::Rpm
                        | PackageType::AppImage
                        | PackageType::Updater
                ),
                BundleFormat::Windows => {
                    matches!(
                        package_type,
                        PackageType::WindowsMsi | PackageType::Nsis | PackageType::Updater
                    )
                }
                BundleFormat::Android => {
                    matches!(package_type, PackageType::Apk | PackageType::Aab)
                }
                BundleFormat::Ios => matches!(package_type, PackageType::IosBundle),
                BundleFormat::Web | BundleFormat::Server => false,
            }
        };

        if let Some(invalid) = package_types.iter().find(|pt| !is_package_supported(pt)) {
            anyhow::bail!(
                "Package type '{invalid:?}' is not supported for bundle format '{bundle}'."
            );
        }

        Ok(())
    }
}
