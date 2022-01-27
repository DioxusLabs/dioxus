use crate::{cfg::ConfigOptsServe, gen_page, server, CrateConfig};
use std::{io::Write, path::PathBuf, process::{Command, Stdio}};
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

        if self.serve.example.is_some() {
            crate_config.as_example(self.serve.example.unwrap());
        }

        match self.serve.platform.as_str() {
            "web" => {
                crate::builder::build(&crate_config)?;
            }
            "desktop" => {
                crate::builder::build_desktop(&crate_config)?;

                match &crate_config.executable {
                    crate::ExecutableType::Binary(name)
                    | crate::ExecutableType::Lib(name)
                    | crate::ExecutableType::Example(name) => {
                        let mut file = crate_config.out_dir.join(name);
                        if cfg!(windows) {
                            file.set_extension("exe");
                        }
                        Command::new(
                            crate_config
                                .out_dir
                                .join(file)
                                .to_str()
                                .unwrap()
                                .to_string(),
                        )
                        .stdout(Stdio::inherit())
                        .output()?;
                    }
                }
                return Ok(());
            }
            _ => {
                return Err(anyhow::anyhow!("Unsoppurt platform target."));
            }
        }

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
                        .application
                        .out_dir
                        .clone()
                        .unwrap_or_else(|| PathBuf::from("dist")),
                )
                .join("index.html"),
        )?;
        file.write_all(serve_html.as_bytes())?;

        Ok(())
    }
}
