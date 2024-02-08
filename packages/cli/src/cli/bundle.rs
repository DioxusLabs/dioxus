use core::panic;
use dioxus_cli_config::ExecutableType;
use std::{fs::create_dir_all, str::FromStr};

use tauri_bundler::{BundleSettings, PackageSettings, SettingsBuilder};

use super::*;
use crate::{build_desktop, cfg::ConfigOptsBundle};

/// Bundle the Rust desktop app and all of its assets
#[derive(Clone, Debug, Parser)]
#[clap(name = "bundle")]
pub struct Bundle {
    #[clap(long)]
    pub package: Option<Vec<String>>,
    #[clap(flatten)]
    pub build: ConfigOptsBundle,
}

#[derive(Clone, Debug)]
pub enum PackageType {
    MacOsBundle,
    IosBundle,
    WindowsMsi,
    Deb,
    Rpm,
    AppImage,
    Dmg,
    Updater,
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
            _ => Err(format!("{} is not a valid package type", s)),
        }
    }
}

impl From<PackageType> for tauri_bundler::PackageType {
    fn from(val: PackageType) -> Self {
        match val {
            PackageType::MacOsBundle => tauri_bundler::PackageType::MacOsBundle,
            PackageType::IosBundle => tauri_bundler::PackageType::IosBundle,
            PackageType::WindowsMsi => tauri_bundler::PackageType::WindowsMsi,
            PackageType::Deb => tauri_bundler::PackageType::Deb,
            PackageType::Rpm => tauri_bundler::PackageType::Rpm,
            PackageType::AppImage => tauri_bundler::PackageType::AppImage,
            PackageType::Dmg => tauri_bundler::PackageType::Dmg,
            PackageType::Updater => tauri_bundler::PackageType::Updater,
        }
    }
}

impl Bundle {
    pub fn bundle(self, bin: Option<PathBuf>) -> Result<()> {
        let mut crate_config = dioxus_cli_config::CrateConfig::new(bin)?;

        // change the release state.
        crate_config.with_release(self.build.release);
        crate_config.with_verbose(self.build.verbose);

        if self.build.example.is_some() {
            crate_config.as_example(self.build.example.unwrap());
        }

        if self.build.profile.is_some() {
            crate_config.set_profile(self.build.profile.unwrap());
        }

        if let Some(target) = &self.build.target {
            crate_config.set_target(target.to_string());
        }

        crate_config.set_cargo_args(self.build.cargo_args);

        // build the desktop app
        // Since the `bundle()` function is only run for the desktop platform,
        // the `rust_flags` argument is set to `None`.
        build_desktop(&crate_config, false, false, None)?;

        // copy the binary to the out dir
        let package = crate_config.manifest.package.as_ref().unwrap();

        let mut name: PathBuf = match &crate_config.executable {
            ExecutableType::Binary(name)
            | ExecutableType::Lib(name)
            | ExecutableType::Example(name) => name,
        }
        .into();
        if cfg!(windows) {
            name.set_extension("exe");
        }

        // bundle the app
        let binaries = vec![
            tauri_bundler::BundleBinary::new(name.display().to_string(), true)
                .set_src_path(Some(crate_config.crate_dir.display().to_string())),
        ];

        let mut bundle_settings: BundleSettings = crate_config.dioxus_config.bundle.clone().into();
        if cfg!(windows) {
            let windows_icon_override = crate_config
                .dioxus_config
                .bundle
                .windows
                .as_ref()
                .map(|w| &w.icon_path);
            if windows_icon_override.is_none() {
                let icon_path = bundle_settings
                    .icon
                    .as_ref()
                    .and_then(|icons| icons.first());
                let icon_path = if let Some(icon_path) = icon_path {
                    icon_path.into()
                } else {
                    let path = PathBuf::from("./icons/icon.ico");
                    // create the icon if it doesn't exist
                    if !path.exists() {
                        create_dir_all(path.parent().unwrap()).unwrap();
                        let mut file = File::create(&path).unwrap();
                        file.write_all(include_bytes!("../assets/icon.ico"))
                            .unwrap();
                    }
                    path
                };
                bundle_settings.windows.icon_path = icon_path;
            }
        }

        // Add all assets from collect assets to the bundle
        {
            let config = manganis_cli_support::Config::current();
            let location = config.assets_serve_location().to_string();
            let location = format!("./{}", location);
            println!("Adding assets from {} to bundle", location);
            if let Some(resources) = &mut bundle_settings.resources {
                resources.push(location);
            } else {
                bundle_settings.resources = Some(vec![location]);
            }
        }

        let mut settings = SettingsBuilder::new()
            .project_out_directory(crate_config.out_dir())
            .package_settings(PackageSettings {
                product_name: crate_config.dioxus_config.application.name.clone(),
                version: package.version().to_string(),
                description: package.description().unwrap_or_default().to_string(),
                homepage: Some(package.homepage().unwrap_or_default().to_string()),
                authors: Some(Vec::from(package.authors())),
                default_run: Some(crate_config.dioxus_config.application.name.clone()),
            })
            .binaries(binaries)
            .bundle_settings(bundle_settings);
        if let Some(packages) = self.package {
            settings = settings.package_types(
                packages
                    .into_iter()
                    .map(|p| p.parse::<PackageType>().unwrap().into())
                    .collect(),
            );
        }

        if let Some(target) = &self.build.target {
            settings = settings.target(target.to_string());
        }

        let settings = settings.build();

        // on macos we need to set CI=true (https://github.com/tauri-apps/tauri/issues/2567)
        #[cfg(target_os = "macos")]
        std::env::set_var("CI", "true");

        tauri_bundler::bundle::bundle_project(settings.unwrap()).unwrap_or_else(|err|{
            #[cfg(target_os = "macos")]
            panic!("Failed to bundle project: {:#?}\nMake sure you have automation enabled in your terminal (https://github.com/tauri-apps/tauri/issues/3055#issuecomment-1624389208) and full disk access enabled for your terminal (https://github.com/tauri-apps/tauri/issues/3055#issuecomment-1624389208)", err);
            #[cfg(not(target_os = "macos"))]
            panic!("Failed to bundle project: {:#?}", err);
        });

        Ok(())
    }
}
