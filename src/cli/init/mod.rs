use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use structopt::StructOpt;

use crate::error::{Error, Result};

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, StructOpt)]
#[structopt(name = "init")]
pub struct Init {
    /// Init project path
    #[structopt(default_value = ".")]
    path: String,

    /// Template path
    #[structopt(default_value = "default", long)]
    template: String,
}

impl Init {
    pub fn init(self) -> Result<()> {
        log::info!("🔧 Start to init a new project '{}'.", self.path);

        let project_path = PathBuf::from(&self.path);

        if project_path.join("Cargo.toml").is_file() {
            log::warn!("Path '{}' is initialized.", self.path);
            return Ok(());
        }

        let output = Command::new("cargo")
            .arg("init")
            .arg(&self.path)
            .arg("--bin")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            return Err(Error::CargoError("Cargo init failed".into()));
        }

        // get the template code
        let template_str = match self.template {
            _ => include_str!("../../template/default.rs"),
        };

        let main_rs_file = project_path.join("src").join("main.rs");
        if !main_rs_file.is_file() {
            return Err(Error::FailedToWrite);
        }

        let mut file = File::create(main_rs_file)?;
        file.write_all(&template_str.as_bytes())?;

        // log::info!("🎯 Project initialization completed.");

        if !Command::new("cargo")
            .args(["add", "dioxus", "--features web"])
            .output()?
            .status
            .success()
        {
            let mut file = OpenOptions::new()
                .append(true)
                .open(project_path.join("Cargo.toml"))?;
            file.write_all("dioxus = { version = \"0.1.7\", features = [\"web\"] }".as_bytes())?;
        }

        log::info!("\n💡 Project initialized:");
        log::info!("🎯> cd {}", self.path);
        log::info!("🎯> dioxus serve");

        Ok(())
    }
}
