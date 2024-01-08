use super::*;

/// Clean build artifacts.
#[derive(Clone, Debug, Parser)]
#[clap(name = "clean")]
pub struct Clean {}

impl Clean {
    pub fn clean(self, bin: Option<PathBuf>) -> Result<()> {
        let crate_config = dioxus_cli_config::CrateConfig::new(bin)?;

        let output = Command::new("cargo")
            .arg("clean")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            return custom_error!("Cargo clean failed.");
        }

        let out_dir = crate_config.dioxus_config.application.out_dir;
        if crate_config.crate_dir.join(&out_dir).is_dir() {
            remove_dir_all(crate_config.crate_dir.join(&out_dir))?;
        }

        let fullstack_out_dir = crate_config.crate_dir.join(".dioxus");

        if fullstack_out_dir.is_dir() {
            remove_dir_all(fullstack_out_dir)?;
        }

        Ok(())
    }
}
