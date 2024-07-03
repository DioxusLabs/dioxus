use dioxus_cli_config::Platform;
use manganis_cli_support::AssetManifest;

use super::*;
use cargo_toml::Dependency::{Detailed, Inherited, Simple};
use std::fs::create_dir_all;

/// Run the WASM project on dev-server
#[derive(Clone, Debug, Parser)]
#[clap(name = "serve")]
pub struct Serve {
    #[clap(flatten)]
    pub serve: ConfigOptsServe,
}

impl Serve {
    pub fn serve(self, bin: Option<PathBuf>) -> Result<()> {
        let mut crate_config = dioxus_cli_config::CrateConfig::new(bin)?;
        let mut serve_cfg = self.serve.clone();

        // Handle cli settings
        let cli_settings = crate_config.dioxus_config.cli_settings.clone().unwrap();

        if serve_cfg.hot_reload.is_none() {
            // we're going to override the hot_reload setting in the project's cfg based on settings
            //
            // let hot_reload = self.serve.hot_reload || crate_config.dioxus_config.application.hot_reload;

            let value = cli_settings.always_hot_reload.unwrap_or(true);
            serve_cfg.hot_reload = Some(value);
            crate_config.with_hot_reload(value);
        }

        if serve_cfg.open.is_none() {
            serve_cfg.open = Some(cli_settings.always_open_browser.unwrap_or(false));
        }

        // Set config settings
        crate_config.with_cross_origin_policy(self.serve.cross_origin_policy);
        crate_config.with_release(self.serve.release);
        crate_config.with_verbose(self.serve.verbose);

        if let Some(example) = self.serve.example {
            crate_config.as_example(example);
        }

        if let Some(profile) = self.serve.profile {
            crate_config.set_profile(profile);
        }

        if let Some(features) = self.serve.features {
            crate_config.set_features(features);
        }

        if let Some(target) = self.serve.target {
            crate_config.set_target(target);
        }

        crate_config.set_cargo_args(self.serve.cargo_args);

        let mut platform = self.serve.platform;

        if platform.is_none() {
            if let Some(dependency) = &crate_config.manifest.dependencies.get("dioxus") {
                let features = match dependency {
                    Inherited(detail) => detail.features.to_vec(),
                    Detailed(detail) => detail.features.to_vec(),
                    Simple(_) => vec![],
                };

                platform = features
                    .iter()
                    .find_map(|platform| serde_json::from_str(&format!(r#""{}""#, platform)).ok());
            }
        }

        let platform = platform.unwrap_or(crate_config.dioxus_config.application.default_platform);
        crate_config.extend_with_platform(platform);

        // start the develop server
        use server::{desktop, fullstack, web};
        match platform {
            Platform::Web => web::startup(crate_config.clone(), &serve_cfg)?,
            Platform::Desktop => desktop::startup(crate_config.clone(), &serve_cfg)?,
            Platform::Fullstack | Platform::StaticGeneration => {
                fullstack::startup(crate_config.clone(), &serve_cfg)?
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    pub fn regen_dev_page(
        crate_config: &CrateConfig,
        manifest: Option<&AssetManifest>,
    ) -> anyhow::Result<()> {
        let serve_html = gen_page(crate_config, manifest, true);

        let dist_path = crate_config.out_dir();
        if !dist_path.is_dir() {
            create_dir_all(&dist_path)?;
        }
        let index_path = dist_path.join("index.html");
        let mut file = std::fs::File::create(index_path)?;
        file.write_all(serve_html.as_bytes())?;

        Ok(())
    }
}
