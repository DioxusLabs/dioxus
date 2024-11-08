use crate::Builder;
use crate::DioxusCrate;
use crate::{build::BuildArgs, PackageType};
use anyhow::Context;
use itertools::Itertools;
use std::{collections::HashMap, str::FromStr};
use tauri_bundler::{BundleBinary, BundleSettings, PackageSettings, SettingsBuilder};

use super::*;

/// Bundle the Rust desktop app and all of its assets
#[derive(Clone, Debug, Parser)]
#[clap(name = "bundle")]
pub struct Bundle {
    /// The package types to bundle
    #[clap(long)]
    pub packages: Option<Vec<PackageType>>,

    /// The arguments for the dioxus build
    #[clap(flatten)]
    pub(crate) build_arguments: BuildArgs,
}

impl Bundle {
    pub(crate) async fn bundle(mut self) -> Result<()> {
        tracing::info!("Bundling project...");

        let krate = DioxusCrate::new(&self.build_arguments.target_args)
            .context("Failed to load Dioxus workspace")?;

        self.build_arguments.resolve(&krate)?;

        tracing::info!("Building app...");

        let bundle = Builder::start(&krate, self.build_arguments.clone())?
            .finish()
            .await?;

        tracing::info!("Copying app to output directory...");

        _ = std::fs::remove_dir_all(krate.bundle_dir(self.build_arguments.platform()));

        let package = krate.package();
        let mut name: PathBuf = krate.executable_name().into();
        if cfg!(windows) {
            name.set_extension("exe");
        }

        // Make sure we copy the exe to the bundle dir so the bundler can find it
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
            let new = PathBuf::from("assets").join(old.file_name().unwrap());

            bundle_settings
                .resources_map
                .as_mut()
                .expect("to be set")
                .insert(old.display().to_string(), new.display().to_string());
        }

        // Drain any resources set in the config into the resources map. Tauri bundle doesn't let
        // you set both resources and resources_map https://github.com/DioxusLabs/dioxus/issues/2941
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

        if let Some(packages) = &self.packages {
            settings = settings.package_types(packages.iter().map(|p| (*p).into()).collect());
        }

        if let Some(target) = self.build_arguments.target_args.target.as_ref() {
            settings = settings.target(target.to_string());
        }

        let settings = settings.build()?;

        tracing::debug!("Bundling project with settings: {:#?}", settings);

        // on macos we need to set CI=true (https://github.com/tauri-apps/tauri/issues/2567)
        if cfg!(target_os = "macos") {
            std::env::set_var("CI", "true");
        }

        let bundles = tauri_bundler::bundle::bundle_project(&settings).inspect_err(|err| {
            tracing::error!("Failed to bundle project: {:#?}", err);
            if cfg!(target_os = "macos") {
                tracing::error!("Make sure you have automation enabled in your terminal (https://github.com/tauri-apps/tauri/issues/3055#issuecomment-1624389208) and full disk access enabled for your terminal (https://github.com/tauri-apps/tauri/issues/3055#issuecomment-1624389208)");
            }
        })?;

        tracing::info!("Bundled app successfully!");
        tracing::info!("App produced {} outputs:", bundles.len());

        for bundle in bundles {
            tracing::info!(
                "{} - [{}]",
                bundle.package_type.short_name(),
                bundle.bundle_paths.iter().map(|p| p.display()).join(", ")
            );
        }

        Ok(())
    }
}

impl FromStr for PackageType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "macos" => Ok(PackageType::MacOsBundle),
            "ios" => Ok(PackageType::IosBundle),
            "msi" => Ok(PackageType::WindowsMsi),
            "deb" => Ok(PackageType::Deb),
            "rpm" => Ok(PackageType::Rpm),
            "appimage" => Ok(PackageType::AppImage),
            "dmg" => Ok(PackageType::Dmg),
            "updater" => Ok(PackageType::Updater),
            _ => Err(format!("{} is not a valid package type", s)),
        }
    }
}
