use std::{io::Write, path::PathBuf, process::Command, fs::copy};

use crate::{cfg::ConfigOptsBuild, gen_page};
use std::fs::remove_dir_all;
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
                log::info!("🚅 Running build command...");

                let mut cmd = Command::new("cargo");
                cmd.current_dir(&crate_config.crate_dir)
                    .arg("build")
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit());

                if self.build.release {
                    cmd.arg("--release");
                }

                match &crate_config.executable {
                    crate::ExecutableType::Binary(name) => cmd.arg("--bin").arg(name),
                    crate::ExecutableType::Lib(name) => cmd.arg("--lib").arg(name),
                    crate::ExecutableType::Example(name) => cmd.arg("--example").arg(name),
                };

                let output = cmd.output()?;

                if output.status.success() {
                    if crate_config.out_dir.is_dir() {

                        remove_dir_all(&crate_config.out_dir)?;

                        let release_type = match crate_config.release {
                            true => "release",
                            false => "debug",
                        };
                    
                        let mut res_path = match &crate_config.executable {
                            crate::ExecutableType::Binary(name)
                            | crate::ExecutableType::Lib(name) => crate_config
                                .target_dir
                                .join(format!("{}", release_type))
                                .join(format!("{}", name)),

                            crate::ExecutableType::Example(name) => crate_config
                                .target_dir
                                .join(format!("{}/examples", release_type))
                                .join(format!("{}", name)),
                        };

                        let target_file;
                        if cfg!(windows) {
                            res_path.set_extension("exe");
                            target_file = format!("{}.exe", &crate_config.dioxus_config.application.name);
                        } else {
                            target_file = crate_config.dioxus_config.application.name.clone();
                        }
                        copy(res_path, &crate_config.out_dir.join(target_file))?;
                    }
                }

                return Ok(());
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
                        .web
                        .app
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
