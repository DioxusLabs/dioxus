use super::*;

/// Clean build artifacts.
///
/// Simlpy runs `cargo clean`
#[derive(Clone, Debug, Parser)]
#[clap(name = "clean")]
pub(crate) struct Clean {}

impl Clean {
    /// todo(jon): we should add a config option that just wipes target/dx and target/dioxus-client instead of doing a full clean
    pub(crate) fn clean(self) -> anyhow::Result<()> {
        let output = Command::new("cargo")
            .arg("clean")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Cargo clean failed."));
        }

        Ok(())
    }
}
