use std::{io::Write, path::PathBuf};

use crate::{cfg::ConfigOptsBuild, gen_page};
use structopt::StructOpt;

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, StructOpt)]
#[structopt(name = "build")]
pub struct Build {
    #[structopt(flatten)]
    pub build: ConfigOptsBuild,
}

impl Build {
    pub fn build(self) -> anyhow::Result<()> {
        let mut crate_config = crate::CrateConfig::new()?;

        // change the relase state.
        crate_config.with_release(self.build.release);

        if self.build.example.is_some() {
            crate_config.as_example(self.build.example.unwrap());
        }

        if self.build.platform.is_some() {
            if self.build.platform.unwrap().to_uppercase() == "DESKTOP" {
                crate::builder::build_desktop(&crate_config)?;
            }
        }

        crate::builder::build(&crate_config)?;

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
                        .unwrap_or(PathBuf::from("dist")),
                )
                .join("index.html"),
        )?;
        file.write_all(temp.as_bytes())?;

        Ok(())
    }
}
