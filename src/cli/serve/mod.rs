use super::*;
use std::{
    fs::create_dir_all,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

/// Run the WASM project on dev-server
#[derive(Clone, Debug, Parser)]
#[clap(name = "serve")]
pub struct Serve {
    #[clap(flatten)]
    pub serve: ConfigOptsServe,
}

impl Serve {
    pub async fn serve(self) -> Result<()> {
        let mut crate_config = crate::CrateConfig::new()?;

        // change the relase state.
        crate_config.with_hot_reload(self.serve.hot_reload);
        crate_config.with_cross_origin_policy(self.serve.cross_origin_policy);
        crate_config.with_release(self.serve.release);
        crate_config.with_verbose(self.serve.verbose);

        if self.serve.example.is_some() {
            crate_config.as_example(self.serve.example.unwrap());
        }

        if self.serve.profile.is_some() {
            crate_config.set_profile(self.serve.profile.unwrap());
        }
        
        if self.serve.features.is_some() {
            crate_config.set_features(self.serve.features.unwrap());
        }

        let platform = self.serve.platform.unwrap_or_else(|| {
            crate_config
                .dioxus_config
                .application
                .default_platform
                .clone()
        });

        if platform.as_str() == "desktop" {
            crate::builder::build_desktop(&crate_config, true)?;

            match &crate_config.executable {
                crate::ExecutableType::Binary(name)
                | crate::ExecutableType::Lib(name)
                | crate::ExecutableType::Example(name) => {
                    let mut file = crate_config.out_dir.join(name);
                    if cfg!(windows) {
                        file.set_extension("exe");
                    }
                    Command::new(file.to_str().unwrap())
                        .stdout(Stdio::inherit())
                        .output()?;
                }
            }
            return Ok(());
        } else if platform != "web" {
            return custom_error!("Unsupported platform target.");
        }

        // generate dev-index page
        Serve::regen_dev_page(&crate_config)?;

        // start the develop server
        server::startup(self.serve.port, crate_config.clone(), self.serve.open).await?;

        Ok(())
    }

    pub fn regen_dev_page(crate_config: &CrateConfig) -> Result<()> {
        let serve_html = gen_page(&crate_config.dioxus_config, true);

        let dist_path = crate_config.crate_dir.join(
            crate_config
                .dioxus_config
                .application
                .out_dir
                .clone()
                .unwrap_or_else(|| PathBuf::from("dist")),
        );
        if !dist_path.is_dir() {
            create_dir_all(&dist_path)?;
        }
        let index_path = dist_path.join("index.html");
        let mut file = std::fs::File::create(index_path)?;
        file.write_all(serve_html.as_bytes())?;

        Ok(())
    }
}
