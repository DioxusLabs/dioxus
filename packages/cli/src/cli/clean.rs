use super::*;
use crate::Result;

/// Clean build artifacts.
///
/// Simply runs `cargo clean`
#[derive(Clone, Debug, Parser)]
pub(crate) struct Clean {}

impl Clean {
    /// todo(jon): we should add a config option that just wipes target/dx and target/dioxus-client instead of doing a full clean
    pub(crate) async fn clean(self) -> Result<StructuredOutput> {
        let output = tokio::process::Command::new("cargo")
            .arg("clean")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Cargo clean failed.").into());
        }

        Ok(StructuredOutput::Success)
    }
}
