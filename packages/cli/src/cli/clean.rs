use crate::DioxusCrate;

use super::*;

/// Clean build artifacts.
#[derive(Clone, Debug, Parser)]
#[clap(name = "clean")]
pub struct Clean {}

impl Clean {
    pub fn clean(self, crate_config: DioxusCrate) -> Result<()> {
        let output = Command::new("cargo")
            .arg("clean")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            return custom_error!("Cargo clean failed.");
        }

        let out_dir = &crate_config.out_dir();
        if out_dir.is_dir() {
            remove_dir_all(out_dir)?;
        }

        let fullstack_out_dir = crate_config.fullstack_out_dir();

        if fullstack_out_dir.is_dir() {
            remove_dir_all(fullstack_out_dir)?;
        }

        Ok(())
    }
}
