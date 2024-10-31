use crate::bundle_utils::make_tauri_bundler_settings;
use crate::DioxusCrate;
use crate::{build::BuildArgs, PackageType};
use anyhow::Context;
use std::env::current_dir;
use std::str::FromStr;
use tauri_bundler::{PackageSettings, SettingsBuilder};

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
    pub(crate) async fn bundle(mut self) -> anyhow::Result<()> {
        let krate = DioxusCrate::new(&self.build_arguments.target_args)
            .context("Failed to load Dioxus workspace")?;

        self.build_arguments.resolve(&krate)?;

        // Build the app
        let bundle = self.build_arguments.build().await?;

        // copy the binary to the out dir
        let package = krate.package();

        let mut name: PathBuf = krate.executable_name().into();
        if cfg!(windows) {
            name.set_extension("exe");
        }

        // bundle the app
        let binaries = vec![
            tauri_bundler::BundleBinary::new(name.display().to_string(), true)
                .set_src_path(Some(krate.workspace_dir().display().to_string())),
        ];

        let bundle_config = krate.config.bundle.clone();
        let mut bundle_settings = make_tauri_bundler_settings(bundle_config);

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

        // Don't copy the executable or the old bundle directory
        let ignored_files = [krate
            .bundle_dir(self.build_arguments.platform())
            .join("bundle")];

        for entry in std::fs::read_dir(bundle.asset_dir())?.flatten() {
            let path = entry.path().canonicalize()?;
            if ignored_files.iter().any(|f| path.starts_with(f)) {
                continue;
            }

            // Tauri bundle will add a __root__ prefix if the input path is absolute even though the output path is relative?
            // We strip the prefix here to make sure the input path is relative so that the bundler puts the output path in the right place
            let path = path
                .strip_prefix(&current_dir()?)
                .unwrap()
                .display()
                .to_string();
            if let Some(resources) = &mut bundle_settings.resources_map {
                resources.insert(path, "".to_string());
            } else {
                bundle_settings.resources_map = Some([(path, "".to_string())].into());
            }
        }

        // Drain any resources set in the config into the resources map. Tauri bundle doesn't let you set both resources and resources_map https://github.com/DioxusLabs/dioxus/issues/2941
        for resource_path in bundle_settings.resources.take().into_iter().flatten() {
            if let Some(resources) = &mut bundle_settings.resources_map {
                resources.insert(resource_path, "".to_string());
            } else {
                bundle_settings.resources_map = Some([(resource_path, "".to_string())].into());
            }
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
            .binaries(binaries)
            .bundle_settings(bundle_settings);
        if let Some(packages) = &self.packages {
            settings = settings.package_types(packages.iter().map(|p| (*p).into()).collect());
        }

        if let Some(target) = self.build_arguments.target_args.target.as_ref() {
            settings = settings.target(target.to_string());
        }

        let settings = settings.build();

        // on macos we need to set CI=true (https://github.com/tauri-apps/tauri/issues/2567)
        #[cfg(target_os = "macos")]
        std::env::set_var("CI", "true");

        tauri_bundler::bundle::bundle_project(&settings.unwrap()).unwrap_or_else(|err|{
            #[cfg(target_os = "macos")]
            panic!("Failed to bundle project: {:#?}\nMake sure you have automation enabled in your terminal (https://github.com/tauri-apps/tauri/issues/3055#issuecomment-1624389208) and full disk access enabled for your terminal (https://github.com/tauri-apps/tauri/issues/3055#issuecomment-1624389208)", err);
            #[cfg(not(target_os = "macos"))]
            panic!("Failed to bundle project: {:#?}", err);
        });

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
