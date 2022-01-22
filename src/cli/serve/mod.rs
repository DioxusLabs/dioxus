use crate::{cfg::ConfigOptsServe, server};
use anyhow::Result;
use std::path::PathBuf;
use std::io::Write;
use structopt::StructOpt;

mod develop;

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

        let serve_html = String::from(include_str!("../../server/serve.html"));

        let mut file = std::fs::File::create(crate_config.out_dir.join("index.html"))?;
        file.write_all(serve_html.as_bytes())?;

        // start the develop server
        server::startup(crate_config.clone()).await?;

        Ok(())
    }
}
