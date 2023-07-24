use super::*;

/// Clean build artifacts.
#[derive(Clone, Debug, Parser)]
#[clap(name = "clean")]
pub struct Clean {}

impl Clean {
    pub fn clean(self, bin: Option<PathBuf>) -> Result<()> {
        let crate_config = crate::CrateConfig::new(bin)?;

        let output = Command::new("cargo")
            .arg("clean")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            return custom_error!("Cargo clean failed.");
        }

        let out_dir = crate_config
            .dioxus_config
            .application
            .out_dir
            .unwrap_or_else(|| PathBuf::from("dist"));
        if crate_config.crate_dir.join(&out_dir).is_dir() {
            remove_dir_all(crate_config.crate_dir.join(&out_dir))?;
        }

        Ok(())
    }
}
