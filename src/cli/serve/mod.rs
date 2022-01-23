use crate::{cfg::ConfigOptsServe, gen_page, server, CrateConfig};
use std::{io::Write, path::PathBuf};
use structopt::StructOpt;

/// Run the WASM project on dev-server
#[derive(Clone, Debug, StructOpt)]
#[structopt(name = "serve")]
pub struct Serve {
    #[structopt(flatten)]
    pub serve: ConfigOptsServe,
}

impl Serve {
    pub async fn serve(self) -> anyhow::Result<()> {
        let mut crate_config = crate::CrateConfig::new()?;
        // change the relase state.
        crate_config.with_release(self.serve.release);

        crate::builder::build(&crate_config).expect("build failed");

        // generate dev-index page
        Serve::regen_dev_page(&crate_config)?;

        // start the develop server
        server::startup(crate_config.clone()).await?;

        Ok(())
    }

    pub fn regen_dev_page(crate_config: &CrateConfig) -> anyhow::Result<()> {
        let serve_html = gen_page(&crate_config.dioxus_config, true);

        let mut file = std::fs::File::create(
            crate_config
                .crate_dir
                .join(
                    crate_config
                        .dioxus_config
                        .web
                        .app
                        .out_dir
                        .clone()
                        .unwrap_or(PathBuf::from("dist")),
                )
                .join("index.html"),
        )?;
        file.write_all(serve_html.as_bytes())?;

        Ok(())
    }
}
