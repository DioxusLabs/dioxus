use dioxus_cli_config::Platform;
use manganis_cli_support::AssetManifest;

use super::*;
use crate::plugin::interface::plugins::main::types::CommandEvent::Serve as ServeEvent;
use cargo_toml::Dependency::{Detailed, Inherited, Simple};
use std::{fs::create_dir_all, io::Write, path::PathBuf};

/// Run the WASM project on dev-server
#[derive(Clone, Debug, Parser)]
#[clap(name = "serve")]
pub struct Serve {
    #[clap(flatten)]
    pub serve: ConfigOptsServe,
}

impl Serve {
    pub async fn serve(self, bin: Option<PathBuf>) -> Result<()> {
        let mut crate_config = dioxus_cli_config::CrateConfig::new(bin)?;
        let serve_cfg = self.serve.clone();

        // change the release state.
        let hot_reload = self.serve.hot_reload || crate_config.dioxus_config.application.hot_reload;
        crate_config.with_hot_reload(hot_reload);
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

        plugins_before_command(ServeEvent).await;

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
            Platform::Web => web::startup(crate_config.clone(), &serve_cfg).await?,
            Platform::Desktop => desktop::startup(crate_config.clone(), &serve_cfg).await?,
            Platform::Fullstack | Platform::StaticGeneration => {
                fullstack::startup(crate_config.clone(), &serve_cfg).await?
            }
            _ => unreachable!(),
        }

        plugins_after_command(ServeEvent).await;

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
