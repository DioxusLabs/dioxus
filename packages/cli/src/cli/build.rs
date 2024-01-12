use crate::assets::WebAssetConfigDropGuard;
#[cfg(feature = "plugin")]
use crate::plugin::PluginManager;
use crate::server::fullstack::FullstackServerEnvGuard;
use crate::server::fullstack::FullstackWebEnvGuard;
use dioxus_cli_config::Platform;

use super::*;

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, Parser)]
#[clap(name = "build")]
pub struct Build {
    #[clap(flatten)]
    pub build: ConfigOptsBuild,
}

impl Build {
    pub fn build(self, bin: Option<PathBuf>, target_dir: Option<&std::path::Path>) -> Result<()> {
        let mut crate_config = dioxus_cli_config::CrateConfig::new(bin)?;
        if let Some(target_dir) = target_dir {
            crate_config.target_dir = target_dir.to_path_buf();
        }

        // change the release state.
        crate_config.with_release(self.build.release);
        crate_config.with_verbose(self.build.verbose);

        if self.build.example.is_some() {
            crate_config.as_example(self.build.example.clone().unwrap());
        }

        if self.build.profile.is_some() {
            crate_config.set_profile(self.build.profile.clone().unwrap());
        }

        if self.build.features.is_some() {
            crate_config.set_features(self.build.features.clone().unwrap());
        }

        let platform = self
            .build
            .platform
            .unwrap_or(crate_config.dioxus_config.application.default_platform);

        if let Some(target) = self.build.target.clone() {
            crate_config.set_target(target);
        }

        crate_config.set_cargo_args(self.build.cargo_args.clone());

        // #[cfg(feature = "plugin")]
        // let _ = PluginManager::on_build_start(&crate_config, &platform);

        let build_result = match platform {
            Platform::Web => crate::builder::build(&crate_config, false, self.build.skip_assets)?,
            Platform::Desktop => {
                crate::builder::build_desktop(&crate_config, false, self.build.skip_assets)?
            }
            Platform::Fullstack => {
                // Fullstack mode must be built with web configs on the desktop (server) binary as well as the web binary
                let _config = WebAssetConfigDropGuard::new();
                {
                    let mut web_config = crate_config.clone();
                    let _gaurd = FullstackWebEnvGuard::new(&self.build);
                    let web_feature = self.build.client_feature;
                    let features = &mut web_config.features;
                    match features {
                        Some(features) => {
                            features.push(web_feature);
                        }
                        None => web_config.features = Some(vec![web_feature]),
                    };
                    crate::builder::build(&crate_config, false, self.build.skip_assets)?;
                }
                {
                    let mut desktop_config = crate_config.clone();
                    let desktop_feature = self.build.server_feature;
                    let features = &mut desktop_config.features;
                    match features {
                        Some(features) => {
                            features.push(desktop_feature);
                        }
                        None => desktop_config.features = Some(vec![desktop_feature]),
                    };
                    let _gaurd =
                        FullstackServerEnvGuard::new(self.build.force_debug, self.build.release);
                    crate::builder::build_desktop(&desktop_config, false, self.build.skip_assets)?
                }
            }
        };

        let temp = gen_page(&crate_config, build_result.assets.as_ref(), false);

        let mut file = std::fs::File::create(
            crate_config
                .crate_dir
                .join(crate_config.dioxus_config.application.out_dir.clone())
                .join("index.html"),
        )?;
        file.write_all(temp.as_bytes())?;

        // #[cfg(feature = "plugin")]
        // let _ = PluginManager::on_build_finish(&crate_config, &platform);

        Ok(())
    }
}
