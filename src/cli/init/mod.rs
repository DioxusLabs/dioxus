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
    /// Init project name
    #[structopt(default_value = ".")]
    name: String,

    /// Template path
    #[structopt(default_value = "default", long)]
    template: String,
}

impl Init {
    pub fn init(self) -> Result<()> {
        if self.name.contains(".") {
            log::error!("❗Unsupported project name.");
            return Ok(());
        }

        log::info!("🔧 Start to init a new project '{}'.", self.name);

        let project_path = PathBuf::from(&self.name);

        if project_path.join("Cargo.toml").is_file() {
            log::warn!("Folder '{}' is initialized.", self.name);
            return Ok(());
        }

        let output = Command::new("cargo")
            .arg("init")
            .arg(&format!("./{}", self.name))
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

        let mut file = File::create(project_path.join("Dioxus.toml"))?;
        let dioxus_conf = String::from(include_str!("../../template/config.toml"))
            .replace("{project-name}", &self.name);
        file.write_all(dioxus_conf.as_bytes())?;

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

        println!("");
        log::info!("💡 Project initialized:");
        log::info!("🎯> cd ./{}", self.name);
        log::info!("🎯> dioxus serve");

        Ok(())
    }
}
