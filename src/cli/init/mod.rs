use std::{process::{Command, Stdio}, fs::File, path::PathBuf, io::Write};

use structopt::StructOpt;

use crate::{error::{Error, Result}, cargo};

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

        let output = Command::new("cargo")
            .arg("init")
            .arg(&self.path)
            .arg("--bin")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?
        ;

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

        

        Ok(())
    }
}