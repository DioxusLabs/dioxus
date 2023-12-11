use super::*;
use crate::{
    call_plugins, cfg::Platform,
    plugin::interface::plugins::main::types::CompileEvent::Build as BuildEvent,
};

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, Parser)]
#[clap(name = "build")]
pub struct Build {
    #[clap(flatten)]
    pub build: ConfigOptsBuild,
}

impl Build {
    pub async fn build(self, bin: Option<PathBuf>) -> Result<()> {
        let mut crate_config = crate::CrateConfig::new(bin)?;

        // change the release state.
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

        let platform = self
            .build
            .platform
            .unwrap_or(crate_config.dioxus_config.application.default_platform);

        call_plugins!(before_compile_event BuildEvent);

        match platform {
            Platform::Web => {
                crate::builder::build(&crate_config, true)?;
            }
            Platform::Desktop => {
                crate::builder::build_desktop(&crate_config, false)?;
            }
        }

        call_plugins!(after_compile_event BuildEvent);

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

        Ok(())
    }
}
