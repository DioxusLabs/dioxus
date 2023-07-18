use crate::pipeline::{
    index_file::IndexFile, pull_assets::PullAssets, wasm_build::WasmBuild, web_out::WebOut,
    BuildConfig, CrateInfo, Pipeline, PipelineConfig, sass::SassBuild,
};

#[cfg(feature = "plugin")]
use crate::plugin::PluginManager;

use super::*;

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, Parser)]
#[clap(name = "build")]
pub struct Build {
    #[clap(flatten)]
    pub build: ConfigOptsBuild,
}

impl Build {
    pub fn build(self, bin: Option<PathBuf>) -> Result<()> {
        let config = PipelineConfig::new(
            CrateInfo::from_toml(bin)?,
            BuildConfig::new(
                self.build.release,
                self.build.verbose,
                self.build.features.unwrap_or(Vec::new()),
            ),
        );

        Pipeline::new(config)
            .with_step(PullAssets::new())
            .with_step(SassBuild::new())
            .with_step(WasmBuild::new())
            .with_step(IndexFile::new())
            .with_step(WebOut::new())
            .run()?;

        /*// change the release state.
        crate_config.with_release(self.build.release);
        crate_config.with_verbose(self.build.verbose);

        if self.build.example.is_some() {
            crate_config.as_example(self.build.example.unwrap());
        }

        if self.build.profile.is_some() {
            crate_config.set_profile(self.build.profile.unwrap());
        }

        if self.build.features.is_some() {
            crate_config.set_features(self.build.features.unwrap());
        }

        let platform = self.build.platform.unwrap_or_else(|| {
            crate_config
                .dioxus_config
                .application
                .default_platform
                .clone()
        });

        #[cfg(feature = "plugin")]
        let _ = PluginManager::on_build_start(&crate_config, &platform);

        match platform.as_str() {
            "web" => {
                crate::builder::build(&crate_config, false)?;
            }
            "desktop" => {
                crate::builder::build_desktop(&crate_config, false)?;
            }
            _ => {
                return custom_error!("Unsupported platform target.");
            }
        }

        let temp = gen_page(&crate_config.dioxus_config, false);

        let mut file = std::fs::File::create(
            crate_config
                .crate_dir
                .join(
                    crate_config
                        .dioxus_config
                        .application
                        .out_dir
                        .clone()
                        .unwrap_or_else(|| PathBuf::from("dist")),
                )
                .join("index.html"),
        )?;
        file.write_all(temp.as_bytes())?;

        #[cfg(feature = "plugin")]
        let _ = PluginManager::on_build_finish(&crate_config, &platform);*/

        Ok(())
    }
}
